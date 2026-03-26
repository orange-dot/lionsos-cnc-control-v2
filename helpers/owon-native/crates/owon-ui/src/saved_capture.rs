use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use owon_device::{
    CaptureDebugInfo, CaptureOnceResult, ChannelCalibrationDebugInfo, ChannelCapture,
    ChannelCaptureDebugInfo, NormalizedCapture,
};
use owon_protocol::ChannelId;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedCaptureEntry {
    pub label: String,
    pub meta_path: PathBuf,
}

impl SavedCaptureEntry {
    pub fn id(&self) -> String {
        self.meta_path.display().to_string()
    }
}

#[derive(Debug)]
pub struct LoadedSavedCapture {
    pub entry: SavedCaptureEntry,
    pub note: Option<String>,
    pub result: CaptureOnceResult,
}

#[derive(Debug, Deserialize)]
struct CaptureMetadataFile {
    sample_rate_hz: f64,
    sample_period_ns: f64,
    channels: Vec<CaptureMetadataChannel>,
}

#[derive(Debug, Deserialize)]
struct CaptureMetadataChannel {
    channel: String,
    samples_file: String,
    sample_count: usize,
    trigger_index: Option<usize>,
    frequency_hz: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CaptureDebugFile {
    #[serde(default)]
    requested_channels: Vec<String>,
    #[serde(default)]
    volt_range_index: usize,
    #[serde(default)]
    timebase_prescaler: u32,
    #[serde(default)]
    rollmode: bool,
    #[serde(default)]
    peakmode: bool,
    #[serde(default)]
    pre_trigger_samples: u16,
    #[serde(default)]
    post_trigger_samples: u32,
    #[serde(default)]
    trigger_holdoff: u16,
    #[serde(default)]
    edge_level: u16,
    #[serde(default)]
    freqref: u8,
    #[serde(default)]
    phasefine: u16,
    #[serde(default)]
    calibrations: Vec<CaptureCalibrationDebugFile>,
    #[serde(default)]
    channels: Vec<CaptureChannelDebugFile>,
}

#[derive(Debug, Deserialize)]
struct CaptureCalibrationDebugFile {
    channel: String,
    zero_offset: u16,
    voltage_gain: u16,
}

#[derive(Debug, Deserialize)]
struct CaptureChannelDebugFile {
    channel: String,
    cursor_from_right: u16,
    time_sum: u32,
    period_num: u32,
    sample_offset: usize,
    sample_count: usize,
    sample_min_u8: u8,
    sample_max_u8: u8,
    sample_min_i8: i8,
    sample_max_i8: i8,
}

pub fn scan_saved_captures() -> Result<Vec<SavedCaptureEntry>, io::Error> {
    let mut captures = Vec::new();
    for entry in fs::read_dir(native_root()?)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with("-meta.json") {
            continue;
        }

        captures.push(SavedCaptureEntry {
            label: name.trim_end_matches("-meta.json").to_owned(),
            meta_path: path,
        });
    }

    captures.sort_by(|left, right| left.label.cmp(&right.label));
    Ok(captures)
}

pub fn load_saved_capture(
    meta_path: &Path,
) -> Result<LoadedSavedCapture, Box<dyn std::error::Error>> {
    let metadata_bytes = fs::read(meta_path)?;
    let metadata: CaptureMetadataFile = serde_json::from_slice(&metadata_bytes)?;
    let entry = SavedCaptureEntry {
        label: capture_label_from_path(meta_path),
        meta_path: meta_path.to_path_buf(),
    };

    let mut channels = Vec::with_capacity(metadata.channels.len());
    for channel in metadata.channels {
        let channel_id = parse_channel_name(&channel.channel).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported channel name in metadata: {}", channel.channel),
            )
        })?;
        let samples_path = resolve_samples_path(meta_path, &channel.samples_file);
        let samples = fs::read(&samples_path)?;
        if samples.len() != channel.sample_count {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "sample count mismatch for {}: metadata says {}, file has {} bytes",
                    samples_path.display(),
                    channel.sample_count,
                    samples.len()
                ),
            )
            .into());
        }

        channels.push(ChannelCapture {
            channel: channel_id,
            samples,
            trigger_index: channel.trigger_index,
            frequency_hz: channel.frequency_hz.and_then(nullable_frequency),
        });
    }

    channels.sort_by_key(|channel| channel.channel);
    let capture = NormalizedCapture {
        sample_rate_hz: metadata.sample_rate_hz,
        sample_period_ns: metadata.sample_period_ns,
        channels,
    };

    let debug_path = debug_path_from_meta(meta_path);
    let (debug, note) = if debug_path.exists() {
        let bytes = fs::read(&debug_path)?;
        let debug_file: CaptureDebugFile = serde_json::from_slice(&bytes)?;
        (debug_from_file(debug_file)?, None)
    } else {
        (
            synthesize_debug(&capture),
            Some(
                "Debug sidecar missing; debug values below were inferred from raw samples."
                    .to_owned(),
            ),
        )
    };

    Ok(LoadedSavedCapture {
        entry,
        note,
        result: CaptureOnceResult { capture, debug },
    })
}

fn native_root() -> Result<PathBuf, io::Error> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
}

fn capture_label_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.trim_end_matches("-meta.json").to_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn resolve_samples_path(meta_path: &Path, samples_file: &str) -> PathBuf {
    let samples_path = PathBuf::from(samples_file);
    if samples_path.is_absolute() {
        samples_path
    } else {
        meta_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(samples_path)
    }
}

fn debug_path_from_meta(meta_path: &Path) -> PathBuf {
    let name = meta_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let debug_name = name.replace("-meta.json", "-debug.json");
    meta_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(debug_name)
}

