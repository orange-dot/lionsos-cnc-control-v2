use std::error::Error;
use std::fmt;

pub const READ_FLASH_ADDRESS: u32 = 0x01b0;
pub const READ_FLASH_ARG: u8 = 1;
pub const QUERY_FPGA_ADDRESS: u32 = 0x0223;
pub const QUERY_FPGA_ARG: u8 = 0;
pub const LOAD_FPGA_ADDRESS: u32 = 0x4000;
pub const GET_MACHINE_ADDRESS: u32 = 0x4001;
pub const GET_MACHINE_ARG: u8 = b'V';
pub const GET_DATA_ADDRESS: u32 = 0x1000;
pub const GET_DATA_VALUE_SIZE: u8 = 2;
pub const GET_DATA_CHANNEL_OFF: u8 = 0x04;
pub const GET_DATA_CHANNEL_ON: u8 = 0x05;
pub const SET_PEAKMODE_ADDRESS: u32 = 0x0009;
pub const SET_ROLLMODE_ADDRESS: u32 = 0x000a;
pub const SET_CHL_ON_ADDRESS: u32 = 0x000b;
pub const SET_MULTI_ADDRESS: u32 = 0x0006;
pub const SET_PHASEFINE_ADDRESS: u32 = 0x0018;
pub const SET_TRIGGER_ADDRESS: u32 = 0x0024;
pub const SET_TRG_HOLDOFF_CH1_ADDRESS: u32 = 0x0026;
pub const SET_TRG_HOLDOFF_CH2_ADDRESS: u32 = 0x002a;
pub const SET_EDGE_LEVEL_CH1_ADDRESS: u32 = 0x002e;
pub const SET_EDGE_LEVEL_CH2_ADDRESS: u32 = 0x0030;
pub const SET_FREQREF_CH1_ADDRESS: u32 = 0x004a;
pub const SET_FREQREF_CH2_ADDRESS: u32 = 0x004b;
pub const SET_TIMEBASE_ADDRESS: u32 = 0x0052;
pub const SET_SUF_TRG_ADDRESS: u32 = 0x0056;
pub const SET_PRE_TRG_ADDRESS: u32 = 0x005a;
pub const SET_DEEPMEMORY_ADDRESS: u32 = 0x005c;
pub const SET_ZERO_OFF_CH1_ADDRESS: u32 = 0x010a;
pub const SET_ZERO_OFF_CH2_ADDRESS: u32 = 0x0108;
pub const SET_CHANNEL_CH1_ADDRESS: u32 = 0x0111;
pub const SET_CHANNEL_CH2_ADDRESS: u32 = 0x0110;
pub const SET_VOLT_GAIN_CH1_ADDRESS: u32 = 0x0116;
pub const SET_VOLT_GAIN_CH2_ADDRESS: u32 = 0x0114;

