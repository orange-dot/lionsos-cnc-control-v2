use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use owon_analysis::{AnalysisReport, AnalysisRequest, analyze_capture};
use owon_device::{CaptureOnceResult, DeviceSession};

use crate::saved_capture::{
    LoadedSavedCapture, SavedCaptureEntry, load_saved_capture, scan_saved_captures,
};

const LIVE_CAPTURE_THROTTLE: Duration = Duration::from_millis(80);
const LIVE_CAPTURE_RETRY_BACKOFF: Duration = Duration::from_millis(250);
const LIVE_CAPTURE_MAX_CONSECUTIVE_FAILURES: usize = 3;

#[derive(Debug)]
enum WorkerCommand {
    Connect,
    Disconnect,
    CaptureOnce,
    StartLiveCapture,
    StopLiveCapture,
    RefreshSavedCaptures,
    LoadSavedCapture(PathBuf),
    RunAnalysis {
        capture: CaptureOnceResult,
        request: AnalysisRequest,
    },
    Quit,
}

#[derive(Debug)]
pub enum WorkerEvent {
    Connected {
        device_summary: String,
    },
    LiveStarted,
    LiveRetrying {
        error: String,
        consecutive_failures: usize,
    },
    LiveStopped,
    Disconnected {
        warning: Option<String>,
    },
    CaptureReady(CaptureOnceResult),
    LiveCaptureReady(CaptureOnceResult),
    SavedCapturesScanned(Vec<SavedCaptureEntry>),
    SavedCaptureLoaded {
        entry: SavedCaptureEntry,
        note: Option<String>,
        result: CaptureOnceResult,
    },
    AnalysisReady {
        result: AnalysisReport,
    },
    Failed(String),
}

#[derive(Clone)]
pub struct WorkerHandle {
    sender: Sender<WorkerCommand>,
}

impl WorkerHandle {
    pub fn connect(&self) -> Result<(), String> {
        self.send(WorkerCommand::Connect)
    }

    pub fn disconnect(&self) -> Result<(), String> {
        self.send(WorkerCommand::Disconnect)
    }

    pub fn capture_once(&self) -> Result<(), String> {
        self.send(WorkerCommand::CaptureOnce)
    }

    pub fn start_live_capture(&self) -> Result<(), String> {
        self.send(WorkerCommand::StartLiveCapture)
    }

    pub fn stop_live_capture(&self) -> Result<(), String> {
        self.send(WorkerCommand::StopLiveCapture)
    }

    pub fn refresh_saved_captures(&self) -> Result<(), String> {
        self.send(WorkerCommand::RefreshSavedCaptures)
    }

    pub fn load_saved_capture(&self, meta_path: PathBuf) -> Result<(), String> {
        self.send(WorkerCommand::LoadSavedCapture(meta_path))
    }

    pub fn run_analysis(
        &self,
        capture: CaptureOnceResult,
        request: AnalysisRequest,
    ) -> Result<(), String> {
        self.send(WorkerCommand::RunAnalysis { capture, request })
    }

    pub fn quit(&self) -> Result<(), String> {
        self.send(WorkerCommand::Quit)
    }

    fn send(&self, command: WorkerCommand) -> Result<(), String> {
        self.sender
            .send(command)
            .map_err(|error| format!("Background worker channel closed: {error}"))
    }
}

pub fn spawn_worker() -> (WorkerHandle, Receiver<WorkerEvent>) {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();

    thread::spawn(move || worker_loop(command_rx, event_tx));

    (WorkerHandle { sender: command_tx }, event_rx)
}

