use owon_analysis::{AnalysisReport, AnalysisRequest};
use owon_device::CaptureOnceResult;

use crate::saved_capture::SavedCaptureEntry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperation {
    Connect,
    Disconnect,
    CaptureOnce,
    StartLive,
    StopLive,
    LoadSavedCapture,
    RunAnalysis,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiModel {
    connected: bool,
    live_running: bool,
    pending: Option<PendingOperation>,
    status_text: String,
    error_text: Option<String>,
    device_summary: String,
    last_capture: Option<CaptureOnceResult>,
    last_capture_source: Option<String>,
    last_capture_note: Option<String>,
    saved_capture_label: Option<String>,
    saved_captures: Vec<SavedCaptureEntry>,
    analysis_status_text: String,
    analysis_error_text: Option<String>,
    analysis_request: Option<AnalysisRequest>,
    analysis_result: Option<AnalysisReport>,
}

impl Default for UiModel {
    fn default() -> Self {
        Self {
            connected: false,
            live_running: false,
            pending: None,
            status_text: "Disconnected.".to_owned(),
            error_text: None,
            device_summary: disconnected_device_summary(),
            last_capture: None,
            last_capture_source: None,
            last_capture_note: None,
            saved_capture_label: None,
            saved_captures: Vec::new(),
            analysis_status_text: "Load a saved capture to unlock offline analysis.".to_owned(),
            analysis_error_text: None,
            analysis_request: None,
            analysis_result: None,
        }
    }
}

impl UiModel {
    #[cfg(test)]
    pub fn connected(&self) -> bool {
        self.connected
    }

    #[cfg(test)]
    pub fn live_running(&self) -> bool {
        self.live_running
    }

    #[cfg(test)]
    pub fn pending(&self) -> Option<PendingOperation> {
        self.pending
    }

    pub fn status_text(&self) -> &str {
        &self.status_text
    }

    pub fn error_text(&self) -> Option<&str> {
        self.error_text.as_deref()
    }

    pub fn device_summary(&self) -> &str {
        &self.device_summary
    }

    pub fn last_capture(&self) -> Option<&CaptureOnceResult> {
        self.last_capture.as_ref()
    }

    pub fn last_capture_note(&self) -> Option<&str> {
        self.last_capture_note.as_deref()
    }

    pub fn saved_captures(&self) -> &[SavedCaptureEntry] {
        &self.saved_captures
    }

    pub fn is_live_running(&self) -> bool {
        self.live_running
    }

    pub fn analysis_status_text(&self) -> &str {
        &self.analysis_status_text
    }

    pub fn analysis_error_text(&self) -> Option<&str> {
        self.analysis_error_text.as_deref()
    }

    pub fn analysis_request(&self) -> Option<&AnalysisRequest> {
        self.analysis_request.as_ref()
    }

    pub fn analysis_result(&self) -> Option<&AnalysisReport> {
        self.analysis_result.as_ref()
    }

    pub fn has_saved_capture_loaded(&self) -> bool {
        self.saved_capture_label.is_some() && self.last_capture.is_some()
    }

    pub fn can_run_analysis(&self) -> bool {
        self.has_saved_capture_loaded()
            && !self.live_running
            && !matches!(
                self.pending,
                Some(
                    PendingOperation::Connect
                        | PendingOperation::Disconnect
                        | PendingOperation::CaptureOnce
                        | PendingOperation::StartLive
                        | PendingOperation::StopLive
                        | PendingOperation::LoadSavedCapture
                        | PendingOperation::RunAnalysis
                )
            )
    }

    pub fn saved_captures_summary(&self) -> String {
        match self.saved_captures.len() {
            0 => "No saved captures found in the helper-pack root.".to_owned(),
            1 => "1 saved capture found in the helper-pack root.".to_owned(),
            count => format!("{count} saved captures found in the helper-pack root."),
        }
    }

    pub fn can_connect(&self) -> bool {
        !self.connected && self.pending.is_none()
    }

    pub fn can_disconnect(&self) -> bool {
        self.connected && self.pending.is_none()
    }

    pub fn can_capture_once(&self) -> bool {
        self.connected && self.pending.is_none() && !self.live_running
    }

    pub fn can_start_live(&self) -> bool {
        self.connected && self.pending.is_none() && !self.live_running
    }

    pub fn can_stop_live(&self) -> bool {
        self.live_running && self.pending.is_none()
    }

    pub fn can_interact_with_saved_captures(&self) -> bool {
        self.pending.is_none() && !self.live_running
    }

    pub fn connection_text(&self) -> &'static str {
        match self.pending {
            Some(PendingOperation::Connect) => "Connection: opening device",
            Some(PendingOperation::Disconnect) => "Connection: closing session",
            _ if self.connected => "Connection: connected",
            _ => "Connection: disconnected",
        }
    }

    pub fn live_button_label(&self) -> &'static str {
        if self.live_running || matches!(self.pending, Some(PendingOperation::StartLive)) {
            "Stop Live"
        } else {
            "Start Live"
        }
    }

    pub fn waveform_hint(&self) -> String {
        if let Some(result) = &self.last_capture {
            let channel_count = result.capture.channels.len();
            let sample_count = result
                .capture
                .channels
                .first()
                .map(|channel| channel.samples.len())
                .unwrap_or(0);
            let source = self
                .last_capture_source
                .as_deref()
                .unwrap_or("Latest capture");
            format!("{source}: {channel_count} channel(s), {sample_count} samples per channel.")
        } else if self.live_running || matches!(self.pending, Some(PendingOperation::StartLive)) {
            "Live capture is starting. The waveform will update as new frames arrive.".to_owned()
        } else if self.connected {
            "No capture yet. Press Capture Once or Start Live.".to_owned()
        } else if !self.saved_captures.is_empty() {
            "Select a saved capture or connect to the device.".to_owned()
        } else {
            "Connect to the OWON VDS1022 to fetch a waveform.".to_owned()
        }
    }

    pub fn analysis_hint(&self) -> String {
        if self.has_saved_capture_loaded() {
            if let Some(result) = &self.analysis_result {
                let peak = result
                    .spectrum
                    .peaks
                    .first()
                    .map(|peak| format!("{:.1} Hz", peak.frequency_hz))
                    .unwrap_or_else(|| "n/a".to_owned());
                format!(
                    "Offline analysis on {} selection samples. Strongest spectral peak: {peak}.",
                    result.raw_samples.len()
                )
            } else if matches!(self.pending, Some(PendingOperation::RunAnalysis)) {
                "Offline analysis is running over raw saved-session bins.".to_owned()
            } else {
                "Saved capture loaded. Run offline analysis or adjust the channel/selection first."
                    .to_owned()
            }
        } else {
            "Analysis works on saved sessions only. Load a saved capture to enable heavy offline plots and decoders."
                .to_owned()
        }
    }

    pub fn begin_connect(&mut self) -> bool {
        if !self.can_connect() {
            return false;
        }
        self.pending = Some(PendingOperation::Connect);
        self.status_text = "Opening device session...".to_owned();
        self.error_text = None;
        true
    }

    pub fn begin_disconnect(&mut self) -> bool {
        if !self.can_disconnect() {
            return false;
        }
        self.pending = Some(PendingOperation::Disconnect);
        self.status_text = if self.live_running {
            "Stopping live capture and closing device session...".to_owned()
        } else {
            "Closing device session...".to_owned()
        };
        self.error_text = None;
        true
    }

    pub fn begin_capture_once(&mut self) -> bool {
        if !self.can_capture_once() {
            return false;
        }
        self.pending = Some(PendingOperation::CaptureOnce);
        self.status_text = "Capturing waveform...".to_owned();
        self.error_text = None;
        true
    }

    pub fn begin_start_live(&mut self) -> bool {
        if !self.can_start_live() {
            return false;
        }
        self.pending = Some(PendingOperation::StartLive);
        self.status_text = "Starting live capture...".to_owned();
        self.error_text = None;
        true
    }

    pub fn begin_stop_live(&mut self) -> bool {
        if !self.can_stop_live() {
            return false;
        }
        self.pending = Some(PendingOperation::StopLive);
        self.status_text = "Stopping live capture...".to_owned();
        self.error_text = None;
        true
    }

    pub fn begin_load_saved_capture(&mut self) -> bool {
        if !self.can_interact_with_saved_captures() {
            return false;
        }
        self.pending = Some(PendingOperation::LoadSavedCapture);
        self.status_text = "Loading saved capture...".to_owned();
        self.error_text = None;
        true
    }

    pub fn begin_run_analysis(&mut self, request: AnalysisRequest) -> bool {
        if !self.can_run_analysis() {
            return false;
        }
        self.pending = Some(PendingOperation::RunAnalysis);
        self.analysis_status_text =
            "Running offline analysis over raw saved-session bins...".to_owned();
        self.analysis_error_text = None;
        self.analysis_request = Some(request);
        true
    }

    pub fn on_saved_captures_scanned(&mut self, captures: Vec<SavedCaptureEntry>) {
        self.saved_captures = captures;
    }

    pub fn on_connected(&mut self, device_summary: String) {
        self.connected = true;
        self.pending = None;
        self.status_text = "Connected to OWON VDS1022.".to_owned();
        self.error_text = None;
        self.device_summary = device_summary;
    }

    pub fn on_live_started(&mut self) {
        self.live_running = true;
        self.pending = None;
        self.status_text = "Live capture running...".to_owned();
        self.error_text = None;
        self.saved_capture_label = None;
        self.analysis_status_text = "Analysis is disabled while live capture is active.".to_owned();
        self.analysis_error_text = None;
        self.analysis_result = None;
        self.analysis_request = None;
    }

    pub fn on_live_retrying(&mut self, error: String, consecutive_failures: usize) {
        self.live_running = true;
        self.pending = None;
        self.status_text = "Live capture retrying after a transient device error...".to_owned();
        self.error_text = Some(format!("{error} (live retry {consecutive_failures}/3)"));
    }

    pub fn on_live_stopped(&mut self) {
        self.live_running = false;
        self.pending = None;
        self.status_text = "Live capture stopped.".to_owned();
        self.error_text = None;
    }

    pub fn on_disconnected(&mut self, warning: Option<String>) {
        self.connected = false;
        self.live_running = false;
        self.pending = None;
        self.status_text = match warning {
            Some(_) => "Disconnected with cleanup warning.".to_owned(),
            None => "Disconnected.".to_owned(),
        };
        self.error_text = warning;
        self.device_summary = disconnected_device_summary();
    }

    pub fn on_capture_ready(&mut self, result: CaptureOnceResult) {
        self.pending = None;
        self.status_text = "Capture complete.".to_owned();
        self.error_text = None;
        self.last_capture = Some(result);
        self.last_capture_source = Some("One-shot capture".to_owned());
        self.last_capture_note = None;
        self.saved_capture_label = None;
        self.analysis_result = None;
        self.analysis_request = None;
        self.analysis_status_text =
            "Analysis works on saved sessions only. Load a saved capture to unlock it.".to_owned();
        self.analysis_error_text = None;
    }

    pub fn on_live_capture_ready(&mut self, result: CaptureOnceResult) {
        self.live_running = true;
        self.pending = None;
        self.status_text = "Live capture running...".to_owned();
        self.error_text = None;
        self.last_capture = Some(result);
        self.last_capture_source = Some("Live capture".to_owned());
        self.last_capture_note = None;
        self.saved_capture_label = None;
        self.analysis_result = None;
        self.analysis_request = None;
        self.analysis_status_text = "Analysis is available only for saved sessions.".to_owned();
        self.analysis_error_text = None;
    }

    pub fn on_saved_capture_ready(
        &mut self,
        label: String,
        note: Option<String>,
        result: CaptureOnceResult,
    ) {
        self.live_running = false;
        self.pending = None;
        self.status_text = format!("Loaded saved capture: {label}.");
        self.error_text = None;
        self.last_capture = Some(result);
        self.last_capture_source = Some(format!("Saved capture `{label}`"));
        self.last_capture_note = note;
        self.saved_capture_label = Some(label);
        self.analysis_result = None;
        self.analysis_request = None;
        self.analysis_status_text =
            "Saved capture loaded. Ready for offline analysis over raw bins.".to_owned();
        self.analysis_error_text = None;
    }

    pub fn on_analysis_ready(&mut self, result: AnalysisReport) {
        self.pending = None;
        self.analysis_status_text = format!(
            "Offline analysis ready for {} over samples {}..{}.",
            result.channel, result.selection.start, result.selection.end
        );
        self.analysis_error_text = None;
        self.analysis_result = Some(result);
    }

    pub fn on_analysis_input_error(&mut self, error: String) {
        self.analysis_status_text = "Offline analysis input error.".to_owned();
        self.analysis_error_text = Some(error);
    }

    pub fn on_failure(&mut self, error: String) {
        let pending = self.pending.take();
        let live_failed = pending.is_none() && self.live_running;
        if matches!(
            pending,
            Some(PendingOperation::StartLive) | Some(PendingOperation::StopLive)
        ) || live_failed
        {
            self.live_running = false;
        }

        match pending {
            Some(PendingOperation::RunAnalysis) => {
                self.analysis_status_text = "Offline analysis failed.".to_owned();
                self.analysis_error_text = Some(error);
            }
            Some(PendingOperation::Connect) => {
                self.status_text = "Connection failed.".to_owned();
                self.connected = false;
                self.device_summary = disconnected_device_summary();
                self.error_text = Some(error);
            }
            Some(PendingOperation::Disconnect) => {
                self.status_text = "Disconnect failed.".to_owned();
                self.error_text = Some(error);
            }
            Some(PendingOperation::CaptureOnce) => {
                self.status_text = "Capture failed.".to_owned();
                self.error_text = Some(error);
            }
            Some(PendingOperation::StartLive) => {
                self.status_text = "Live capture failed to start.".to_owned();
                self.error_text = Some(error);
            }
            Some(PendingOperation::StopLive) => {
                self.status_text = "Live capture stop failed.".to_owned();
                self.error_text = Some(error);
            }
            Some(PendingOperation::LoadSavedCapture) => {
                self.status_text = "Saved capture load failed.".to_owned();
                self.error_text = Some(error);
            }
            None if live_failed => {
                self.status_text = "Live capture failed.".to_owned();
                self.error_text = Some(error);
            }
            None => {
                self.status_text = "Operation failed.".to_owned();
                self.error_text = Some(error);
            }
        }
    }
}