fn debug_from_file(file: CaptureDebugFile) -> Result<CaptureDebugInfo, io::Error> {
    let requested_channels = file
        .requested_channels
        .iter()
        .map(|channel| {
            parse_channel_name(channel).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unsupported debug channel: {channel}"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let calibrations = file
        .calibrations
        .into_iter()
        .map(|channel| {
            Ok(ChannelCalibrationDebugInfo {
                channel: parse_channel_name(&channel.channel).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unsupported calibration channel: {}", channel.channel),
                    )
                })?,
                zero_offset: channel.zero_offset,
                voltage_gain: channel.voltage_gain,
            })
        })
        .collect::<Result<Vec<_>, io::Error>>()?;
    let channels = file
        .channels
        .into_iter()
        .map(|channel| {
            Ok(ChannelCaptureDebugInfo {
                channel: parse_channel_name(&channel.channel).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unsupported capture debug channel: {}", channel.channel),
                    )
                })?,
                cursor_from_right: channel.cursor_from_right,
                time_sum: channel.time_sum,
                period_num: channel.period_num,
                sample_offset: channel.sample_offset,
                sample_count: channel.sample_count,
                sample_min_u8: channel.sample_min_u8,
                sample_max_u8: channel.sample_max_u8,
                sample_min_i8: channel.sample_min_i8,
                sample_max_i8: channel.sample_max_i8,
            })
        })
        .collect::<Result<Vec<_>, io::Error>>()?;

    Ok(CaptureDebugInfo {
        requested_channels,
        volt_range_index: file.volt_range_index,
        timebase_prescaler: file.timebase_prescaler,
        rollmode: file.rollmode,
        peakmode: file.peakmode,
        pre_trigger_samples: file.pre_trigger_samples,
        post_trigger_samples: file.post_trigger_samples,
        trigger_holdoff: file.trigger_holdoff,
        edge_level: file.edge_level,
        freqref: file.freqref,
        phasefine: file.phasefine,
        calibrations,
        channels,
    })
}

fn synthesize_debug(capture: &NormalizedCapture) -> CaptureDebugInfo {
    let channels = capture
        .channels
        .iter()
        .map(|channel| {
            let stats = byte_sample_stats(&channel.samples);
            ChannelCaptureDebugInfo {
                channel: channel.channel,
                cursor_from_right: channel
                    .trigger_index
                    .and_then(|value| u16::try_from(value).ok())
                    .unwrap_or(0),
                time_sum: 0,
                period_num: 0,
                sample_offset: 0,
                sample_count: channel.samples.len(),
                sample_min_u8: stats.min_u8,
                sample_max_u8: stats.max_u8,
                sample_min_i8: stats.min_i8,
                sample_max_i8: stats.max_i8,
            }
        })
        .collect::<Vec<_>>();

    CaptureDebugInfo {
        requested_channels: capture
            .channels
            .iter()
            .map(|channel| channel.channel)
            .collect(),
        volt_range_index: 0,
        timebase_prescaler: 0,
        rollmode: false,
        peakmode: false,
        pre_trigger_samples: 0,
        post_trigger_samples: 0,
        trigger_holdoff: 0,
        edge_level: 0,
        freqref: 0,
        phasefine: 0,
        calibrations: Vec::new(),
        channels,
    }
}

fn byte_sample_stats(samples: &[u8]) -> ByteSampleStats {
    ByteSampleStats {
        min_u8: samples.iter().copied().min().unwrap_or(0),
        max_u8: samples.iter().copied().max().unwrap_or(0),
        min_i8: samples
            .iter()
            .copied()
            .map(|sample| sample as i8)
            .min()
            .unwrap_or(0),
        max_i8: samples
            .iter()
            .copied()
            .map(|sample| sample as i8)
            .max()
            .unwrap_or(0),
    }
}

fn nullable_frequency(value: f64) -> Option<f64> {
    if value > 0.0 && value.is_finite() {
        Some(value)
    } else {
        None
    }
}

fn parse_channel_name(value: &str) -> Option<ChannelId> {
    match value.to_ascii_lowercase().as_str() {
        "ch1" => Some(ChannelId::Ch1),
        "ch2" => Some(ChannelId::Ch2),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteSampleStats {
    min_u8: u8,
    max_u8: u8,
    min_i8: i8,
    max_i8: i8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_saved_captures_finds_known_fixture_names() {
        let captures = scan_saved_captures().expect("scan should succeed");
        assert!(
            captures
                .iter()
                .any(|capture| capture.label == "twocap-clean")
        );
        assert!(
            captures
                .iter()
                .any(|capture| capture.label == "rpi-uart-boot")
        );
    }

    #[test]
    fn load_saved_capture_supports_missing_debug_sidecar() {
        let root = native_root().expect("native root");
        let loaded =
            load_saved_capture(&root.join("twocap-clean-meta.json")).expect("load saved capture");

        assert_eq!(loaded.entry.label, "twocap-clean");
        assert_eq!(loaded.result.capture.channels.len(), 2);
        assert!(loaded.note.is_some());
        assert_eq!(loaded.result.debug.channels.len(), 2);
    }

    #[test]
    fn load_saved_capture_uses_debug_sidecar_when_available() {
        let root = native_root().expect("native root");
        let loaded =
            load_saved_capture(&root.join("rpi-uart-boot-meta.json")).expect("load saved capture");

        assert_eq!(loaded.entry.label, "rpi-uart-boot");
        assert_eq!(loaded.result.capture.channels.len(), 1);
        assert_eq!(loaded.result.debug.channels.len(), 1);
        assert!(loaded.note.is_none());
    }
}
