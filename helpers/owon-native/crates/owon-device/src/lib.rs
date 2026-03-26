use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use owon_protocol::{
    ChannelId, FLASH_SIZE, FPGA_FRAME_HEADER_SIZE, FlashSummary, FpgaState, GET_DATA_ADC_SIZE,
    GET_DATA_EDGE_MARGIN_SAMPLES, GET_DATA_FRAME_SIZE, GET_DATA_NOT_READY_SIZE,
    GET_DATA_PERTINENT_SAMPLES, MachineKind, ProtocolError, RawDataFrame, STATUS_SUCCESS,
    U32_RESPONSE_SIZE, fpga_state, machine_kind, pack_get_data_command, pack_get_machine_command,
    pack_load_fpga_chunk, pack_load_fpga_command, pack_query_fpga_command, pack_read_flash_command,
    pack_set_channel_command, pack_set_channels_on_command, pack_set_deepmemory_command,
    pack_set_edge_level_command, pack_set_freqref_command, pack_set_multi_command,
    pack_set_peakmode_command, pack_set_phasefine_command, pack_set_post_trigger_command,
    pack_set_pre_trigger_command, pack_set_rollmode_command, pack_set_timebase_command,
    pack_set_trigger_default_command, pack_set_trigger_holdoff_command,
    pack_set_voltage_gain_command, pack_set_zero_offset_command, parse_flash_summary,
    parse_get_data_frame, parse_u32_response,
};
use owon_usb::{
    DEFAULT_USB_TIMEOUT, DeviceSnapshot, OpenSession, UsbOpenError, UsbTransferError, open_session,
};

const CAPTURE_SAMPLE_RATE_HZ: f64 = 2_500_000.0;
const CAPTURE_SAMPLE_PERIOD_NS: f64 = 1_000_000_000.0 / CAPTURE_SAMPLE_RATE_HZ;
const CAPTURE_TIMEBASE_PRESCALER: u32 = 40;
const CAPTURE_MAX_REQUEST_CYCLES: usize = 10;
const CAPTURE_MAX_BUSY_POLLS_PER_CYCLE: usize = 10;
const CAPTURE_RETRY_DELAY: Duration = Duration::from_millis(60);
const CAPTURE_INITIAL_SETTLE_DELAY: Duration = Duration::from_millis(250);
const CAPTURE_USB_TIMEOUT: Duration = Duration::from_secs(1);
const CAPTURE_PRE_TRIGGER_SAMPLES: u16 = ((GET_DATA_ADC_SIZE / 2) - 11) as u16;
const CAPTURE_POST_TRIGGER_SAMPLES: u32 = ((GET_DATA_ADC_SIZE / 2) + 11) as u32;
const CAPTURE_TRIGGER_INDEX: usize = 0;
const FREQUENCY_CLOCK_HZ: f64 = 100_000_000.0;
const FLASH_CHANNEL_COUNT: usize = 2;
const FLASH_CALIBRATION_RANGE_COUNT: usize = 10;
const FLASH_CALIBRATION_OFFSET: usize = 6;
const FLASH_GAIN_INDEX: usize = 0;
const FLASH_COMP_INDEX: usize = 2;
const DEFAULT_VOLT_RANGE_INDEX: usize = 5;
const DEFAULT_CHANNEL_COMMAND: u8 = 0xa0;
const DEFAULT_MULTI_MODE: u16 = 0;
const DEFAULT_TRIGGER_HOLDOFF: u16 = 0x8002;
const DEFAULT_EDGE_LEVEL_DISABLED: u16 = 0x807f;
const DEFAULT_FREQREF_LEVEL: u8 = 20;
const DEFAULT_AUTO_TRIGGER_EDGE_LEVEL: u16 = 0xf600;
const DEFAULT_AUTO_TRIGGER_FREQREF: u8 = 0xfb;

/// Captured samples for one channel.
///
/// In this first cut the samples remain raw ADC bytes. "Normalized" means that
/// USB/protocol framing is removed and timing metadata is attached at the capture
/// level.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelCapture {
    pub channel: ChannelId,
    pub samples: Vec<u8>,
    pub trigger_index: Option<usize>,
    pub frequency_hz: Option<f64>,
}

/// Capture shape that `owon-decode` will later consume.
///
/// This type is intentionally minimal for now: sample timing plus channel-local
/// sample arrays. Voltage calibration and actual acquisition logic land in later
/// steps.
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedCapture {
    pub sample_rate_hz: f64,
    pub sample_period_ns: f64,
    pub channels: Vec<ChannelCapture>,
}

