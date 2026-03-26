//! Waveform analysis and protocol decoders above captured samples.
//!
//! The first implemented decoder is a small offline UART path that operates on
//! digital 0/1 sample streams. This keeps the transport and thresholding work
//! separate from the actual protocol decoding logic.

use std::error::Error;
use std::fmt;

const DATA_BITS: usize = 8;
const STOP_SAMPLE_OFFSET_BITS: f64 = 9.5;
const FIRST_DATA_SAMPLE_OFFSET_BITS: f64 = 1.5;
const FRAME_GUARD_BITS: f64 = 9.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UartFrame {
    pub start_sample: usize,
    pub data: u8,
    pub stop_ok: bool,
    pub ascii: Option<char>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    InvalidSampleRate(f64),
    InvalidBaudRate(f64),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSampleRate(value) => {
                write!(f, "Invalid sample rate for UART decode: {value}")
            }
            Self::InvalidBaudRate(value) => {
                write!(f, "Invalid baud rate for UART decode: {value}")
            }
        }
    }
}

impl Error for DecodeError {}

pub fn decode_uart_8n1(
    samples: &[u8],
    sample_rate_hz: f64,
    baud: f64,
    invert: bool,
) -> Result<Vec<UartFrame>, DecodeError> {
    if !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 {
        return Err(DecodeError::InvalidSampleRate(sample_rate_hz));
    }
    if !baud.is_finite() || baud <= 0.0 {
        return Err(DecodeError::InvalidBaudRate(baud));
    }

    let samples_per_bit = sample_rate_hz / baud;
    let frame_guard_samples = (samples_per_bit * FRAME_GUARD_BITS).ceil() as usize;
    let mut frames = Vec::new();
    let mut index = 1usize;

    while index < samples.len() {
        if is_high(samples[index - 1], invert) && !is_high(samples[index], invert) {
            let frame = sample_frame(samples, index, samples_per_bit, invert);
            match frame {
                Some(frame) => {
                    frames.push(frame);
                    index = index.saturating_add(frame_guard_samples);
                    continue;
                }
                None => break,
            }
        }
        index += 1;
    }

    Ok(frames)
}

fn sample_frame(
    samples: &[u8],
    start_sample: usize,
    samples_per_bit: f64,
    invert: bool,
) -> Option<UartFrame> {
    let stop_index = start_sample + (samples_per_bit * STOP_SAMPLE_OFFSET_BITS).round() as usize;
    if stop_index >= samples.len() {
        return None;
    }

    let mut data = 0u8;
    for bit_index in 0..DATA_BITS {
        let sample_index = start_sample
            + (samples_per_bit * (FIRST_DATA_SAMPLE_OFFSET_BITS + bit_index as f64)).round()
                as usize;
        if sample_index >= samples.len() {
            return None;
        }
        if is_high(samples[sample_index], invert) {
            data |= 1 << bit_index;
        }
    }

    let stop_ok = is_high(samples[stop_index], invert);
    Some(UartFrame {
        start_sample,
        data,
        stop_ok,
        ascii: printable_ascii(data),
    })
}

fn is_high(sample: u8, invert: bool) -> bool {
    let level = sample != 0;
    if invert { !level } else { level }
}

fn printable_ascii(byte: u8) -> Option<char> {
    match byte {
        0x20..=0x7e => Some(char::from(byte)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct FixtureSource {
        capture_file: String,
        capture_url: String,
        sigrok_chunk: String,
        bit_index: u8,
        sample_range: [usize; 2],
    }

    #[derive(Debug, Deserialize)]
    struct Fixture {
        fixture_name: String,
        sample_rate_hz: f64,
        baud: f64,
        data_bits: u8,
        parity: String,
        stop_bits: u8,
        invert: bool,
        channel_name: String,
        samples_file: String,
        expected_bytes_hex: String,
        expected_ascii_rendered: String,
        source: FixtureSource,
    }

    #[test]
    fn decodes_real_amulet_lcd_fixture() {
        let fixture = load_fixture();
        assert_eq!(fixture.fixture_name, "amulet_lcd_menu_changes_pin1_window");
        assert_eq!(fixture.baud, 115_200.0);
        assert_eq!(fixture.data_bits, 8);
        assert_eq!(fixture.parity, "none");
        assert_eq!(fixture.stop_bits, 1);
        assert!(!fixture.invert);
        assert_eq!(fixture.channel_name, "Pin 1");
        assert_eq!(fixture.source.capture_file, "menu_changes.sr");
        assert_eq!(
            fixture.source.capture_url,
            "https://github.com/sigrokproject/sigrok-dumps/blob/master/uart/amulet_lcd/menu_changes.sr"
        );
        assert_eq!(fixture.source.sigrok_chunk, "logic-1-3");
        assert_eq!(fixture.source.bit_index, 0);
        assert_eq!(fixture.source.sample_range, [3_044_396, 3_068_915]);

        let samples = load_samples(&fixture.samples_file);
        let frames = decode_uart_8n1(
            &samples,
            fixture.sample_rate_hz,
            fixture.baud,
            fixture.invert,
        )
        .expect("fixture decode should succeed");

        assert!(frames.iter().all(|frame| frame.stop_ok));
        assert_eq!(bytes(&frames), parse_hex_bytes(&fixture.expected_bytes_hex));
        assert_eq!(render_ascii(&frames), fixture.expected_ascii_rendered);
    }

    #[test]
    fn decodes_inverted_fixture_when_requested() {
        let fixture = load_fixture();
        let samples = load_samples(&fixture.samples_file)
            .into_iter()
            .map(|sample| if sample == 0 { 1 } else { 0 })
            .collect::<Vec<_>>();

        let frames = decode_uart_8n1(&samples, fixture.sample_rate_hz, fixture.baud, true)
            .expect("inverted fixture decode should succeed");

        assert_eq!(bytes(&frames), parse_hex_bytes(&fixture.expected_bytes_hex));
    }

    #[test]
    fn rejects_invalid_rates() {
        assert_eq!(
            decode_uart_8n1(&[1, 0, 1], 0.0, 115_200.0, false).unwrap_err(),
            DecodeError::InvalidSampleRate(0.0)
        );
        assert_eq!(
            decode_uart_8n1(&[1, 0, 1], 10_000_000.0, 0.0, false).unwrap_err(),
            DecodeError::InvalidBaudRate(0.0)
        );
    }

    fn bytes(frames: &[UartFrame]) -> Vec<u8> {
        frames.iter().map(|frame| frame.data).collect()
    }

    fn render_ascii(frames: &[UartFrame]) -> String {
        frames
            .iter()
            .map(|frame| frame.ascii.unwrap_or('.'))
            .collect::<String>()
    }

    fn parse_hex_bytes(value: &str) -> Vec<u8> {
        value
            .split_whitespace()
            .map(|hex| u8::from_str_radix(hex, 16).expect("fixture hex byte should parse"))
            .collect()
    }

    fn load_fixture() -> Fixture {
        let path = fixture_root().join("fixture.json");
        let bytes = fs::read(path).expect("fixture json should exist");
        serde_json::from_slice(&bytes).expect("fixture json should parse")
    }

    fn load_samples(samples_file: &str) -> Vec<u8> {
        fs::read(fixture_root().join(samples_file)).expect("fixture samples should exist")
    }

    fn fixture_root() -> &'static Path {
        Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../testdata/uart/amulet_lcd_menu_changes"
        ))
    }
}
