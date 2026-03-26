use std::cmp::Ordering;
use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use owon_decode::decode_uart_8n1;
use owon_device::NormalizedCapture;
use owon_protocol::ChannelId;

pub const DEFAULT_HISTOGRAM_BIN_COUNT: usize = 32;
pub const DEFAULT_SPECTRUM_BIN_COUNT: usize = 256;
pub const DEFAULT_SPECTROGRAM_WINDOW: usize = 256;
pub const DEFAULT_SPECTROGRAM_STEP: usize = 64;
pub const DEFAULT_UART_BAUD: f64 = 115_200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisSelection {
    pub start: usize,
    pub end: usize,
}

impl AnalysisSelection {
    pub fn full(sample_count: usize) -> Self {
        let end = sample_count.saturating_sub(1);
        Self { start: 0, end }
    }

    pub fn clamp(self, sample_count: usize) -> Self {
        if sample_count == 0 {
            return Self { start: 0, end: 0 };
        }

        let max_index = sample_count - 1;
        let mut start = self.start.min(max_index);
        let mut end = self.end.min(max_index);
        if end < start {
            std::mem::swap(&mut start, &mut end);
        }

        if start == end && end < max_index {
            end += 1;
        }

        Self { start, end }
    }

    pub fn len(self) -> usize {
        if self.end < self.start {
            0
        } else {
            self.end - self.start + 1
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisRequest {
    pub channel: ChannelId,
    pub selection: AnalysisSelection,
    pub histogram_bins: usize,
    pub spectrum_bins: usize,
    pub spectrogram_window: usize,
    pub spectrogram_step: usize,
    pub uart_baud: f64,
    pub uart_invert: bool,
}

impl AnalysisRequest {
    pub fn default_for_capture(capture: &NormalizedCapture, channel: ChannelId) -> Self {
        let sample_count = capture
            .channel(channel)
            .or_else(|| capture.channels.first())
            .map(|capture| capture.samples.len())
            .unwrap_or(0);

        Self {
            channel,
            selection: AnalysisSelection::full(sample_count),
            histogram_bins: DEFAULT_HISTOGRAM_BIN_COUNT,
            spectrum_bins: DEFAULT_SPECTRUM_BIN_COUNT,
            spectrogram_window: DEFAULT_SPECTROGRAM_WINDOW,
            spectrogram_step: DEFAULT_SPECTROGRAM_STEP,
            uart_baud: DEFAULT_UART_BAUD,
            uart_invert: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisError {
    MissingChannel(ChannelId),
    EmptySelection,
    InvalidBaud(f64),
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingChannel(channel) => write!(f, "capture does not contain {channel}"),
            Self::EmptySelection => {
                write!(f, "analysis selection must contain at least two samples")
            }
            Self::InvalidBaud(baud) => write!(f, "invalid UART baud for analysis: {baud}"),
        }
    }
}

impl Error for AnalysisError {}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisReport {
    pub channel: ChannelId,
    pub selection: AnalysisSelection,
    pub sample_rate_hz: f64,
    pub raw_samples: Vec<u8>,
    pub trigger_index_in_selection: Option<usize>,
    pub stats: SignalStats,
    pub histogram: Histogram,
    pub spectrum: Spectrum,
    pub spectrogram: Spectrogram,
    pub digital: DigitalAnalysis,
    pub uart: UartDecodeReport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SignalStats {
    pub sample_count: usize,
    pub min_u8: u8,
    pub max_u8: u8,
    pub mean_u8: f64,
    pub rms_centered: f64,
    pub stddev_u8: f64,
    pub peak_to_peak_u8: u8,
    pub dc_offset_from_midscale: f64,
    pub zero_crossings: usize,
    pub estimated_frequency_hz: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    pub bins: Vec<HistogramBin>,
    pub max_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistogramBin {
    pub start_u8: u8,
    pub end_u8: u8,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spectrum {
    pub points: Vec<SpectrumPoint>,
    pub peaks: Vec<SpectrumPeak>,
    pub nyquist_hz: f64,
    pub window_name: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumPoint {
    pub frequency_hz: f64,
    pub magnitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpectrumPeak {
    pub frequency_hz: f64,
    pub magnitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spectrogram {
    pub slice_count: usize,
    pub bin_count: usize,
    pub window_size: usize,
    pub step_size: usize,
    pub bin_hz: f64,
    pub magnitudes: Vec<f32>,
    pub max_magnitude: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    Rising,
    Falling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitalTransition {
    pub sample_index: usize,
    pub direction: EdgeDirection,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DigitalAnalysis {
    pub threshold_u8: u8,
    pub high_sample_count: usize,
    pub low_sample_count: usize,
    pub rising_edges: usize,
    pub falling_edges: usize,
    pub mean_high_samples: Option<f64>,
    pub mean_low_samples: Option<f64>,
    pub estimated_period_samples: Option<f64>,
    pub estimated_frequency_hz: Option<f64>,
    pub quality_note: Option<String>,
    pub first_transitions: Vec<DigitalTransition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UartDecodeReport {
    pub baud: f64,
    pub invert: bool,
    pub threshold_u8: u8,
    pub frame_count: usize,
    pub good_stop_count: usize,
    pub bad_stop_count: usize,
    pub ascii_rendered: String,
    pub hex_rendered: String,
    pub frames: Vec<UartFrameSummary>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UartFrameSummary {
    pub start_sample: usize,
    pub data: u8,
    pub stop_ok: bool,
    pub ascii: Option<char>,
}

pub fn analyze_capture(
    capture: &NormalizedCapture,
    request: &AnalysisRequest,
) -> Result<AnalysisReport, AnalysisError> {
    if !request.uart_baud.is_finite() || request.uart_baud <= 0.0 {
        return Err(AnalysisError::InvalidBaud(request.uart_baud));
    }

    let channel_capture = capture
        .channel(request.channel)
        .ok_or(AnalysisError::MissingChannel(request.channel))?;
    let selection = request.selection.clamp(channel_capture.samples.len());
    if selection.len() < 2 {
        return Err(AnalysisError::EmptySelection);
    }

    let raw_samples = channel_capture.samples[selection.start..=selection.end].to_vec();
    let threshold_u8 = midpoint_threshold(&raw_samples);
    let digital_samples = raw_samples
        .iter()
        .map(|sample| if *sample >= threshold_u8 { 1u8 } else { 0u8 })
        .collect::<Vec<_>>();

    let stats = compute_signal_stats(&raw_samples, capture.sample_rate_hz);
    let histogram = compute_histogram(&raw_samples, request.histogram_bins.max(8));
    let spectrum = compute_spectrum(&raw_samples, capture.sample_rate_hz, request.spectrum_bins);
    let spectrogram = compute_spectrogram(
        &raw_samples,
        capture.sample_rate_hz,
        request.spectrogram_window,
        request.spectrogram_step,
    );
    let digital = compute_digital_analysis(
        &raw_samples,
        &digital_samples,
        threshold_u8,
        capture.sample_rate_hz,
    );
    let uart = compute_uart_analysis(
        &digital_samples,
        capture.sample_rate_hz,
        request.uart_baud,
        request.uart_invert,
        threshold_u8,
    );
    let trigger_index_in_selection = channel_capture
        .trigger_index
        .filter(|index| *index >= selection.start && *index <= selection.end)
        .map(|index| index - selection.start);

    Ok(AnalysisReport {
        channel: request.channel,
        selection,
        sample_rate_hz: capture.sample_rate_hz,
        raw_samples,
        trigger_index_in_selection,
        stats,
        histogram,
        spectrum,
        spectrogram,
        digital,
        uart,
    })
}

fn compute_signal_stats(samples: &[u8], sample_rate_hz: f64) -> SignalStats {
    let sample_count = samples.len();
    let min_u8 = samples.iter().copied().min().unwrap_or(0);
    let max_u8 = samples.iter().copied().max().unwrap_or(0);
    let mean_u8 =
        samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / sample_count as f64;

    let centered = samples
        .iter()
        .map(|sample| f64::from(*sample) - 127.5)
        .collect::<Vec<_>>();
    let rms_centered =
        (centered.iter().map(|value| value * value).sum::<f64>() / sample_count as f64).sqrt();
    let stddev_u8 = (samples
        .iter()
        .map(|sample| {
            let delta = f64::from(*sample) - mean_u8;
            delta * delta
        })
        .sum::<f64>()
        / sample_count as f64)
        .sqrt();
    let zero_crossings = centered
        .windows(2)
        .filter(|window| {
            (window[0] < 0.0 && window[1] >= 0.0) || (window[0] >= 0.0 && window[1] < 0.0)
        })
        .count();
    let estimated_frequency_hz = if zero_crossings >= 2 && sample_count > 1 {
        let duration_seconds = sample_count as f64 / sample_rate_hz;
        if duration_seconds > 0.0 {
            Some(zero_crossings as f64 / 2.0 / duration_seconds)
        } else {
            None
        }
    } else {
        None
    };

    SignalStats {
        sample_count,
        min_u8,
        max_u8,
        mean_u8,
        rms_centered,
        stddev_u8,
        peak_to_peak_u8: max_u8.saturating_sub(min_u8),
        dc_offset_from_midscale: mean_u8 - 127.5,
        zero_crossings,
        estimated_frequency_hz,
    }
}

fn compute_histogram(samples: &[u8], bin_count: usize) -> Histogram {
    let bin_count = bin_count.clamp(8, 128);
    let width = 256.0 / bin_count as f64;
    let mut counts = vec![0usize; bin_count];
    for sample in samples {
        let mut index = (f64::from(*sample) / width).floor() as usize;
        if index >= bin_count {
            index = bin_count - 1;
        }
        counts[index] += 1;
    }

    let bins = counts
        .into_iter()
        .enumerate()
        .map(|(index, count)| {
            let start = (index as f64 * width).floor().clamp(0.0, 255.0) as u8;
            let end = (((index + 1) as f64 * width).ceil() as i32 - 1).clamp(0, 255) as u8;
            HistogramBin {
                start_u8: start,
                end_u8: end,
                count,
            }
        })
        .collect::<Vec<_>>();

    let max_count = bins.iter().map(|bin| bin.count).max().unwrap_or(0);
    Histogram { bins, max_count }
}

fn compute_spectrum(samples: &[u8], sample_rate_hz: f64, requested_bins: usize) -> Spectrum {
    let requested_bins = requested_bins.clamp(16, 512);
    let centered = windowed_centered_samples(samples);
    let sample_count = centered.len();
    let available_bins = (sample_count / 2).max(1);
    let bin_count = requested_bins.min(available_bins);
    let bin_hz = sample_rate_hz / sample_count as f64;

    let mut points = Vec::with_capacity(bin_count);
    for bin in 0..bin_count {
        let magnitude = dft_bin_magnitude(&centered, bin);
        points.push(SpectrumPoint {
            frequency_hz: bin as f64 * bin_hz,
            magnitude,
        });
    }

    let mut peak_candidates = points
        .iter()
        .skip(1)
        .filter(|point| point.magnitude.is_finite())
        .cloned()
        .collect::<Vec<_>>();
    peak_candidates.sort_by(|left, right| {
        right
            .magnitude
            .partial_cmp(&left.magnitude)
            .unwrap_or(Ordering::Equal)
    });
    peak_candidates.truncate(5);

    Spectrum {
        points,
        peaks: peak_candidates
            .into_iter()
            .map(|point| SpectrumPeak {
                frequency_hz: point.frequency_hz,
                magnitude: point.magnitude,
            })
            .collect(),
        nyquist_hz: sample_rate_hz / 2.0,
        window_name: "Hann",
    }
}

fn compute_spectrogram(
    samples: &[u8],
    sample_rate_hz: f64,
    requested_window: usize,
    requested_step: usize,
) -> Spectrogram {
    if samples.is_empty() {
        return Spectrogram {
            slice_count: 0,
            bin_count: 0,
            window_size: 0,
            step_size: 0,
            bin_hz: 0.0,
            magnitudes: Vec::new(),
            max_magnitude: 0.0,
        };
    }

    let window_size = requested_window.clamp(64, samples.len().max(64));
    let step_size = requested_step.clamp(16, window_size.max(16));
    let bin_count = (window_size / 4).clamp(16, 128);
    let mut magnitudes = Vec::new();
    let mut slice_count = 0usize;
    let mut max_magnitude = 0.0f32;
    let bin_hz = sample_rate_hz / window_size as f64;

    if samples.len() <= window_size {
        let window = windowed_centered_samples(samples);
        for bin in 0..bin_count {
            let magnitude = dft_bin_magnitude(&window, bin) as f32;
            max_magnitude = max_magnitude.max(magnitude);
            magnitudes.push(magnitude);
        }
        slice_count = 1;
    } else {
        let mut start = 0usize;
        while start + window_size <= samples.len() {
            let window = windowed_centered_samples(&samples[start..start + window_size]);
            for bin in 0..bin_count {
                let magnitude = dft_bin_magnitude(&window, bin) as f32;
                max_magnitude = max_magnitude.max(magnitude);
                magnitudes.push(magnitude);
            }
            slice_count += 1;
            start += step_size;
        }
    }

    Spectrogram {
        slice_count,
        bin_count,
        window_size,
        step_size,
        bin_hz,
        magnitudes,
        max_magnitude,
    }
}

fn compute_digital_analysis(
    raw_samples: &[u8],
    digital_samples: &[u8],
    threshold_u8: u8,
    sample_rate_hz: f64,
) -> DigitalAnalysis {
    let mut transitions = Vec::new();
    let mut high_runs = Vec::new();
    let mut low_runs = Vec::new();
    let mut current_level = digital_samples.first().copied().unwrap_or(0);
    let mut run_start = 0usize;

    for (index, window) in digital_samples.windows(2).enumerate() {
        if window[0] == window[1] {
            continue;
        }

        let next_level = window[1];
        let direction = if next_level > window[0] {
            EdgeDirection::Rising
        } else {
            EdgeDirection::Falling
        };
        transitions.push(DigitalTransition {
            sample_index: index + 1,
            direction,
        });

        let run_len = (index + 1) - run_start;
        if current_level == 0 {
            low_runs.push(run_len);
        } else {
            high_runs.push(run_len);
        }
        current_level = next_level;
        run_start = index + 1;
    }

    let trailing_run = digital_samples.len().saturating_sub(run_start);
    if trailing_run > 0 {
        if current_level == 0 {
            low_runs.push(trailing_run);
        } else {
            high_runs.push(trailing_run);
        }
    }

    let contrast = raw_samples
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .saturating_sub(raw_samples.iter().copied().min().unwrap_or(0));
    let quality_note = if contrast < 12 {
        Some("Low contrast selection; digital thresholding may be unreliable.".to_owned())
    } else {
        None
    };

    let mean_high_samples = mean_usize(&high_runs);
    let mean_low_samples = mean_usize(&low_runs);
    let estimated_period_samples = match (mean_high_samples, mean_low_samples) {
        (Some(high), Some(low)) => Some(high + low),
        _ => None,
    };
    let estimated_frequency_hz = estimated_period_samples
        .filter(|period| *period > 0.0)
        .map(|period| sample_rate_hz / period);

    DigitalAnalysis {
        threshold_u8,
        high_sample_count: digital_samples
            .iter()
            .filter(|sample| **sample != 0)
            .count(),
        low_sample_count: digital_samples
            .iter()
            .filter(|sample| **sample == 0)
            .count(),
        rising_edges: transitions
            .iter()
            .filter(|transition| transition.direction == EdgeDirection::Rising)
            .count(),
        falling_edges: transitions
            .iter()
            .filter(|transition| transition.direction == EdgeDirection::Falling)
            .count(),
        mean_high_samples,
        mean_low_samples,
        estimated_period_samples,
        estimated_frequency_hz,
        quality_note,
        first_transitions: transitions.into_iter().take(24).collect(),
    }
}

fn compute_uart_analysis(
    digital_samples: &[u8],
    sample_rate_hz: f64,
    baud: f64,
    invert: bool,
    threshold_u8: u8,
) -> UartDecodeReport {
    match decode_uart_8n1(digital_samples, sample_rate_hz, baud, invert) {
        Ok(frames) => {
            let good_stop_count = frames.iter().filter(|frame| frame.stop_ok).count();
            let bad_stop_count = frames.len().saturating_sub(good_stop_count);
            let ascii_rendered = frames
                .iter()
                .map(|frame| frame.ascii.unwrap_or('.'))
                .collect::<String>();
            let hex_rendered = frames
                .iter()
                .map(|frame| format!("{:02x}", frame.data))
                .collect::<Vec<_>>()
                .join(" ");
            let note = if frames.is_empty() {
                Some("No UART frames decoded from the selected signal.".to_owned())
            } else if good_stop_count == 0 {
                Some(
                    "Every decoded frame failed the stop-bit check; polarity, threshold, or baud likely need adjustment."
                        .to_owned(),
                )
            } else {
                None
            };

            UartDecodeReport {
                baud,
                invert,
                threshold_u8,
                frame_count: frames.len(),
                good_stop_count,
                bad_stop_count,
                ascii_rendered,
                hex_rendered,
                frames: frames
                    .into_iter()
                    .take(64)
                    .map(|frame| UartFrameSummary {
                        start_sample: frame.start_sample,
                        data: frame.data,
                        stop_ok: frame.stop_ok,
                        ascii: frame.ascii,
                    })
                    .collect(),
                note,
            }
        }
        Err(error) => UartDecodeReport {
            baud,
            invert,
            threshold_u8,
            frame_count: 0,
            good_stop_count: 0,
            bad_stop_count: 0,
            ascii_rendered: String::new(),
            hex_rendered: String::new(),
            frames: Vec::new(),
            note: Some(format!("UART decode failed: {error}")),
        },
    }
}

fn midpoint_threshold(samples: &[u8]) -> u8 {
    let min = samples.iter().copied().min().unwrap_or(0);
    let max = samples.iter().copied().max().unwrap_or(0);
    (((u16::from(min) + u16::from(max)) / 2) & 0xff) as u8
}

fn mean_usize(values: &[usize]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<usize>() as f64 / values.len() as f64)
    }
}

fn windowed_centered_samples(samples: &[u8]) -> Vec<f64> {
    let sample_count = samples.len().max(1);
    let mean = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>() / sample_count as f64;
    samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let hann = if sample_count <= 1 {
                1.0
            } else {
                0.5 - 0.5 * (2.0 * PI * index as f64 / (sample_count as f64 - 1.0)).cos()
            };
            (f64::from(*sample) - mean) * hann
        })
        .collect()
}

fn dft_bin_magnitude(samples: &[f64], bin: usize) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    let n = samples.len() as f64;
    let mut re = 0.0;
    let mut im = 0.0;
    for (index, sample) in samples.iter().enumerate() {
        let angle = 2.0 * PI * bin as f64 * index as f64 / n;
        re += sample * angle.cos();
        im -= sample * angle.sin();
    }
    (re.hypot(im)) / n
}

#[cfg(test)]
mod tests {
    use super::*;
    use owon_device::ChannelCapture;

    #[test]
    fn analyzes_square_wave_edges_and_frequency() {
        let samples = generate_square_wave(1_000.0, 100_000.0, 2_000);
        let capture = capture_for_samples(ChannelId::Ch1, samples, 100_000.0);
        let request = AnalysisRequest::default_for_capture(&capture, ChannelId::Ch1);
        let report = analyze_capture(&capture, &request).expect("analysis should succeed");

        assert!(report.digital.rising_edges > 10);
        assert!(report.digital.falling_edges > 10);
        let frequency = report
            .digital
            .estimated_frequency_hz
            .expect("digital estimate should exist");
        assert!((frequency - 1_000.0).abs() < 150.0);
        assert!(!report.spectrum.points.is_empty());
    }

    #[test]
    fn spectrum_finds_sine_peak() {
        let samples = generate_sine_wave(2_500.0, 50_000.0, 2_048);
        let capture = capture_for_samples(ChannelId::Ch1, samples, 50_000.0);
        let request = AnalysisRequest::default_for_capture(&capture, ChannelId::Ch1);
        let report = analyze_capture(&capture, &request).expect("analysis should succeed");

        let peak = report.spectrum.peaks.first().expect("peak should exist");
        assert!((peak.frequency_hz - 2_500.0).abs() < 400.0);
    }

    #[test]
    fn uart_report_decodes_simple_frame() {
        let samples = generate_uart_samples(0x41, 115_200.0, 2_500_000.0);
        let capture = capture_for_samples(ChannelId::Ch1, samples, 2_500_000.0);
        let request = AnalysisRequest::default_for_capture(&capture, ChannelId::Ch1);
        let report = analyze_capture(&capture, &request).expect("analysis should succeed");

        assert!(report.uart.frame_count >= 1);
        assert!(report.uart.hex_rendered.contains("41"));
    }

    fn capture_for_samples(
        channel: ChannelId,
        samples: Vec<u8>,
        sample_rate_hz: f64,
    ) -> NormalizedCapture {
        NormalizedCapture {
            sample_rate_hz,
            sample_period_ns: 1_000_000_000.0 / sample_rate_hz,
            channels: vec![ChannelCapture {
                channel,
                samples,
                trigger_index: Some(0),
                frequency_hz: None,
            }],
        }
    }

    fn generate_square_wave(
        frequency_hz: f64,
        sample_rate_hz: f64,
        sample_count: usize,
    ) -> Vec<u8> {
        (0..sample_count)
            .map(|index| {
                let phase = (index as f64 * frequency_hz / sample_rate_hz) % 1.0;
                if phase < 0.5 { 240 } else { 16 }
            })
            .collect()
    }

    fn generate_sine_wave(frequency_hz: f64, sample_rate_hz: f64, sample_count: usize) -> Vec<u8> {
        (0..sample_count)
            .map(|index| {
                let angle = 2.0 * PI * frequency_hz * index as f64 / sample_rate_hz;
                let sample = 127.5 + (angle.sin() * 110.0);
                sample.round().clamp(0.0, 255.0) as u8
            })
            .collect()
    }

    fn generate_uart_samples(byte: u8, baud: f64, sample_rate_hz: f64) -> Vec<u8> {
        let samples_per_bit = (sample_rate_hz / baud).round() as usize;
        let mut bits = Vec::new();
        bits.extend(std::iter::repeat(1u8).take(samples_per_bit * 3));
        bits.extend(std::iter::repeat(0u8).take(samples_per_bit));
        for bit_index in 0..8 {
            let bit = if (byte >> bit_index) & 1 == 0 {
                0u8
            } else {
                1u8
            };
            bits.extend(std::iter::repeat(bit).take(samples_per_bit));
        }
        bits.extend(std::iter::repeat(1u8).take(samples_per_bit * 2));
        bits.into_iter()
            .map(|bit| if bit == 0 { 16 } else { 240 })
            .collect()
    }
}
