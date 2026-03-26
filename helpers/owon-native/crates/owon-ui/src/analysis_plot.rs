use owon_analysis::{AnalysisReport, Histogram, Spectrogram, Spectrum};

const BACKGROUND: (f64, f64, f64) = (0.05, 0.07, 0.10);
const PANEL: (f64, f64, f64) = (0.10, 0.13, 0.17);
const GRID: (f64, f64, f64) = (0.22, 0.27, 0.33);
const SIGNAL: (f64, f64, f64) = (0.95, 0.74, 0.24);
const ACCENT: (f64, f64, f64) = (0.29, 0.82, 0.74);
const WARN: (f64, f64, f64) = (0.93, 0.43, 0.32);
const LABEL: (f64, f64, f64) = (0.72, 0.77, 0.83);
const PADDING: f64 = 14.0;

pub fn draw_signal_plot(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    report: Option<&AnalysisReport>,
) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    draw_panel_background(cr, width, height);

    let Some(report) = report else {
        draw_placeholder(
            cr,
            "Load a saved capture to inspect an offline waveform slice.",
        );
        return;
    };
    if report.raw_samples.len() < 2 {
        draw_placeholder(cr, "Selection is too small to draw.");
        return;
    }

    let plot = plot_bounds(width, height);
    draw_grid(cr, plot.0, plot.1, plot.2, plot.3, 8, 4);
    draw_waveform(cr, plot, &report.raw_samples, SIGNAL);
    draw_threshold_line(cr, plot, report.digital.threshold_u8);
    if let Some(trigger_index) = report.trigger_index_in_selection {
        draw_vertical_marker(cr, plot, trigger_index, report.raw_samples.len(), WARN);
    }
    draw_caption(
        cr,
        "Selected Signal",
        &format!(
            "{} samples, threshold {}",
            report.raw_samples.len(),
            report.digital.threshold_u8
        ),
        width,
    );
}

pub fn draw_histogram_plot(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    report: Option<&AnalysisReport>,
) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    draw_panel_background(cr, width, height);

    let Some(report) = report else {
        draw_placeholder(cr, "Histogram appears after offline analysis completes.");
        return;
    };

    let plot = plot_bounds(width, height);
    draw_grid(cr, plot.0, plot.1, plot.2, plot.3, 8, 4);
    draw_histogram(cr, plot, &report.histogram);
    draw_caption(
        cr,
        "Histogram",
        &format!("{} bins", report.histogram.bins.len()),
        width,
    );
}

pub fn draw_spectrum_plot(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    report: Option<&AnalysisReport>,
) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    draw_panel_background(cr, width, height);

    let Some(report) = report else {
        draw_placeholder(cr, "Spectrum appears after offline analysis completes.");
        return;
    };

    let plot = plot_bounds(width, height);
    draw_grid(cr, plot.0, plot.1, plot.2, plot.3, 8, 4);
    draw_spectrum(cr, plot, &report.spectrum);
    draw_caption(
        cr,
        "Spectrum / PSD View",
        &format!("Nyquist {:.0} Hz", report.spectrum.nyquist_hz),
        width,
    );
}

pub fn draw_spectrogram_plot(
    cr: &gtk::cairo::Context,
    width: i32,
    height: i32,
    report: Option<&AnalysisReport>,
) {
    let width = width.max(1) as f64;
    let height = height.max(1) as f64;
    draw_panel_background(cr, width, height);

    let Some(report) = report else {
        draw_placeholder(cr, "Spectrogram appears after offline analysis completes.");
        return;
    };

    let plot = plot_bounds(width, height);
    draw_grid(cr, plot.0, plot.1, plot.2, plot.3, 6, 4);
    draw_spectrogram(cr, plot, &report.spectrogram);
    draw_caption(
        cr,
        "Spectrogram",
        &format!(
            "{} slices x {} bins",
            report.spectrogram.slice_count, report.spectrogram.bin_count
        ),
        width,
    );
}

fn draw_panel_background(cr: &gtk::cairo::Context, width: f64, height: f64) {
    set_source(cr, BACKGROUND);
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();

    set_source(cr, PANEL);
    cr.rectangle(0.0, 0.0, width, height);
    let _ = cr.fill();
}

fn draw_placeholder(cr: &gtk::cairo::Context, text: &str) {
    set_source(cr, LABEL);
    cr.select_font_face(
        "Monospace",
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Normal,
    );
    cr.set_font_size(14.0);
    cr.move_to(PADDING, 28.0);
    let _ = cr.show_text(text);
}

fn plot_bounds(width: f64, height: f64) -> (f64, f64, f64, f64) {
    let left = PADDING;
    let top = PADDING + 14.0;
    let right = width - PADDING;
    let bottom = height - PADDING;
    (left, top, (right - left).max(1.0), (bottom - top).max(1.0))
}

fn draw_grid(
    cr: &gtk::cairo::Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    columns: usize,
    rows: usize,
) {
    set_source(cr, GRID);
    cr.set_line_width(1.0);
    for index in 0..=columns {
        let px = x + width * index as f64 / columns.max(1) as f64;
        cr.move_to(px, y);
        cr.line_to(px, y + height);
    }
    for index in 0..=rows {
        let py = y + height * index as f64 / rows.max(1) as f64;
        cr.move_to(x, py);
        cr.line_to(x + width, py);
    }
    let _ = cr.stroke();
}