fn disconnected_device_summary() -> String {
    "Connect to load flash, FPGA, and transport details.".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use owon_analysis::AnalysisSelection;
    use owon_device::{
        CaptureDebugInfo, CaptureOnceResult, ChannelCalibrationDebugInfo, ChannelCapture,
        ChannelCaptureDebugInfo, NormalizedCapture,
    };
    use owon_protocol::ChannelId;

    #[test]
    fn connect_success_enables_capture_once_and_live() {
        let mut model = UiModel::default();

        assert!(model.begin_connect());
        assert_eq!(model.pending(), Some(PendingOperation::Connect));

        model.on_connected("Machine: VDS1022".to_owned());
        assert!(model.connected());
        assert!(model.can_capture_once());
        assert!(model.can_start_live());
        assert_eq!(model.pending(), None);
        assert_eq!(model.error_text(), None);
    }

    #[test]
    fn capture_failure_preserves_previous_capture() {
        let mut model = UiModel::default();
        let previous = sample_capture_result();

        model.on_connected("Machine: VDS1022".to_owned());
        model.on_capture_ready(previous.clone());
        assert!(model.begin_capture_once());

        model.on_failure("USB bulk read failed".to_owned());

        assert!(model.connected());
        assert_eq!(model.last_capture(), Some(&previous));
        assert_eq!(model.pending(), None);
        assert_eq!(model.status_text(), "Capture failed.");
    }

    #[test]
    fn live_start_success_disables_manual_actions_until_stopped() {
        let mut model = UiModel::default();

        model.on_connected("Machine: VDS1022".to_owned());
        assert!(model.begin_start_live());

        model.on_live_started();
        assert!(model.live_running());
        assert!(!model.can_capture_once());
        assert!(!model.can_start_live());
        assert!(model.can_stop_live());
        assert!(!model.can_interact_with_saved_captures());

        assert!(model.begin_stop_live());
        model.on_live_stopped();
        assert!(!model.live_running());
        assert!(model.can_capture_once());
        assert!(model.can_interact_with_saved_captures());
    }

    #[test]
    fn live_failure_stops_loop_and_preserves_previous_capture() {
        let mut model = UiModel::default();
        let previous = sample_capture_result();

        model.on_connected("Machine: VDS1022".to_owned());
        model.on_live_capture_ready(previous.clone());

        model.on_failure("USB bulk read failed".to_owned());

        assert!(!model.live_running());
        assert_eq!(model.last_capture(), Some(&previous));
        assert_eq!(model.status_text(), "Live capture failed.");
    }

    #[test]
    fn saved_capture_load_sets_source_and_enables_analysis() {
        let mut model = UiModel::default();
        let previous = sample_capture_result();

        assert!(model.begin_load_saved_capture());
        model.on_saved_capture_ready(
            "twocap-clean".to_owned(),
            Some("inferred debug".to_owned()),
            previous.clone(),
        );

        assert_eq!(model.last_capture(), Some(&previous));
        assert!(model.has_saved_capture_loaded());
        assert!(model.can_run_analysis());
        assert_eq!(
            model.last_capture_source.as_deref(),
            Some("Saved capture `twocap-clean`")
        );
        assert_eq!(model.last_capture_note(), Some("inferred debug"));
        assert_eq!(model.status_text(), "Loaded saved capture: twocap-clean.");
    }

    #[test]
    fn analysis_failure_is_reported_separately() {
        let mut model = UiModel::default();
        let request = AnalysisRequest {
            channel: ChannelId::Ch1,
            selection: AnalysisSelection { start: 0, end: 2 },
            histogram_bins: 32,
            spectrum_bins: 32,
            spectrogram_window: 64,
            spectrogram_step: 16,
            uart_baud: 115_200.0,
            uart_invert: false,
        };

        model.on_saved_capture_ready("fixture".to_owned(), None, sample_capture_result());
        assert!(model.begin_run_analysis(request));

        model.on_failure("bad selection".to_owned());

        assert_eq!(model.analysis_status_text(), "Offline analysis failed.");
        assert_eq!(model.analysis_error_text(), Some("bad selection"));
    }

    #[test]
    fn analysis_input_error_is_local_to_analysis_panel() {
        let mut model = UiModel::default();
        model.on_saved_capture_ready("fixture".to_owned(), None, sample_capture_result());

        model.on_analysis_input_error("UART baud must be positive".to_owned());

        assert_eq!(
            model.analysis_status_text(),
            "Offline analysis input error."
        );
        assert_eq!(
            model.analysis_error_text(),
            Some("UART baud must be positive")
        );
    }

    fn sample_capture_result() -> CaptureOnceResult {
        CaptureOnceResult {
            capture: NormalizedCapture {
                sample_rate_hz: 2_500_000.0,
                sample_period_ns: 400.0,
                channels: vec![
                    ChannelCapture {
                        channel: ChannelId::Ch1,
                        samples: vec![0, 127, 255, 32],
                        trigger_index: Some(1),
                        frequency_hz: Some(51.1),
                    },
                    ChannelCapture {
                        channel: ChannelId::Ch2,
                        samples: vec![176, 200, 226, 180],
                        trigger_index: Some(1),
                        frequency_hz: None,
                    },
                ],
            },
            debug: CaptureDebugInfo {
                requested_channels: vec![ChannelId::Ch1, ChannelId::Ch2],
                volt_range_index: 5,
                timebase_prescaler: 40,
                rollmode: false,
                peakmode: false,
                pre_trigger_samples: 4989,
                post_trigger_samples: 5011,
                trigger_holdoff: 0x8002,
                edge_level: 0xf600,
                freqref: 0xfb,
                phasefine: 0,
                calibrations: vec![
                    ChannelCalibrationDebugInfo {
                        channel: ChannelId::Ch1,
                        zero_offset: 535,
                        voltage_gain: 550,
                    },
                    ChannelCalibrationDebugInfo {
                        channel: ChannelId::Ch2,
                        zero_offset: 510,
                        voltage_gain: 545,
                    },
                ],
                channels: vec![
                    ChannelCaptureDebugInfo {
                        channel: ChannelId::Ch1,
                        cursor_from_right: 5108,
                        time_sum: 1_956_947,
                        period_num: 1,
                        sample_offset: 50,
                        sample_count: 5000,
                        sample_min_u8: 0,
                        sample_max_u8: 255,
                        sample_min_i8: -32,
                        sample_max_i8: 7,
                    },
                    ChannelCaptureDebugInfo {
                        channel: ChannelId::Ch2,
                        cursor_from_right: 5108,
                        time_sum: 0,
                        period_num: 0,
                        sample_offset: 50,
                        sample_count: 5000,
                        sample_min_u8: 176,
                        sample_max_u8: 226,
                        sample_min_i8: -80,
                        sample_max_i8: -30,
                    },
                ],
            },
        }
    }
}