impl NormalizedCapture {
    pub fn channel(&self, channel: ChannelId) -> Option<&ChannelCapture> {
        self.channels
            .iter()
            .find(|capture| capture.channel == channel)
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub machine: MachineKind,
    pub flash: FlashSummary,
    pub fpga_state_before: FpgaState,
    pub fpga_state_after: FpgaState,
    pub selected_firmware: Option<PathBuf>,
    pub firmware_loaded_this_session: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureChannelSelection {
    Ch1,
    Ch2,
    Both,
}

impl CaptureChannelSelection {
    fn ch1_enabled(self) -> bool {
        matches!(self, Self::Ch1 | Self::Both)
    }

    fn ch2_enabled(self) -> bool {
        matches!(self, Self::Ch2 | Self::Both)
    }

    fn includes(self, channel: ChannelId) -> bool {
        match channel {
            ChannelId::Ch1 => self.ch1_enabled(),
            ChannelId::Ch2 => self.ch2_enabled(),
        }
    }

    fn requested_channels(self) -> Vec<ChannelId> {
        match self {
            Self::Ch1 => vec![ChannelId::Ch1],
            Self::Ch2 => vec![ChannelId::Ch2],
            Self::Both => vec![ChannelId::Ch1, ChannelId::Ch2],
        }
    }

    fn trigger_channel(self) -> ChannelId {
        match self {
            Self::Ch1 | Self::Both => ChannelId::Ch1,
            Self::Ch2 => ChannelId::Ch2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelCalibrationDebugInfo {
    pub channel: ChannelId,
    pub zero_offset: u16,
    pub voltage_gain: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelCaptureDebugInfo {
    pub channel: ChannelId,
    pub cursor_from_right: u16,
    pub time_sum: u32,
    pub period_num: u32,
    pub sample_offset: usize,
    pub sample_count: usize,
    pub sample_min_u8: u8,
    pub sample_max_u8: u8,
    pub sample_min_i8: i8,
    pub sample_max_i8: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureDebugInfo {
    pub requested_channels: Vec<ChannelId>,
    pub volt_range_index: usize,
    pub timebase_prescaler: u32,
    pub rollmode: bool,
    pub peakmode: bool,
    pub pre_trigger_samples: u16,
    pub post_trigger_samples: u32,
    pub trigger_holdoff: u16,
    pub edge_level: u16,
    pub freqref: u8,
    pub phasefine: u16,
    pub calibrations: Vec<ChannelCalibrationDebugInfo>,
    pub channels: Vec<ChannelCaptureDebugInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureOnceResult {
    pub capture: NormalizedCapture,
    pub debug: CaptureDebugInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UartBenchOptions {
    pub channel: ChannelId,
    pub volt_range_index: usize,
    pub timebase_prescaler: u32,
    pub rollmode: bool,
    pub peakmode: bool,
    pub pre_trigger_samples: u16,
    pub post_trigger_samples: u32,
    pub trigger_holdoff: u16,
    pub edge_level: u16,
    pub freqref: u8,
}

impl UartBenchOptions {
    pub fn for_channel(channel: ChannelId) -> Self {
        Self {
            channel,
            volt_range_index: 8,
            timebase_prescaler: CAPTURE_TIMEBASE_PRESCALER,
            rollmode: false,
            peakmode: false,
            pre_trigger_samples: CAPTURE_PRE_TRIGGER_SAMPLES,
            post_trigger_samples: CAPTURE_POST_TRIGGER_SAMPLES,
            trigger_holdoff: DEFAULT_TRIGGER_HOLDOFF,
            edge_level: DEFAULT_AUTO_TRIGGER_EDGE_LEVEL,
            freqref: DEFAULT_AUTO_TRIGGER_FREQREF,
        }
    }
}

#[derive(Debug)]
pub struct DeviceSession {
    transport: OpenSession,
    info: DeviceInfo,
    flash_bytes: Vec<u8>,
    live_capture_plan: Option<LiveCapturePlan>,
}

impl DeviceSession {
    pub fn open() -> Result<Self, DeviceError> {
        let transport = open_session()?;
        let flash_bytes = read_flash_bytes(&transport)?;
        let flash = parse_flash_summary(&flash_bytes)?;

        let fpga_state_before =
            fpga_state(execute_u32_command(&transport, &pack_query_fpga_command())?);
        let load_report = load_fpga_if_needed(&transport, &flash, fpga_state_before)?;
        let machine = machine_kind(execute_u32_command(
            &transport,
            &pack_get_machine_command(),
        )?);

        Ok(Self {
            transport,
            flash_bytes,
            live_capture_plan: None,
            info: DeviceInfo {
                machine,
                flash,
                fpga_state_before: load_report.state_before,
                fpga_state_after: load_report.state_after,
                selected_firmware: load_report.firmware_path,
                firmware_loaded_this_session: load_report.loaded_this_session,
            },
        })
    }

    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    pub fn transport_snapshot(&self) -> &DeviceSnapshot {
        self.transport.snapshot()
    }

    pub fn capture_once(&mut self) -> Result<NormalizedCapture, DeviceError> {
        Ok(self.capture_once_with_debug()?.capture)
    }

    pub fn capture_once_with_debug(&mut self) -> Result<CaptureOnceResult, DeviceError> {
        self.stop_live_capture();
        self.capture_with_request(CaptureRequest::default_dual())
    }

    pub fn start_live_capture(&mut self) -> Result<(), DeviceError> {
        let plan = self.prepare_capture_plan(CaptureRequest::default_dual())?;
        push_capture_defaults(&self.transport, &plan.capture_profile, &plan.request)?;
        self.live_capture_plan = Some(plan);
        Ok(())
    }

    pub fn retry_live_capture(&mut self) -> Result<(), DeviceError> {
        self.stop_live_capture();
        self.start_live_capture()
    }

    pub fn capture_live_frame(&mut self) -> Result<CaptureOnceResult, DeviceError> {
        let Some(plan) = self.live_capture_plan.as_ref() else {
            return Err(DeviceError::LiveCaptureNotStarted);
        };

        let frames = read_capture_frames(&self.transport, &plan.request)?;
        normalize_capture_result(frames, &plan.request, &plan.capture_profile)
    }

    pub fn stop_live_capture(&mut self) {
        self.live_capture_plan = None;
    }

    pub fn capture_uart_bench(
        &mut self,
        channel: ChannelId,
    ) -> Result<CaptureOnceResult, DeviceError> {
        self.capture_uart_bench_with_options(UartBenchOptions::for_channel(channel))
    }

    pub fn capture_uart_bench_with_options(
        &mut self,
        options: UartBenchOptions,
    ) -> Result<CaptureOnceResult, DeviceError> {
        let request = CaptureRequest::uart_bench(options);
        let capture_profile = parse_capture_profile(
            &self.flash_bytes,
            &self.info.flash,
            request.volt_range_index,
        )?;
        push_capture_defaults(&self.transport, &capture_profile, &request)?;

        // Rapid-fire captures for up to SCAN_DURATION, keeping the one with the
        // best signal (highest i8 swing).  The scope buffer is only 2 ms wide, so
        // we repeat as fast as possible to maximise the chance of landing inside
        // the UART burst.
        const SCAN_DURATION: Duration = Duration::from_secs(10);

        let deadline = std::time::Instant::now() + SCAN_DURATION;
        let mut best: Option<CaptureOnceResult> = None;
        let mut best_swing: i16 = 0;
        let mut attempt: usize = 0;

        while std::time::Instant::now() < deadline {
            attempt += 1;
            let frames = read_capture_frames(&self.transport, &request)?;
            let result = normalize_capture_result(frames, &request, &capture_profile)?;

            let ch = result
                .capture
                .channel(options.channel)
                .ok_or(DeviceError::MissingChannelFrame(options.channel))?;
            let stats = byte_sample_stats(&ch.samples);
            let swing = (stats.max_i8 as i16) - (stats.min_i8 as i16);

            if capture_trace_enabled() {
                eprintln!(
                    "capture-trace: uart-scan attempt={} swing={} best={} min_i8={} max_i8={}",
                    attempt, swing, best_swing, stats.min_i8, stats.max_i8,
                );
            }

            if swing > best_swing {
                best_swing = swing;
                best = Some(result);
            }
        }

        if capture_trace_enabled() {
            eprintln!(
                "capture-trace: uart-scan done attempts={} best_swing={}",
                attempt, best_swing,
            );
        }

        best.ok_or(DeviceError::DataNotReadyAfterRetries { attempts: attempt })
    }

    pub fn close(self) -> Result<(), DeviceError> {
        let Self { transport, .. } = self;
        transport.close().map_err(DeviceError::from)
    }
}

#[derive(Debug)]
pub enum DeviceError {
    UsbOpen(UsbOpenError),
    UsbTransfer(UsbTransferError),
    Protocol(ProtocolError),
    Io(io::Error),
    FirmwareNotFound {
        directory: PathBuf,
        vfpga: u32,
    },
    InvalidFpgaFrameSize(u32),
    FpgaChunkIndexOverflow(usize),
    FpgaChunkAckStatus {
        expected: u8,
        actual: u8,
    },
    FpgaChunkAckIndex {
        expected: u32,
        actual: u32,
    },
    FpgaStillMissingAfterLoad(FpgaState),
    InvalidAdcSampleCount {
        expected: usize,
        actual: usize,
    },
    UnexpectedGetDataResponseLength(usize),
    DataNotReadyAfterRetries {
        attempts: usize,
    },
    LiveCaptureNotStarted,
    DuplicateChannelFrame(ChannelId),
    MissingChannelFrame(ChannelId),
    UnexpectedChannelFrame {
        requested: ChannelId,
        actual: ChannelId,
    },
    UnsupportedRollCursor {
        channel: ChannelId,
        cursor_from_right: u16,
    },
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UsbOpen(error) => write!(f, "{error}"),
            Self::UsbTransfer(error) => write!(f, "{error}"),
            Self::Protocol(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::FirmwareNotFound { directory, vfpga } => write!(
                f,
                "No FPGA image found in {} for inferred version {}",
                directory.display(),
                vfpga
            ),
            Self::InvalidFpgaFrameSize(frame_size) => {
                write!(
                    f,
                    "Invalid FPGA frame size returned by device: {frame_size}"
                )
            }
            Self::FpgaChunkIndexOverflow(index) => {
                write!(f, "FPGA upload chunk index does not fit in u32: {index}")
            }
            Self::FpgaChunkAckStatus { expected, actual } => write!(
                f,
                "Unexpected FPGA chunk ack status: expected 0x{expected:02x}, got 0x{actual:02x}"
            ),
            Self::FpgaChunkAckIndex { expected, actual } => write!(
                f,
                "Unexpected FPGA chunk ack index: expected {expected}, got {actual}"
            ),
            Self::FpgaStillMissingAfterLoad(state) => {
                write!(f, "FPGA is not ready after upload: {state}")
            }
            Self::InvalidAdcSampleCount { expected, actual } => write!(
                f,
                "Unexpected ADC sample count: expected {expected} bytes, got {actual}"
            ),
            Self::UnexpectedGetDataResponseLength(actual) => write!(
                f,
                "Unexpected GET_DATA response length: expected 5 or {GET_DATA_FRAME_SIZE} bytes, got {actual}"
            ),
            Self::DataNotReadyAfterRetries { attempts } => {
                write!(f, "GET_DATA stayed busy after {attempts} attempts")
            }
            Self::LiveCaptureNotStarted => {
                write!(
                    f,
                    "Live capture has not been started for this device session"
                )
            }
            Self::DuplicateChannelFrame(channel) => {
                write!(f, "Received duplicate GET_DATA frame for {channel}")
            }
            Self::MissingChannelFrame(channel) => {
                write!(f, "Missing GET_DATA frame for {channel}")
            }
            Self::UnexpectedChannelFrame { requested, actual } => {
                write!(f, "Expected GET_DATA frame for {requested}, got {actual}")
            }
            Self::UnsupportedRollCursor {
                channel,
                cursor_from_right,
            } => write!(
                f,
                "GET_DATA returned a non-centered cursor for {channel}: {cursor_from_right}"
            ),
        }
    }
}

impl Error for DeviceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UsbOpen(error) => Some(error),
            Self::UsbTransfer(error) => Some(error),
            Self::Protocol(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::FirmwareNotFound { .. }
            | Self::InvalidFpgaFrameSize(_)
            | Self::FpgaChunkIndexOverflow(_)
            | Self::FpgaChunkAckStatus { .. }
            | Self::FpgaChunkAckIndex { .. }
            | Self::FpgaStillMissingAfterLoad(_)
            | Self::InvalidAdcSampleCount { .. }
            | Self::UnexpectedGetDataResponseLength(_)
            | Self::DataNotReadyAfterRetries { .. }
            | Self::LiveCaptureNotStarted
            | Self::DuplicateChannelFrame(_)
            | Self::MissingChannelFrame(_)
            | Self::UnexpectedChannelFrame { .. }
            | Self::UnsupportedRollCursor { .. } => None,
        }
    }
}

impl From<UsbOpenError> for DeviceError {
    fn from(value: UsbOpenError) -> Self {
        Self::UsbOpen(value)
    }
}

impl From<UsbTransferError> for DeviceError {
    fn from(value: UsbTransferError) -> Self {
        Self::UsbTransfer(value)
    }
}

impl From<ProtocolError> for DeviceError {
    fn from(value: ProtocolError) -> Self {
        Self::Protocol(value)
    }
}

impl From<io::Error> for DeviceError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug)]
struct FpgaLoadReport {
    state_before: FpgaState,
    state_after: FpgaState,
    firmware_path: Option<PathBuf>,
    loaded_this_session: bool,
}

#[derive(Debug, Clone, Copy)]
struct CaptureChannelProfile {
    zero_offset: u16,
    voltage_gain: u16,
}

#[derive(Debug, Clone)]
struct CaptureProfile {
    phasefine: u16,
    channels: [CaptureChannelProfile; FLASH_CHANNEL_COUNT],
}

impl CaptureProfile {
    fn channel(&self, channel: ChannelId) -> CaptureChannelProfile {
        match channel {
            ChannelId::Ch1 => self.channels[0],
            ChannelId::Ch2 => self.channels[1],
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CaptureRequest {
    channels: CaptureChannelSelection,
    volt_range_index: usize,
    timebase_prescaler: u32,
    rollmode: bool,
    peakmode: bool,
    pre_trigger_samples: u16,
    post_trigger_samples: u32,
    trigger_holdoff: u16,
    edge_level: u16,
    freqref: u8,
}

impl CaptureRequest {
    fn default_dual() -> Self {
        Self {
            channels: CaptureChannelSelection::Both,
            volt_range_index: DEFAULT_VOLT_RANGE_INDEX,
            timebase_prescaler: CAPTURE_TIMEBASE_PRESCALER,
            rollmode: false,
            peakmode: false,
            pre_trigger_samples: CAPTURE_PRE_TRIGGER_SAMPLES,
            post_trigger_samples: CAPTURE_POST_TRIGGER_SAMPLES,
            trigger_holdoff: DEFAULT_TRIGGER_HOLDOFF,
            edge_level: DEFAULT_AUTO_TRIGGER_EDGE_LEVEL,
            freqref: DEFAULT_AUTO_TRIGGER_FREQREF,
        }
    }

    fn uart_bench(options: UartBenchOptions) -> Self {
        Self {
            channels: match options.channel {
                ChannelId::Ch1 => CaptureChannelSelection::Ch1,
                ChannelId::Ch2 => CaptureChannelSelection::Ch2,
            },
            volt_range_index: options.volt_range_index,
            timebase_prescaler: options.timebase_prescaler,
            rollmode: options.rollmode,
            peakmode: options.peakmode,
            pre_trigger_samples: options.pre_trigger_samples,
            post_trigger_samples: options.post_trigger_samples,
            trigger_holdoff: options.trigger_holdoff,
            edge_level: options.edge_level,
            freqref: options.freqref,
            ..Self::default_dual()
        }
    }
}

#[derive(Debug, Clone)]
struct LiveCapturePlan {
    request: CaptureRequest,
    capture_profile: CaptureProfile,
}

fn capture_trace_enabled() -> bool {
    matches!(env::var("OWON_CAPTURE_TRACE").as_deref(), Ok("1"))
}

impl DeviceSession {
    fn prepare_capture_plan(
        &self,
        request: CaptureRequest,
    ) -> Result<LiveCapturePlan, DeviceError> {
        let capture_profile = parse_capture_profile(
            &self.flash_bytes,
            &self.info.flash,
            request.volt_range_index,
        )?;

        Ok(LiveCapturePlan {
            request,
            capture_profile,
        })
    }

    fn capture_with_request(
        &mut self,
        request: CaptureRequest,
    ) -> Result<CaptureOnceResult, DeviceError> {
        let plan = self.prepare_capture_plan(request)?;
        push_capture_defaults(&self.transport, &plan.capture_profile, &plan.request)?;
        let frames = read_capture_frames(&self.transport, &plan.request)?;
        normalize_capture_result(frames, &plan.request, &plan.capture_profile)
    }
}

fn execute_u32_command(session: &OpenSession, request: &[u8]) -> Result<u32, DeviceError> {
    let mut response = [0u8; U32_RESPONSE_SIZE];
    session.write_bulk_exact(request, DEFAULT_USB_TIMEOUT)?;
    session.read_bulk_exact(&mut response, DEFAULT_USB_TIMEOUT)?;
    Ok(parse_u32_response(&response)?.value)
}

fn read_flash_bytes(session: &OpenSession) -> Result<Vec<u8>, DeviceError> {
    let request = pack_read_flash_command();
    let mut response = vec![0u8; FLASH_SIZE];
    session.write_bulk_exact(&request, DEFAULT_USB_TIMEOUT)?;
    session.read_bulk_exact(&mut response, DEFAULT_USB_TIMEOUT)?;
    Ok(response)
}

fn parse_capture_profile(
    flash_bytes: &[u8],
    flash: &FlashSummary,
    volt_range_index: usize,
) -> Result<CaptureProfile, DeviceError> {
    if flash_bytes.len() != FLASH_SIZE {
        return Err(DeviceError::Protocol(ProtocolError::UnexpectedLength {
            expected: FLASH_SIZE,
            actual: flash_bytes.len(),
        }));
    }

    Ok(CaptureProfile {
        phasefine: flash.phasefine,
        channels: [
            CaptureChannelProfile {
                zero_offset: calibration_value(
                    flash_bytes,
                    FLASH_COMP_INDEX,
                    ChannelId::Ch1,
                    volt_range_index,
                )?,
                voltage_gain: calibration_value(
                    flash_bytes,
                    FLASH_GAIN_INDEX,
                    ChannelId::Ch1,
                    volt_range_index,
                )?,
            },
            CaptureChannelProfile {
                zero_offset: calibration_value(
                    flash_bytes,
                    FLASH_COMP_INDEX,
                    ChannelId::Ch2,
                    volt_range_index,
                )?,
                voltage_gain: calibration_value(
                    flash_bytes,
                    FLASH_GAIN_INDEX,
                    ChannelId::Ch2,
                    volt_range_index,
                )?,
            },
        ],
    })
}

fn calibration_value(
    flash_bytes: &[u8],
    calibration_index: usize,
    channel: ChannelId,
    range_index: usize,
) -> Result<u16, DeviceError> {
    let channel_index = match channel {
        ChannelId::Ch1 => 0,
        ChannelId::Ch2 => 1,
    };
    let word_index = ((calibration_index * FLASH_CHANNEL_COUNT + channel_index)
        * FLASH_CALIBRATION_RANGE_COUNT)
        + range_index;
    let offset = FLASH_CALIBRATION_OFFSET + word_index * 2;
    if offset + 2 > flash_bytes.len() {
        return Err(DeviceError::Protocol(ProtocolError::FieldOutOfRange(
            "flash_calibration",
        )));
    }

    Ok(u16::from_le_bytes([
        flash_bytes[offset],
        flash_bytes[offset + 1],
    ]))
}

fn push_capture_defaults(
    session: &OpenSession,
    capture_profile: &CaptureProfile,
    request: &CaptureRequest,
) -> Result<(), DeviceError> {
    let mut requests = vec![
        pack_set_channels_on_command(false, false).to_vec(),
        pack_set_phasefine_command(capture_profile.phasefine).to_vec(),
        pack_set_peakmode_command(false).to_vec(),
        pack_set_deepmemory_command(GET_DATA_ADC_SIZE as u16).to_vec(),
        pack_set_pre_trigger_command(GET_DATA_ADC_SIZE as u16).to_vec(),
        pack_set_post_trigger_command(0).to_vec(),
        pack_set_multi_command(DEFAULT_MULTI_MODE).to_vec(),
        pack_set_trigger_default_command().to_vec(),
    ];

    for channel in [ChannelId::Ch1, ChannelId::Ch2] {
        let profile = capture_profile.channel(channel);
        requests.push(pack_set_trigger_holdoff_command(channel, DEFAULT_TRIGGER_HOLDOFF).to_vec());
        requests.push(pack_set_edge_level_command(channel, DEFAULT_EDGE_LEVEL_DISABLED).to_vec());
        requests.push(pack_set_freqref_command(channel, DEFAULT_FREQREF_LEVEL).to_vec());
        requests
            .push(pack_set_zero_offset_command(channel, profile.zero_offset.min(4095)).to_vec());
        requests
            .push(pack_set_voltage_gain_command(channel, profile.voltage_gain.min(4095)).to_vec());
        requests.push(pack_set_channel_command(channel, DEFAULT_CHANNEL_COMMAND).to_vec());
    }

    requests.extend([
        pack_set_channels_on_command(
            request.channels.ch1_enabled(),
            request.channels.ch2_enabled(),
        )
        .to_vec(),
        pack_set_timebase_command(request.timebase_prescaler).to_vec(),
        pack_set_rollmode_command(request.rollmode).to_vec(),
        pack_set_peakmode_command(request.peakmode).to_vec(),
        pack_set_deepmemory_command(GET_DATA_ADC_SIZE as u16).to_vec(),
        pack_set_pre_trigger_command(request.pre_trigger_samples).to_vec(),
        pack_set_post_trigger_command(request.post_trigger_samples).to_vec(),
        pack_set_edge_level_command(request.channels.trigger_channel(), request.edge_level)
            .to_vec(),
        pack_set_freqref_command(request.channels.trigger_channel(), request.freqref).to_vec(),
        pack_set_trigger_holdoff_command(
            request.channels.trigger_channel(),
            request.trigger_holdoff,
        )
        .to_vec(),
        pack_set_multi_command(DEFAULT_MULTI_MODE).to_vec(),
        pack_set_trigger_default_command().to_vec(),
    ]);

    for request in requests {
        session.write_bulk_exact(&request, CAPTURE_USB_TIMEOUT)?;
    }

    if capture_trace_enabled() {
        eprintln!(
            "capture-trace: initialized capture defaults channels={:?} volt_range={} timebase={} rollmode={} peakmode={} pre={} post={} holdoff=0x{:04x} edge=0x{:04x} freqref=0x{:02x} phasefine={} ch1(zero={},gain={}) ch2(zero={},gain={})",
            request.channels,
            request.volt_range_index,
            request.timebase_prescaler,
            request.rollmode,
            request.peakmode,
            request.pre_trigger_samples,
            request.post_trigger_samples,
            request.trigger_holdoff,
            request.edge_level,
            request.freqref,
            capture_profile.phasefine,
            capture_profile.channel(ChannelId::Ch1).zero_offset,
            capture_profile.channel(ChannelId::Ch1).voltage_gain,
            capture_profile.channel(ChannelId::Ch2).zero_offset,
            capture_profile.channel(ChannelId::Ch2).voltage_gain,
        );
    }

    thread::sleep(CAPTURE_INITIAL_SETTLE_DELAY);
    Ok(())
}

fn read_capture_frames(
    session: &OpenSession,
    request: &CaptureRequest,
) -> Result<Vec<RawDataFrame>, DeviceError> {
    let request_bytes = pack_get_data_command(
        request.channels.ch1_enabled(),
        request.channels.ch2_enabled(),
    );
    let mut buffer = vec![0u8; GET_DATA_FRAME_SIZE];
    let mut ch1 = None;
    let mut ch2 = None;
    let requested_channels = request.channels.requested_channels();

    for cycle in 0..CAPTURE_MAX_REQUEST_CYCLES {
        if capture_trace_enabled() {
            eprintln!(
                "capture-trace: cycle={} write GET_DATA ch1={} ch2={}",
                cycle + 1,
                if request.channels.ch1_enabled() {
                    "on"
                } else {
                    "off"
                },
                if request.channels.ch2_enabled() {
                    "on"
                } else {
                    "off"
                },
            );
        }
        session.write_bulk_exact(&request_bytes, CAPTURE_USB_TIMEOUT)?;
        let mut busy_polls = 0usize;

        loop {
            let read = match session.read_bulk(&mut buffer, CAPTURE_USB_TIMEOUT) {
                Ok(read) => read,
                Err(error) if error.is_read_timeout() => {
                    if capture_trace_enabled() {
                        eprintln!(
                            "capture-trace: cycle={} read-timeout have_ch1={} have_ch2={}",
                            cycle + 1,
                            ch1.is_some(),
                            ch2.is_some(),
                        );
                    }
                    break;
                }
                Err(error) => return Err(DeviceError::from(error)),
            };
            if capture_trace_enabled() {
                eprintln!(
                    "capture-trace: cycle={} read={} busy_polls={} have_ch1={} have_ch2={}",
                    cycle + 1,
                    read,
                    busy_polls,
                    ch1.is_some(),
                    ch2.is_some(),
                );
            }
            match read {
                GET_DATA_FRAME_SIZE => {
                    let frame = parse_get_data_frame(&buffer[..read])?;
                    if capture_trace_enabled() {
                        eprintln!(
                            "capture-trace: frame channel={} cursor={} time_sum={} period_num={}",
                            frame.header.channel,
                            frame.header.cursor_from_right,
                            frame.header.time_sum,
                            frame.header.period_num,
                        );
                    }
                    let channel = frame.header.channel;
                    if !request.channels.includes(channel) {
                        if capture_trace_enabled() {
                            eprintln!(
                                "capture-trace: ignoring unexpected frame for unrequested channel={}",
                                channel
                            );
                        }
                        continue;
                    }
                    let slot = match channel {
                        ChannelId::Ch1 => &mut ch1,
                        ChannelId::Ch2 => &mut ch2,
                    };
                    if slot.is_none() {
                        *slot = Some(frame);
                    }
                    busy_polls = 0;

                    if requested_channels.iter().all(|channel| match channel {
                        ChannelId::Ch1 => ch1.is_some(),
                        ChannelId::Ch2 => ch2.is_some(),
                    }) {
                        let mut frames = Vec::with_capacity(requested_channels.len());
                        for channel in &requested_channels {
                            frames.push(match channel {
                                ChannelId::Ch1 => ch1
                                    .take()
                                    .ok_or(DeviceError::MissingChannelFrame(ChannelId::Ch1))?,
                                ChannelId::Ch2 => ch2
                                    .take()
                                    .ok_or(DeviceError::MissingChannelFrame(ChannelId::Ch2))?,
                            });
                        }
                        return Ok(frames);
                    }
                }
                GET_DATA_NOT_READY_SIZE => {
                    busy_polls += 1;
                    if capture_trace_enabled() {
                        eprintln!("capture-trace: busy");
                    }
                    if cycle + 1 == CAPTURE_MAX_REQUEST_CYCLES
                        && busy_polls >= CAPTURE_MAX_BUSY_POLLS_PER_CYCLE
                    {
                        return Err(DeviceError::DataNotReadyAfterRetries {
                            attempts: CAPTURE_MAX_REQUEST_CYCLES * CAPTURE_MAX_BUSY_POLLS_PER_CYCLE,
                        });
                    }

                    if busy_polls >= CAPTURE_MAX_BUSY_POLLS_PER_CYCLE {
                        break;
                    }

                    thread::sleep(CAPTURE_RETRY_DELAY);
                    continue;
                }
                actual => return Err(DeviceError::UnexpectedGetDataResponseLength(actual)),
            }
        }

        thread::sleep(CAPTURE_RETRY_DELAY);
    }

    Err(DeviceError::DataNotReadyAfterRetries {
        attempts: CAPTURE_MAX_REQUEST_CYCLES * CAPTURE_MAX_BUSY_POLLS_PER_CYCLE,
    })
}

fn normalize_capture_result(
    frames: Vec<RawDataFrame>,
    request: &CaptureRequest,
    capture_profile: &CaptureProfile,
) -> Result<CaptureOnceResult, DeviceError> {
    let mut channels = Vec::with_capacity(frames.len());
    let mut debug_channels = Vec::with_capacity(frames.len());

    for frame in frames {
        let (capture, debug) = normalize_channel_frame(frame)?;
        channels.push(capture);
        debug_channels.push(debug);
    }

    channels.sort_by_key(|capture| capture.channel);
    debug_channels.sort_by_key(|channel| channel.channel);

    let requested_channels = request.channels.requested_channels();
    let calibrations = requested_channels
        .iter()
        .map(|&channel| {
            let calibration = capture_profile.channel(channel);
            ChannelCalibrationDebugInfo {
                channel,
                zero_offset: calibration.zero_offset,
                voltage_gain: calibration.voltage_gain,
            }
        })
        .collect();

    Ok(CaptureOnceResult {
        capture: NormalizedCapture {
            sample_rate_hz: CAPTURE_SAMPLE_RATE_HZ,
            sample_period_ns: CAPTURE_SAMPLE_PERIOD_NS,
            channels,
        },
        debug: CaptureDebugInfo {
            requested_channels,
            volt_range_index: request.volt_range_index,
            timebase_prescaler: request.timebase_prescaler,
            rollmode: request.rollmode,
            peakmode: request.peakmode,
            pre_trigger_samples: request.pre_trigger_samples,
            post_trigger_samples: request.post_trigger_samples,
            trigger_holdoff: request.trigger_holdoff,
            edge_level: request.edge_level,
            freqref: request.freqref,
            phasefine: capture_profile.phasefine,
            calibrations,
            channels: debug_channels,
        },
    })
}

fn normalize_channel_frame(
    frame: RawDataFrame,
) -> Result<(ChannelCapture, ChannelCaptureDebugInfo), DeviceError> {
    let frequency_hz = if frame.header.time_sum == 0 {
        None
    } else {
        Some(frame.header.period_num as f64 / frame.header.time_sum as f64 * FREQUENCY_CLOCK_HZ)
    };
    let (sample_offset, trigger_index) = match usize::from(frame.header.cursor_from_right) {
        cursor if cursor >= GET_DATA_PERTINENT_SAMPLES => {
            (GET_DATA_EDGE_MARGIN_SAMPLES, Some(CAPTURE_TRIGGER_INDEX))
        }
        0 => (GET_DATA_ADC_SIZE - GET_DATA_PERTINENT_SAMPLES, None),
        _ => {
            return Err(DeviceError::UnsupportedRollCursor {
                channel: frame.header.channel,
                cursor_from_right: frame.header.cursor_from_right,
            });
        }
    };
    let samples = extract_samples(&frame, sample_offset)?;
    let stats = byte_sample_stats(&samples);

    Ok((
        ChannelCapture {
            channel: frame.header.channel,
            samples,
            trigger_index,
            frequency_hz,
        },
        ChannelCaptureDebugInfo {
            channel: frame.header.channel,
            cursor_from_right: frame.header.cursor_from_right,
            time_sum: frame.header.time_sum,
            period_num: frame.header.period_num,
            sample_offset,
            sample_count: GET_DATA_PERTINENT_SAMPLES,
            sample_min_u8: stats.min_u8,
            sample_max_u8: stats.max_u8,
            sample_min_i8: stats.min_i8,
            sample_max_i8: stats.max_i8,
        },
    ))
}

fn extract_samples(frame: &RawDataFrame, start: usize) -> Result<Vec<u8>, DeviceError> {
    if frame.adc_samples.len() != GET_DATA_ADC_SIZE {
        return Err(DeviceError::InvalidAdcSampleCount {
            expected: GET_DATA_ADC_SIZE,
            actual: frame.adc_samples.len(),
        });
    }

    let end = start + GET_DATA_PERTINENT_SAMPLES;
    Ok(frame.adc_samples[start..end].to_vec())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteSampleStats {
    min_u8: u8,
    max_u8: u8,
    min_i8: i8,
    max_i8: i8,
}

fn byte_sample_stats(samples: &[u8]) -> ByteSampleStats {
    let min_u8 = samples.iter().copied().min().unwrap_or(0);
    let max_u8 = samples.iter().copied().max().unwrap_or(0);
    let min_i8 = samples
        .iter()
        .copied()
        .map(|sample| sample as i8)
        .min()
        .unwrap_or(0);
    let max_i8 = samples
        .iter()
        .copied()
        .map(|sample| sample as i8)
        .max()
        .unwrap_or(0);
    ByteSampleStats {
        min_u8,
        max_u8,
        min_i8,
        max_i8,
    }
}

fn load_fpga_if_needed(
    session: &OpenSession,
    flash: &FlashSummary,
    state_before: FpgaState,
) -> Result<FpgaLoadReport, DeviceError> {
    if matches!(state_before, FpgaState::Loaded) {
        return Ok(FpgaLoadReport {
            state_before,
            state_after: state_before,
            firmware_path: None,
            loaded_this_session: false,
        });
    }

    let firmware_path = select_firmware_image(flash.inferred_vfpga)?;
    let firmware = fs::read(&firmware_path)?;
    let frame_size = execute_u32_command(session, &pack_load_fpga_command(firmware.len())?)?;
    let frame_size =
        usize::try_from(frame_size).map_err(|_| DeviceError::InvalidFpgaFrameSize(u32::MAX))?;
    if frame_size <= FPGA_FRAME_HEADER_SIZE {
        return Err(DeviceError::InvalidFpgaFrameSize(frame_size as u32));
    }

    let payload_size = frame_size - FPGA_FRAME_HEADER_SIZE;
    let mut ack_buffer = [0u8; U32_RESPONSE_SIZE];
    for (index, chunk) in firmware.chunks(payload_size).enumerate() {
        let part_index =
            u32::try_from(index).map_err(|_| DeviceError::FpgaChunkIndexOverflow(index))?;
        let frame = pack_load_fpga_chunk(part_index, chunk);
        session.write_bulk_exact(&frame, DEFAULT_USB_TIMEOUT)?;
        session.read_bulk_exact(&mut ack_buffer, DEFAULT_USB_TIMEOUT)?;

        let ack = parse_u32_response(&ack_buffer)?;
        if ack.status != STATUS_SUCCESS {
            return Err(DeviceError::FpgaChunkAckStatus {
                expected: STATUS_SUCCESS,
                actual: ack.status,
            });
        }
        if ack.value != part_index {
            return Err(DeviceError::FpgaChunkAckIndex {
                expected: part_index,
                actual: ack.value,
            });
        }
    }

    let state_after = fpga_state(execute_u32_command(session, &pack_query_fpga_command())?);
    if !matches!(state_after, FpgaState::Loaded) {
        return Err(DeviceError::FpgaStillMissingAfterLoad(state_after));
    }

    Ok(FpgaLoadReport {
        state_before,
        state_after,
        firmware_path: Some(firmware_path),
        loaded_this_session: true,
    })
}

fn select_firmware_image(vfpga: u32) -> Result<PathBuf, DeviceError> {
    let directory = repo_root()?.join("fwr");
    let prefix = format!("VDS1022_FPGAV{vfpga}_");
    let mut candidates = Vec::new();

    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with(&prefix) && name.ends_with(".bin") {
            candidates.push(path);
        }
    }

    candidates.sort();
    candidates
        .pop()
        .ok_or(DeviceError::FirmwareNotFound { directory, vfpga })
}

fn repo_root() -> Result<PathBuf, DeviceError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_capture_channel_lookup_finds_match() {
        let capture = NormalizedCapture {
            sample_rate_hz: CAPTURE_SAMPLE_RATE_HZ,
            sample_period_ns: CAPTURE_SAMPLE_PERIOD_NS,
            channels: vec![
                ChannelCapture {
                    channel: ChannelId::Ch1,
                    samples: vec![1, 2, 3],
                    trigger_index: Some(0),
                    frequency_hz: Some(1_000.0),
                },
                ChannelCapture {
                    channel: ChannelId::Ch2,
                    samples: vec![4, 5, 6],
                    trigger_index: Some(0),
                    frequency_hz: None,
                },
            ],
        };

        assert_eq!(
            capture
                .channel(ChannelId::Ch2)
                .map(|channel| channel.samples.clone()),
            Some(vec![4, 5, 6])
        );
        assert_eq!(
            capture
                .channel(ChannelId::Ch1)
                .map(|channel| channel.samples.len()),
            Some(3)
        );
    }

    #[test]
    fn extract_non_roll_samples_returns_center_window() {
        let frame = RawDataFrame {
            header: owon_protocol::DataFrameHeader {
                channel: ChannelId::Ch1,
                time_sum: 0,
                period_num: 0,
                cursor_from_right: GET_DATA_PERTINENT_SAMPLES as u16,
            },
            trigger_buffer: vec![0; owon_protocol::GET_DATA_TRIGGER_BUFFER_SIZE],
            adc_samples: (0..GET_DATA_ADC_SIZE)
                .map(|index| (index % 251) as u8)
                .collect(),
        };

        let samples = extract_samples(&frame, GET_DATA_EDGE_MARGIN_SAMPLES)
            .expect("non-roll extraction should work");
        assert_eq!(samples.len(), GET_DATA_PERTINENT_SAMPLES);
        assert_eq!(samples[0], 50);
        assert_eq!(samples[1], 51);
        assert_eq!(samples[4999], 29);
    }

    #[test]
    fn normalize_channel_frame_accepts_right_aligned_cursor_zero() {
        let frame = RawDataFrame {
            header: owon_protocol::DataFrameHeader {
                channel: ChannelId::Ch1,
                time_sum: 10,
                period_num: 2,
                cursor_from_right: 0,
            },
            trigger_buffer: vec![0; owon_protocol::GET_DATA_TRIGGER_BUFFER_SIZE],
            adc_samples: (0..GET_DATA_ADC_SIZE)
                .map(|index| (index % 251) as u8)
                .collect(),
        };

        let (capture, debug) =
            normalize_channel_frame(frame).expect("cursor zero fallback should work");
        assert_eq!(capture.samples.len(), GET_DATA_PERTINENT_SAMPLES);
        assert_eq!(capture.samples[0], 100);
        assert_eq!(capture.trigger_index, None);
        assert_eq!(capture.frequency_hz, Some(20_000_000.0));
        assert_eq!(debug.sample_offset, 100);
        assert_eq!(debug.sample_min_i8, -128);
        assert_eq!(debug.sample_max_i8, 127);
    }

    #[test]
    fn select_firmware_image_picks_expected_v1_blob() {
        let path = select_firmware_image(1).expect("firmware image should exist in repo");
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("VDS1022_FPGAV1_V3.9.bin")
        );
    }

    #[test]
    fn byte_sample_stats_tracks_signed_and_unsigned_ranges() {
        let stats = byte_sample_stats(&[0, 1, 2, 255, 254]);
        assert_eq!(stats.min_u8, 0);
        assert_eq!(stats.max_u8, 255);
        assert_eq!(stats.min_i8, -2);
        assert_eq!(stats.max_i8, 2);
    }
}