fn worker_loop(command_rx: Receiver<WorkerCommand>, event_tx: Sender<WorkerEvent>) {
    let mut session: Option<DeviceSession> = None;
    let mut live_running = false;
    let mut live_failures = 0usize;

    loop {
        let command = if live_running {
            match command_rx.try_recv() {
                Ok(command) => Some(command),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => break,
            }
        } else {
            match command_rx.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            }
        };

        if let Some(command) = command {
            match command {
                WorkerCommand::Connect => {
                    if session.is_none() {
                        match DeviceSession::open() {
                            Ok(new_session) => {
                                let summary = format_device_summary(&new_session);
                                session = Some(new_session);
                                if event_tx
                                    .send(WorkerEvent::Connected {
                                        device_summary: summary,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(error) => {
                                if event_tx
                                    .send(WorkerEvent::Failed(error.to_string()))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    } else if let Some(existing) = session.as_ref() {
                        if event_tx
                            .send(WorkerEvent::Connected {
                                device_summary: format_device_summary(existing),
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                }
                WorkerCommand::Disconnect => {
                    live_running = false;
                    live_failures = 0;
                    if let Some(existing) = session.as_mut() {
                        existing.stop_live_capture();
                    }
                    let warning = match session.take() {
                        Some(existing) => existing.close().err().map(|error| error.to_string()),
                        None => None,
                    };
                    if event_tx
                        .send(WorkerEvent::Disconnected { warning })
                        .is_err()
                    {
                        break;
                    }
                }
                WorkerCommand::CaptureOnce => {
                    if live_running {
                        if event_tx
                            .send(WorkerEvent::Failed(
                                "Stop live capture before running a one-shot capture.".to_owned(),
                            ))
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }

                    let Some(existing) = session.as_mut() else {
                        if event_tx
                            .send(WorkerEvent::Failed(
                                "Connect to the device before capturing.".to_owned(),
                            ))
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    };

                    match existing.capture_once_with_debug() {
                        Ok(result) => {
                            if event_tx.send(WorkerEvent::CaptureReady(result)).is_err() {
                                break;
                            }
                        }
                        Err(error) => {
                            if event_tx
                                .send(WorkerEvent::Failed(error.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
                WorkerCommand::StartLiveCapture => {
                    if live_running {
                        continue;
                    }

                    if session.is_none() {
                        if event_tx
                            .send(WorkerEvent::Failed(
                                "Connect to the device before starting live capture.".to_owned(),
                            ))
                            .is_err()
                        {
                            break;
                        }
                        continue;
                    }

                    live_running = true;
                    live_failures = 0;
                    if let Some(existing) = session.as_mut() {
                        if let Err(error) = existing.start_live_capture() {
                            live_running = false;
                            if event_tx
                                .send(WorkerEvent::Failed(error.to_string()))
                                .is_err()
                            {
                                break;
                            }
                            continue;
                        }
                    }
                    if event_tx.send(WorkerEvent::LiveStarted).is_err() {
                        break;
                    }
                }
                WorkerCommand::StopLiveCapture => {
                    if !live_running {
                        continue;
                    }

                    live_running = false;
                    live_failures = 0;
                    if let Some(existing) = session.as_mut() {
                        existing.stop_live_capture();
                    }
                    if event_tx.send(WorkerEvent::LiveStopped).is_err() {
                        break;
                    }
                }
                WorkerCommand::RefreshSavedCaptures => match scan_saved_captures() {
                    Ok(captures) => {
                        if event_tx
                            .send(WorkerEvent::SavedCapturesScanned(captures))
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        if event_tx
                            .send(WorkerEvent::Failed(error.to_string()))
                            .is_err()
                        {
                            break;
                        }
                    }
                },
                WorkerCommand::LoadSavedCapture(meta_path) => {
                    match load_saved_capture(&meta_path) {
                        Ok(LoadedSavedCapture {
                            entry,
                            note,
                            result,
                        }) => {
                            if event_tx
                                .send(WorkerEvent::SavedCaptureLoaded {
                                    entry,
                                    note,
                                    result,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(error) => {
                            if event_tx
                                .send(WorkerEvent::Failed(error.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
                WorkerCommand::RunAnalysis { capture, request } => {
                    match analyze_capture(&capture.capture, &request) {
                        Ok(result) => {
                            if event_tx
                                .send(WorkerEvent::AnalysisReady { result })
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(error) => {
                            if event_tx
                                .send(WorkerEvent::Failed(error.to_string()))
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
                WorkerCommand::Quit => break,
            }

            continue;
        }

        let Some(existing) = session.as_mut() else {
            live_running = false;
            if event_tx
                .send(WorkerEvent::Failed(
                    "Live capture lost its device session.".to_owned(),
                ))
                .is_err()
            {
                break;
            }
            continue;
        };

        match existing.capture_live_frame() {
            Ok(result) => {
                live_failures = 0;
                if event_tx
                    .send(WorkerEvent::LiveCaptureReady(result))
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                live_failures += 1;
                let error = error.to_string();

                if live_failures < LIVE_CAPTURE_MAX_CONSECUTIVE_FAILURES {
                    let rearm_error = existing
                        .retry_live_capture()
                        .err()
                        .map(|err| err.to_string());
                    if event_tx
                        .send(WorkerEvent::LiveRetrying {
                            error: match rearm_error {
                                Some(rearm_error) => {
                                    format!("{error}; live rearm failed: {rearm_error}")
                                }
                                None => error,
                            },
                            consecutive_failures: live_failures,
                        })
                        .is_err()
                    {
                        break;
                    }
                    thread::sleep(LIVE_CAPTURE_RETRY_BACKOFF);
                    continue;
                }

                live_running = false;
                live_failures = 0;
                existing.stop_live_capture();
                if event_tx
                    .send(WorkerEvent::Failed(format!(
                        "Live capture stopped after {LIVE_CAPTURE_MAX_CONSECUTIVE_FAILURES} consecutive errors. Last error: {error}"
                    )))
                    .is_err()
                {
                    break;
                }
                continue;
            }
        }

        thread::sleep(LIVE_CAPTURE_THROTTLE);
    }
}

fn format_device_summary(session: &DeviceSession) -> String {
    let info = session.info();
    let snapshot = session.transport_snapshot();
    let selected_firmware = info
        .selected_firmware
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "none selected in this session".to_owned());
    let sysfs_name = snapshot.sysfs_name.as_deref().unwrap_or("?");
    let interface_name = snapshot.interface_sysfs_name.as_deref().unwrap_or("?");
    let active_driver = snapshot.active_driver().unwrap_or("none");
    let product = snapshot.product.as_deref().unwrap_or("?");
    let manufacturer = snapshot.manufacturer.as_deref().unwrap_or("?");
    let serial = snapshot.serial.as_deref().unwrap_or(&info.flash.serial);

    format!(
        "Machine: {}\n\
         Product: {product}\n\
         Manufacturer: {manufacturer}\n\
         Serial: {serial}\n\
         Flash header: 0x{:04x}\n\
         Flash version: {}\n\
         Device version: {}\n\
         OEM: {}\n\
         Non-zero locale bytes: {}\n\
         Inferred FPGA version: {}\n\
         FPGA before: {}\n\
         FPGA after: {}\n\
         Firmware loaded this session: {}\n\
         Selected firmware: {}\n\
         Bus/device: {:03}/{:03}\n\
         USB path: {}\n\
         Sysfs device: {}\n\
         Sysfs interface: {}\n\
         Active driver at scan: {}\n\
         Layout matches OWON bulk shape: {}",
        info.machine,
        info.flash.header,
        info.flash.version,
        info.flash.device_version,
        info.flash.oem,
        info.flash.nonzero_locale_bytes,
        info.flash.inferred_vfpga,
        info.fpga_state_before,
        info.fpga_state_after,
        yes_no(info.firmware_loaded_this_session),
        truncate_home(&selected_firmware),
        snapshot.bus_number,
        snapshot.address,
        snapshot.bus_device_path(),
        sysfs_name,
        interface_name,
        active_driver,
        yes_no(snapshot.matches_expected_layout()),
    )
}

fn truncate_home(path: &str) -> String {
    let home = std::env::var("HOME").ok();
    if let Some(home) = home.as_deref() {
        if let Ok(stripped) = Path::new(path).strip_prefix(home) {
            return format!("~{}", stripped.display());
        }
    }
    path.to_owned()
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
