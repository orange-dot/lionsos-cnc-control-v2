use owon_device::NormalizedCapture;

const BACKGROUND: (f64, f64, f64) = (0.06, 0.08, 0.11);
const LANE_BACKGROUND: (f64, f64, f64) = (0.11, 0.14, 0.18);
const GRID: (f64, f64, f64) = (0.24, 0.28, 0.33);
const CH1: (f64, f64, f64) = (0.98, 0.74, 0.24);
const CH2: (f64, f64, f64) = (0.28, 0.85, 0.73);
const TRIGGER: (f64, f64, f64) = (0.92, 0.40, 0.32);
const LANE_PADDING: f64 = 6.0;
const MIN_VISIBLE_ENVELOPE_HEIGHT: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct EnvelopeColumn {
    x: f64,
    min_y: f64,
    max_y: f64,
}

pub fn draw_capture(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    capture: Option<&NormalizedCapture>,
) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;

    set_source(cr, BACKGROUND);
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();

    let Some(capture) = capture else {
        return;
    };
    if capture.channels.is_empty() {
        return;
    }

    let lane_count = capture.channels.len().max(1) as f64;
    let lane_height = height / lane_count;

    for (index, channel) in capture.channels.iter().enumerate() {
        let top = lane_height * index as f64;
        let color = if index == 0 { CH1 } else { CH2 };
        draw_lane_background(cr, width, top, lane_height);
        draw_waveform_envelope(cr, width, top, lane_height, &channel.samples, color);
    }

    if let Some(trigger_index) = capture
        .channels
        .iter()
        .find_map(|channel| channel.trigger_index)
    {
        let sample_count = capture
            .channels
            .first()
            .map(|channel| channel.samples.len())
            .unwrap_or(0);
        if sample_count > 1 {
            let x = trigger_index as f64 / (sample_count - 1) as f64 * (width - 1.0);
            set_source(cr, TRIGGER);
            cr.set_line_width(1.0);
            cr.move_to(x, 0.0);
            cr.line_to(x, height);
            let _ = cr.stroke();
        }
    }
}

fn build_envelope_columns(
    samples: &[u8],
    width: f64,
    lane_top: f64,
    lane_height: f64,
) -> Vec<EnvelopeColumn> {
    if samples.is_empty() || width <= 1.0 || lane_height <= 1.0 {
        return Vec::new();
    }

    let column_count = width.round().max(1.0) as usize;
    let last_sample_index = samples.len().saturating_sub(1);
    let last_column_index = column_count.saturating_sub(1);
    let mut buckets = vec![None::<(u8, u8)>; column_count];

    for (sample_index, sample) in samples.iter().copied().enumerate() {
        let column = if last_sample_index == 0 {
            0
        } else {
            sample_index * last_column_index / last_sample_index
        };
        match &mut buckets[column] {
            Some((min_sample, max_sample)) => {
                *min_sample = (*min_sample).min(sample);
                *max_sample = (*max_sample).max(sample);
            }
            slot @ None => *slot = Some((sample, sample)),
        }
    }

    buckets
        .into_iter()
        .enumerate()
        .filter_map(|(column, bucket)| {
            let (min_sample, max_sample) = bucket?;
            let x = if last_column_index == 0 {
                width / 2.0
            } else {
                column as f64 / last_column_index as f64 * (width - 1.0)
            };
            let min_y = sample_to_y(max_sample, lane_top, lane_height);
            let max_y = sample_to_y(min_sample, lane_top, lane_height);
            let (min_y, max_y) = ensure_visible_envelope(min_y, max_y, lane_top, lane_height);

            Some(EnvelopeColumn { x, min_y, max_y })
        })
        .collect()
}

fn draw_lane_background(cr: &gtk::cairo::Context, width: f64, top: f64, lane_height: f64) {
    set_source(cr, LANE_BACKGROUND);
    cr.rectangle(0.0, top, width, lane_height);
    let _ = cr.fill();

    set_source(cr, GRID);
    cr.set_line_width(1.0);
    cr.move_to(0.0, top + lane_height / 2.0);
    cr.line_to(width, top + lane_height / 2.0);
    let _ = cr.stroke();
}

fn draw_waveform_envelope(
    cr: &gtk::cairo::Context,
    width: f64,
    lane_top: f64,
    lane_height: f64,
    samples: &[u8],
    color: (f64, f64, f64),
) {
    let columns = build_envelope_columns(samples, width, lane_top, lane_height);
    if columns.is_empty() {
        return;
    }

    set_source(cr, color);
    cr.set_line_width(1.0);
    for column in columns {
        cr.move_to(column.x, column.min_y);
        cr.line_to(column.x, column.max_y);
    }
    let _ = cr.stroke();
}

fn sample_to_y(sample: u8, lane_top: f64, lane_height: f64) -> f64 {
    let usable_height = (lane_height - (2.0 * LANE_PADDING)).max(1.0);
    let normalized = f64::from(sample) / 255.0;
    lane_top + LANE_PADDING + usable_height - (normalized * usable_height)
}

fn ensure_visible_envelope(min_y: f64, max_y: f64, lane_top: f64, lane_height: f64) -> (f64, f64) {
    let lower_bound = lane_top + LANE_PADDING;
    let upper_bound = lane_top + lane_height - LANE_PADDING;
    if (max_y - min_y) >= MIN_VISIBLE_ENVELOPE_HEIGHT {
        return (
            min_y.clamp(lower_bound, upper_bound),
            max_y.clamp(lower_bound, upper_bound),
        );
    }

    let midpoint = ((min_y + max_y) / 2.0).clamp(lower_bound, upper_bound);
    let half_height = MIN_VISIBLE_ENVELOPE_HEIGHT / 2.0;
    (
        (midpoint - half_height).clamp(lower_bound, upper_bound),
        (midpoint + half_height).clamp(lower_bound, upper_bound),
    )
}

fn set_source(cr: &gtk::cairo::Context, color: (f64, f64, f64)) {
    cr.set_source_rgb(color.0, color.1, color.2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_builder_buckets_samples_by_output_column() {
        let columns = build_envelope_columns(&[0, 10, 40, 80, 120, 180], 3.0, 0.0, 100.0);

        assert_eq!(columns.len(), 3);
        assert!(columns[0].x < columns[1].x);
        assert!(columns[1].x < columns[2].x);
    }

    #[test]
    fn fullscale_mapping_is_independent_of_frame_local_range() {
        let low = sample_to_y(0, 0.0, 120.0);
        let midpoint = sample_to_y(128, 0.0, 120.0);
        let high = sample_to_y(255, 0.0, 120.0);

        assert!(high < midpoint);
        assert!(midpoint < low);
    }

    #[test]
    fn flat_buckets_stay_visible() {
        let columns = build_envelope_columns(&[120, 120, 120, 120], 2.0, 0.0, 100.0);

        assert_eq!(columns.len(), 2);
        for column in columns {
            assert!((column.max_y - column.min_y) >= MIN_VISIBLE_ENVELOPE_HEIGHT);
        }
    }

    #[test]
    fn narrow_noise_does_not_expand_to_full_lane_height() {
        let columns = build_envelope_columns(&[120, 121, 120, 121, 120, 121], 6.0, 0.0, 120.0);
        let usable_height = 120.0 - (2.0 * LANE_PADDING);

        assert!(!columns.is_empty());
        for column in columns {
            assert!((column.max_y - column.min_y) < usable_height / 8.0);
        }
    }
}