fn draw_waveform(
    cr: &gtk::cairo::Context,
    plot: (f64, f64, f64, f64),
    samples: &[u8],
    color: (f64, f64, f64),
) {
    if samples.is_empty() {
        return;
    }
    let (x, y, width, height) = plot;
    set_source(cr, color);
    cr.set_line_width(1.2);
    for (index, sample) in samples.iter().enumerate() {
        let px = x + width * index as f64 / samples.len().saturating_sub(1).max(1) as f64;
        let py = y + height - (f64::from(*sample) / 255.0) * height;
        if index == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    let _ = cr.stroke();
}

fn draw_threshold_line(cr: &gtk::cairo::Context, plot: (f64, f64, f64, f64), threshold_u8: u8) {
    let (x, y, width, height) = plot;
    let py = y + height - (f64::from(threshold_u8) / 255.0) * height;
    set_source(cr, ACCENT);
    cr.set_line_width(1.0);
    cr.set_dash(&[4.0, 4.0], 0.0);
    cr.move_to(x, py);
    cr.line_to(x + width, py);
    let _ = cr.stroke();
    cr.set_dash(&[], 0.0);
}

fn draw_vertical_marker(
    cr: &gtk::cairo::Context,
    plot: (f64, f64, f64, f64),
    sample_index: usize,
    sample_count: usize,
    color: (f64, f64, f64),
) {
    let (x, y, width, height) = plot;
    let px = x + width * sample_index as f64 / sample_count.saturating_sub(1).max(1) as f64;
    set_source(cr, color);
    cr.set_line_width(1.0);
    cr.move_to(px, y);
    cr.line_to(px, y + height);
    let _ = cr.stroke();
}

fn draw_histogram(cr: &gtk::cairo::Context, plot: (f64, f64, f64, f64), histogram: &Histogram) {
    if histogram.bins.is_empty() || histogram.max_count == 0 {
        return;
    }
    let (x, y, width, height) = plot;
    let bar_width = width / histogram.bins.len().max(1) as f64;
    set_source(cr, SIGNAL);
    for (index, bin) in histogram.bins.iter().enumerate() {
        let bar_height = height * bin.count as f64 / histogram.max_count as f64;
        let px = x + index as f64 * bar_width;
        cr.rectangle(
            px,
            y + height - bar_height,
            (bar_width - 1.0).max(1.0),
            bar_height.max(1.0),
        );
        let _ = cr.fill();
    }
}

fn draw_spectrum(cr: &gtk::cairo::Context, plot: (f64, f64, f64, f64), spectrum: &Spectrum) {
    if spectrum.points.is_empty() {
        return;
    }
    let max_magnitude = spectrum
        .points
        .iter()
        .map(|point| point.magnitude)
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let (x, y, width, height) = plot;
    set_source(cr, ACCENT);
    cr.set_line_width(1.2);
    for (index, point) in spectrum.points.iter().enumerate() {
        let px = x + width * index as f64 / spectrum.points.len().saturating_sub(1).max(1) as f64;
        let py = y + height - (point.magnitude / max_magnitude) * height;
        if index == 0 {
            cr.move_to(px, py);
        } else {
            cr.line_to(px, py);
        }
    }
    let _ = cr.stroke();

    set_source(cr, WARN);
    for peak in &spectrum.peaks {
        let px = x + width * (peak.frequency_hz / spectrum.nyquist_hz).clamp(0.0, 1.0);
        cr.move_to(px, y);
        cr.line_to(px, y + height);
    }
    let _ = cr.stroke();
}

fn draw_spectrogram(
    cr: &gtk::cairo::Context,
    plot: (f64, f64, f64, f64),
    spectrogram: &Spectrogram,
) {
    if spectrogram.slice_count == 0
        || spectrogram.bin_count == 0
        || spectrogram.max_magnitude <= 0.0
    {
        return;
    }
    let (x, y, width, height) = plot;
    let cell_width = width / spectrogram.slice_count.max(1) as f64;
    let cell_height = height / spectrogram.bin_count.max(1) as f64;

    for slice in 0..spectrogram.slice_count {
        for bin in 0..spectrogram.bin_count {
            let index = slice * spectrogram.bin_count + bin;
            let magnitude = spectrogram.magnitudes.get(index).copied().unwrap_or(0.0);
            let normalized = (magnitude / spectrogram.max_magnitude).clamp(0.0, 1.0);
            let py = y + height - ((bin + 1) as f64 * cell_height);
            let px = x + slice as f64 * cell_width;
            cr.set_source_rgb(
                0.10 + 0.85 * normalized as f64,
                0.14 + 0.45 * normalized as f64,
                0.22 + 0.65 * (1.0 - normalized as f64),
            );
            cr.rectangle(
                px,
                py,
                cell_width.ceil().max(1.0),
                cell_height.ceil().max(1.0),
            );
            let _ = cr.fill();
        }
    }
}

fn draw_caption(cr: &gtk::cairo::Context, title: &str, subtitle: &str, width: f64) {
    cr.select_font_face(
        "Monospace",
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Bold,
    );
    cr.set_font_size(13.0);
    set_source(cr, LABEL);
    cr.move_to(PADDING, 16.0);
    let _ = cr.show_text(title);

    cr.select_font_face(
        "Monospace",
        gtk::cairo::FontSlant::Normal,
        gtk::cairo::FontWeight::Normal,
    );
    cr.set_font_size(11.0);
    let extents = cr.text_extents(subtitle).ok();
    let subtitle_x = extents
        .map(|extents| (width - PADDING - extents.width()).max(PADDING))
        .unwrap_or(PADDING);
    cr.move_to(subtitle_x, 16.0);
    let _ = cr.show_text(subtitle);
}

fn set_source(cr: &gtk::cairo::Context, color: (f64, f64, f64)) {
    cr.set_source_rgb(color.0, color.1, color.2);
}