pub const U32_RESPONSE_SIZE: usize = 5;
pub const FLASH_SIZE: usize = 2002;
pub const FLASH_SUMMARY_OFFSET: usize = 206;
pub const FLASH_LOCALES_SIZE: usize = 100;
pub const FPGA_FRAME_HEADER_SIZE: usize = 4;
pub const STATUS_SUCCESS: u8 = b'S';
pub const GET_DATA_FRAME_SIZE: usize = 5211;
pub const GET_DATA_NOT_READY_SIZE: usize = 5;
pub const GET_DATA_TRIGGER_BUFFER_SIZE: usize = 100;
pub const GET_DATA_ADC_SIZE: usize = 5100;
pub const GET_DATA_PERTINENT_SAMPLES: usize = 5000;
pub const GET_DATA_EDGE_MARGIN_SAMPLES: usize = 50;
pub const GET_DATA_NON_ROLL_SAMPLE_OFFSET: usize = 161;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U32Response {
    pub status: u8,
    pub value: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelId {
    Ch1,
    Ch2,
}

impl ChannelId {
    pub fn from_wire(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::Ch1),
            1 => Ok(Self::Ch2),
            other => Err(ProtocolError::UnknownChannel(other)),
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Ch1 => "ch1",
            Self::Ch2 => "ch2",
        }
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ch1 => write!(f, "CH1"),
            Self::Ch2 => write!(f, "CH2"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineKind {
    Error,
    Vds1022,
    Vds2052,
    Unknown(u32),
}

impl fmt::Display for MachineKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Vds1022 => write!(f, "VDS1022"),
            Self::Vds2052 => write!(f, "VDS2052"),
            Self::Unknown(value) => write!(f, "unknown({value})"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpgaState {
    Missing,
    Loaded,
    Unknown(u32),
}

impl fmt::Display for FpgaState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => write!(f, "missing"),
            Self::Loaded => write!(f, "loaded"),
            Self::Unknown(value) => write!(f, "unknown({value})"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlashSummary {
    pub header: u16,
    pub version: u32,
    pub oem: u8,
    pub device_version: String,
    pub serial: String,
    pub nonzero_locale_bytes: usize,
    pub phasefine: u16,
    pub inferred_vfpga: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFrameHeader {
    pub channel: ChannelId,
    pub time_sum: u32,
    pub period_num: u32,
    pub cursor_from_right: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawDataFrame {
    pub header: DataFrameHeader,
    pub trigger_buffer: Vec<u8>,
    pub adc_samples: Vec<u8>,
}

#[derive(Debug)]
pub enum ProtocolError {
    UnexpectedLength { expected: usize, actual: usize },
    InvalidFlashHeader(u16),
    InvalidFlashVersion(u32),
    MissingTerminator(&'static str),
    FieldOutOfRange(&'static str),
    InvalidDeviceVersion(String),
    ValueOutOfRange { field: &'static str, value: usize },
    UnknownChannel(u8),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedLength { expected, actual } => write!(
                f,
                "Unexpected response length: expected {expected} bytes, got {actual}"
            ),
            Self::InvalidFlashHeader(header) => {
                write!(f, "Unexpected flash header: 0x{header:04x}")
            }
            Self::InvalidFlashVersion(version) => {
                write!(f, "Unexpected flash version: {version}")
            }
            Self::MissingTerminator(field) => {
                write!(
                    f,
                    "Missing NUL terminator while parsing flash field `{field}`"
                )
            }
            Self::FieldOutOfRange(field) => {
                write!(f, "Flash field `{field}` is outside the available buffer")
            }
            Self::InvalidDeviceVersion(version) => {
                write!(
                    f,
                    "Unable to infer FPGA version from device version `{version}`"
                )
            }
            Self::ValueOutOfRange { field, value } => {
                write!(
                    f,
                    "Value for `{field}` is too large for the protocol: {value}"
                )
            }
            Self::UnknownChannel(value) => {
                write!(f, "Unexpected channel identifier: 0x{value:02x}")
            }
        }
    }
}

impl Error for ProtocolError {}

pub fn pack_u8_command(address: u32, arg: u8) -> [u8; 6] {
    let mut buffer = [0u8; 6];
    buffer[..4].copy_from_slice(&address.to_le_bytes());
    buffer[4] = 1;
    buffer[5] = arg;
    buffer
}

pub fn pack_get_machine_command() -> [u8; 6] {
    pack_u8_command(GET_MACHINE_ADDRESS, GET_MACHINE_ARG)
}

pub fn pack_u16_command(address: u32, arg: u16) -> [u8; 7] {
    let mut buffer = [0u8; 7];
    buffer[..4].copy_from_slice(&address.to_le_bytes());
    buffer[4] = 2;
    buffer[5..].copy_from_slice(&arg.to_le_bytes());
    buffer
}

pub fn pack_u32_command(address: u32, arg: u32) -> [u8; 9] {
    let mut buffer = [0u8; 9];
    buffer[..4].copy_from_slice(&address.to_le_bytes());
    buffer[4] = 4;
    buffer[5..].copy_from_slice(&arg.to_le_bytes());
    buffer
}

pub fn pack_query_fpga_command() -> [u8; 6] {
    pack_u8_command(QUERY_FPGA_ADDRESS, QUERY_FPGA_ARG)
}

pub fn pack_read_flash_command() -> [u8; 6] {
    pack_u8_command(READ_FLASH_ADDRESS, READ_FLASH_ARG)
}

pub fn pack_get_data_command(ch1_on: bool, ch2_on: bool) -> [u8; 7] {
    [
        GET_DATA_ADDRESS.to_le_bytes()[0],
        GET_DATA_ADDRESS.to_le_bytes()[1],
        GET_DATA_ADDRESS.to_le_bytes()[2],
        GET_DATA_ADDRESS.to_le_bytes()[3],
        GET_DATA_VALUE_SIZE,
        if ch1_on {
            GET_DATA_CHANNEL_ON
        } else {
            GET_DATA_CHANNEL_OFF
        },
        if ch2_on {
            GET_DATA_CHANNEL_ON
        } else {
            GET_DATA_CHANNEL_OFF
        },
    ]
}

pub fn pack_set_channels_on_command(ch1_on: bool, ch2_on: bool) -> [u8; 6] {
    let mask = u8::from(ch1_on) | (u8::from(ch2_on) << 1);
    pack_u8_command(SET_CHL_ON_ADDRESS, mask)
}

pub fn pack_set_peakmode_command(enabled: bool) -> [u8; 6] {
    pack_u8_command(SET_PEAKMODE_ADDRESS, enabled.into())
}

pub fn pack_set_rollmode_command(enabled: bool) -> [u8; 6] {
    pack_u8_command(SET_ROLLMODE_ADDRESS, enabled.into())
}

pub fn pack_set_timebase_command(prescaler: u32) -> [u8; 9] {
    pack_u32_command(SET_TIMEBASE_ADDRESS, prescaler)
}

pub fn pack_set_multi_command(value: u16) -> [u8; 7] {
    pack_u16_command(SET_MULTI_ADDRESS, value)
}

pub fn pack_set_phasefine_command(value: u16) -> [u8; 7] {
    pack_u16_command(SET_PHASEFINE_ADDRESS, value)
}

pub fn pack_set_post_trigger_command(samples: u32) -> [u8; 9] {
    pack_u32_command(SET_SUF_TRG_ADDRESS, samples)
}

pub fn pack_set_pre_trigger_command(samples: u16) -> [u8; 7] {
    pack_u16_command(SET_PRE_TRG_ADDRESS, samples)
}

pub fn pack_set_deepmemory_command(samples: u16) -> [u8; 7] {
    pack_u16_command(SET_DEEPMEMORY_ADDRESS, samples)
}

pub fn pack_set_trigger_default_command() -> [u8; 7] {
    pack_u16_command(SET_TRIGGER_ADDRESS, 0)
}

pub fn pack_set_trigger_holdoff_command(channel: ChannelId, value: u16) -> [u8; 7] {
    pack_u16_command(
        match channel {
            ChannelId::Ch1 => SET_TRG_HOLDOFF_CH1_ADDRESS,
            ChannelId::Ch2 => SET_TRG_HOLDOFF_CH2_ADDRESS,
        },
        value,
    )
}

pub fn pack_set_edge_level_command(channel: ChannelId, value: u16) -> [u8; 7] {
    pack_u16_command(
        match channel {
            ChannelId::Ch1 => SET_EDGE_LEVEL_CH1_ADDRESS,
            ChannelId::Ch2 => SET_EDGE_LEVEL_CH2_ADDRESS,
        },
        value,
    )
}

pub fn pack_set_freqref_command(channel: ChannelId, value: u8) -> [u8; 6] {
    pack_u8_command(
        match channel {
            ChannelId::Ch1 => SET_FREQREF_CH1_ADDRESS,
            ChannelId::Ch2 => SET_FREQREF_CH2_ADDRESS,
        },
        value,
    )
}

pub fn pack_set_zero_offset_command(channel: ChannelId, value: u16) -> [u8; 7] {
    pack_u16_command(
        match channel {
            ChannelId::Ch1 => SET_ZERO_OFF_CH1_ADDRESS,
            ChannelId::Ch2 => SET_ZERO_OFF_CH2_ADDRESS,
        },
        value,
    )
}

pub fn pack_set_voltage_gain_command(channel: ChannelId, value: u16) -> [u8; 7] {
    pack_u16_command(
        match channel {
            ChannelId::Ch1 => SET_VOLT_GAIN_CH1_ADDRESS,
            ChannelId::Ch2 => SET_VOLT_GAIN_CH2_ADDRESS,
        },
        value,
    )
}

pub fn pack_set_channel_command(channel: ChannelId, value: u8) -> [u8; 6] {
    pack_u8_command(
        match channel {
            ChannelId::Ch1 => SET_CHANNEL_CH1_ADDRESS,
            ChannelId::Ch2 => SET_CHANNEL_CH2_ADDRESS,
        },
        value,
    )
}

pub fn pack_load_fpga_command(image_len: usize) -> Result<[u8; 9], ProtocolError> {
    let image_len = u32::try_from(image_len).map_err(|_| ProtocolError::ValueOutOfRange {
        field: "image_len",
        value: image_len,
    })?;
    Ok(pack_u32_command(LOAD_FPGA_ADDRESS, image_len))
}

pub fn pack_load_fpga_chunk(part_index: u32, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(FPGA_FRAME_HEADER_SIZE + payload.len());
    frame.extend_from_slice(&part_index.to_le_bytes());
    frame.extend_from_slice(payload);
    frame
}

pub fn parse_u32_response(buffer: &[u8]) -> Result<U32Response, ProtocolError> {
    if buffer.len() != U32_RESPONSE_SIZE {
        return Err(ProtocolError::UnexpectedLength {
            expected: U32_RESPONSE_SIZE,
            actual: buffer.len(),
        });
    }

    Ok(U32Response {
        status: buffer[0],
        value: u32::from_le_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]),
    })
}

pub fn parse_get_data_frame(buffer: &[u8]) -> Result<RawDataFrame, ProtocolError> {
    if buffer.len() != GET_DATA_FRAME_SIZE {
        return Err(ProtocolError::UnexpectedLength {
            expected: GET_DATA_FRAME_SIZE,
            actual: buffer.len(),
        });
    }

    let header = DataFrameHeader {
        channel: ChannelId::from_wire(buffer[0])?,
        time_sum: u32::from_le_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]),
        period_num: u32::from_le_bytes([buffer[5], buffer[6], buffer[7], buffer[8]]),
        cursor_from_right: u16::from_le_bytes([buffer[9], buffer[10]]),
    };

    let trigger_start = 11;
    let trigger_end = trigger_start + GET_DATA_TRIGGER_BUFFER_SIZE;
    let adc_end = trigger_end + GET_DATA_ADC_SIZE;

    Ok(RawDataFrame {
        header,
        trigger_buffer: buffer[trigger_start..trigger_end].to_vec(),
        adc_samples: buffer[trigger_end..adc_end].to_vec(),
    })
}

pub fn machine_kind(value: u32) -> MachineKind {
    match value {
        0 => MachineKind::Error,
        1 => MachineKind::Vds1022,
        3 => MachineKind::Vds2052,
        other => MachineKind::Unknown(other),
    }
}

pub fn fpga_state(value: u32) -> FpgaState {
    match value {
        0 => FpgaState::Missing,
        1 => FpgaState::Loaded,
        other => FpgaState::Unknown(other),
    }
}

pub fn parse_flash_summary(buffer: &[u8]) -> Result<FlashSummary, ProtocolError> {
    if buffer.len() != FLASH_SIZE {
        return Err(ProtocolError::UnexpectedLength {
            expected: FLASH_SIZE,
            actual: buffer.len(),
        });
    }

    let header = u16::from_le_bytes([buffer[0], buffer[1]]);
    if header != 0x55aa && header != 0xaa55 {
        return Err(ProtocolError::InvalidFlashHeader(header));
    }

    let version = u32::from_le_bytes([buffer[2], buffer[3], buffer[4], buffer[5]]);
    if version != 2 {
        return Err(ProtocolError::InvalidFlashVersion(version));
    }

    if FLASH_SUMMARY_OFFSET >= buffer.len() {
        return Err(ProtocolError::FieldOutOfRange("flash_summary"));
    }

    let mut cursor = FLASH_SUMMARY_OFFSET;
    let oem = buffer[cursor];
    cursor += 1;

    let (device_version, next_cursor) = parse_c_string(buffer, cursor, "device_version")?;
    let (serial, next_cursor) = parse_c_string(buffer, next_cursor, "serial")?;
    cursor = next_cursor;

    if cursor + FLASH_LOCALES_SIZE + 2 > buffer.len() {
        return Err(ProtocolError::FieldOutOfRange("locales_phasefine"));
    }

    let locales_end = cursor + FLASH_LOCALES_SIZE;
    let nonzero_locale_bytes = buffer[cursor..locales_end]
        .iter()
        .copied()
        .filter(|byte| *byte != 0)
        .count();
    let phasefine = u16::from_le_bytes([buffer[locales_end], buffer[locales_end + 1]]);
    let inferred_vfpga = infer_vfpga_version(&device_version)?;

    Ok(FlashSummary {
        header,
        version,
        oem,
        device_version,
        serial,
        nonzero_locale_bytes,
        phasefine,
        inferred_vfpga,
    })
}

fn parse_c_string(
    buffer: &[u8],
    start: usize,
    field: &'static str,
) -> Result<(String, usize), ProtocolError> {
    if start >= buffer.len() {
        return Err(ProtocolError::FieldOutOfRange(field));
    }

    let relative_end = buffer[start..]
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(ProtocolError::MissingTerminator(field))?;
    let end = start + relative_end;
    let value = String::from_utf8_lossy(&buffer[start..end]).into_owned();
    Ok((value, end + 1))
}

fn infer_vfpga_version(version: &str) -> Result<u32, ProtocolError> {
    let upper = version.to_ascii_uppercase();
    if upper.starts_with("V2.7.0") {
        return Ok(3);
    }
    if upper.starts_with("V2.4.623") || upper.starts_with("V2.6.0") {
        return Ok(2);
    }
    if upper.starts_with("V2.") || upper.starts_with("V1.") {
        return Ok(1);
    }
    if let Some(rest) = upper.strip_prefix('V') {
        let Some(dot_index) = rest.find('.') else {
            return Err(ProtocolError::InvalidDeviceVersion(version.to_owned()));
        };
        return rest[..dot_index]
            .parse::<u32>()
            .map_err(|_| ProtocolError::InvalidDeviceVersion(version.to_owned()));
    }

    Err(ProtocolError::InvalidDeviceVersion(version.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_get_data_command_enables_both_channels() {
        assert_eq!(
            pack_get_data_command(true, true),
            [0x00, 0x10, 0x00, 0x00, 0x02, 0x05, 0x05]
        );
    }

    #[test]
    fn parse_get_data_frame_splits_header_trigger_and_adc_regions() {
        let mut frame = vec![0u8; GET_DATA_FRAME_SIZE];
        frame[0] = 1;
        frame[1..5].copy_from_slice(&1234u32.to_le_bytes());
        frame[5..9].copy_from_slice(&5678u32.to_le_bytes());
        frame[9..11].copy_from_slice(&5000u16.to_le_bytes());

        for (index, byte) in frame[11..11 + GET_DATA_TRIGGER_BUFFER_SIZE]
            .iter_mut()
            .enumerate()
        {
            *byte = index as u8;
        }
        for (index, byte) in frame[11 + GET_DATA_TRIGGER_BUFFER_SIZE..]
            .iter_mut()
            .enumerate()
        {
            *byte = (index % 251) as u8;
        }

        let parsed = parse_get_data_frame(&frame).expect("frame should parse");
        assert_eq!(parsed.header.channel, ChannelId::Ch2);
        assert_eq!(parsed.header.time_sum, 1234);
        assert_eq!(parsed.header.period_num, 5678);
        assert_eq!(parsed.header.cursor_from_right, 5000);
        assert_eq!(parsed.trigger_buffer.len(), GET_DATA_TRIGGER_BUFFER_SIZE);
        assert_eq!(parsed.trigger_buffer[0], 0);
        assert_eq!(parsed.trigger_buffer[99], 99);
        assert_eq!(parsed.adc_samples.len(), GET_DATA_ADC_SIZE);
        assert_eq!(parsed.adc_samples[0], 0);
        assert_eq!(parsed.adc_samples[1], 1);
    }
}
