use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use adw::prelude::*;
use gtk::glib::{self, ControlFlow};
use gtk::{Orientation, PolicyType, WrapMode};
use owon_analysis::{AnalysisReport, AnalysisRequest, AnalysisSelection, DEFAULT_UART_BAUD};
use owon_device::{CaptureDebugInfo, NormalizedCapture};
use owon_protocol::ChannelId;

use crate::analysis_plot;
use crate::saved_capture::SavedCaptureEntry;
use crate::state::UiModel;
use crate::waveform;
use crate::worker::{WorkerEvent, WorkerHandle, spawn_worker};

pub fn run() {
    let app = adw::Application::builder()
        .application_id("dev.sel4.owon_vds1022")
        .build();

    app.connect_startup(|_| {
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
    });
    app.connect_activate(build_ui);
    let _ = app.run();
}

#[derive(Clone)]
struct UiWidgets {
    connection_label: gtk::Label,
    status_label: gtk::Label,
    error_label: gtk::Label,
    connect_button: gtk::Button,
    disconnect_button: gtk::Button,
    capture_once_button: gtk::Button,
    live_button: gtk::Button,
    refresh_saved_button: gtk::Button,
    load_saved_button: gtk::Button,
    saved_capture_combo: gtk::ComboBoxText,
    saved_capture_info_label: gtk::Label,
    waveform_hint_label: gtk::Label,
    waveform_area: gtk::DrawingArea,
    device_summary_view: gtk::TextView,
    capture_summary_view: gtk::TextView,
    debug_summary_view: gtk::TextView,
    analysis_hint_label: gtk::Label,
    analysis_status_label: gtk::Label,
    analysis_error_label: gtk::Label,
    analysis_channel_combo: gtk::ComboBoxText,
    analysis_start_spin: gtk::SpinButton,
    analysis_end_spin: gtk::SpinButton,
    analysis_uart_baud_entry: gtk::Entry,
    analysis_uart_invert_check: gtk::CheckButton,
    analysis_run_button: gtk::Button,
    analysis_signal_area: gtk::DrawingArea,
    analysis_histogram_area: gtk::DrawingArea,
    analysis_spectrum_area: gtk::DrawingArea,
    analysis_spectrogram_area: gtk::DrawingArea,
    analysis_metrics_view: gtk::TextView,
    analysis_digital_view: gtk::TextView,
    analysis_uart_view: gtk::TextView,
}

fn build_ui(app: &adw::Application) {
    let (worker, event_rx) = spawn_worker();
    let model = Rc::new(RefCell::new(UiModel::default()));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("OWON UI")
        .default_width(1560)
        .default_height(980)
        .build();

    let title = adw::WindowTitle::new(
        "OWON UI",
        "Manual native, live, saved, and offline analysis workspace",
    );
    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));

    let content = gtk::Box::new(Orientation::Vertical, 0);
    content.append(&header);

    let workspace = gtk::Box::new(Orientation::Horizontal, 16);
    workspace.set_margin_top(16);
    workspace.set_margin_bottom(16);
    workspace.set_margin_start(16);
    workspace.set_margin_end(16);
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);
    content.append(&workspace);

    let sidebar = gtk::Box::new(Orientation::Vertical, 12);
    sidebar.set_width_request(360);
    sidebar.set_hexpand(false);
    sidebar.set_vexpand(true);
    workspace.append(&sidebar);

    let connection_label = gtk::Label::new(None);
    connection_label.set_xalign(0.0);
    connection_label.add_css_class("title-3");
    sidebar.append(&connection_label);

    let status_label = gtk::Label::new(None);
    status_label.set_xalign(0.0);
    status_label.add_css_class("dim-label");
    sidebar.append(&status_label);

    let error_label = gtk::Label::new(None);
    error_label.set_xalign(0.0);
    error_label.set_wrap(true);
    error_label.set_visible(false);
    sidebar.append(&error_label);

    let button_row = gtk::Box::new(Orientation::Horizontal, 8);
    sidebar.append(&button_row);

    let connect_button = gtk::Button::with_label("Connect");
    connect_button.add_css_class("suggested-action");
    button_row.append(&connect_button);

    let disconnect_button = gtk::Button::with_label("Disconnect");
    button_row.append(&disconnect_button);

    let capture_once_button = gtk::Button::with_label("Capture Once");
    button_row.append(&capture_once_button);

    let live_button = gtk::Button::with_label("Start Live");
    live_button.add_css_class("suggested-action");
    button_row.append(&live_button);

    let saved_frame = gtk::Frame::new(Some("Saved Captures"));
    sidebar.append(&saved_frame);

    let saved_box = gtk::Box::new(Orientation::Vertical, 12);
    saved_box.set_margin_top(12);
    saved_box.set_margin_bottom(12);
    saved_box.set_margin_start(12);
    saved_box.set_margin_end(12);
    saved_frame.set_child(Some(&saved_box));

    let saved_capture_info_label = gtk::Label::new(None);
    saved_capture_info_label.set_xalign(0.0);
    saved_capture_info_label.set_wrap(true);
    saved_capture_info_label.add_css_class("dim-label");
    saved_box.append(&saved_capture_info_label);

    let saved_capture_combo = gtk::ComboBoxText::new();
    saved_capture_combo.set_hexpand(true);
    saved_box.append(&saved_capture_combo);

    let saved_button_row = gtk::Box::new(Orientation::Horizontal, 8);
    saved_box.append(&saved_button_row);

    let load_saved_button = gtk::Button::with_label("Show Saved");
    saved_button_row.append(&load_saved_button);

    let refresh_saved_button = gtk::Button::with_label("Refresh List");
    saved_button_row.append(&refresh_saved_button);

    let (device_frame, device_summary_view) = make_text_panel(
        "Device Summary",
        "Connect to load flash, FPGA, and transport details.",
        WrapMode::WordChar,
    );
    device_frame.set_vexpand(true);
    sidebar.append(&device_frame);

    let notebook = gtk::Notebook::new();
    notebook.set_hexpand(true);
    notebook.set_vexpand(true);
    workspace.append(&notebook);

    let capture_page = gtk::Box::new(Orientation::Vertical, 0);
    capture_page.set_hexpand(true);
    capture_page.set_vexpand(true);

    let main_paned = gtk::Paned::new(Orientation::Vertical);
    main_paned.set_wide_handle(true);
    main_paned.set_hexpand(true);
    main_paned.set_vexpand(true);
    main_paned.set_position(560);
    capture_page.append(&main_paned);

    let waveform_box = gtk::Box::new(Orientation::Vertical, 8);
    waveform_box.set_hexpand(true);
    waveform_box.set_vexpand(true);

    let waveform_hint_label = gtk::Label::new(None);
    waveform_hint_label.set_xalign(0.0);
    waveform_hint_label.set_wrap(true);
    waveform_box.append(&waveform_hint_label);

    let waveform_frame = gtk::Frame::new(Some("Waveform (Raw ADC Envelope)"));
    waveform_frame.set_hexpand(true);
    waveform_frame.set_vexpand(true);
    waveform_box.append(&waveform_frame);

    let waveform_area = gtk::DrawingArea::new();
    waveform_area.set_content_width(900);
    waveform_area.set_content_height(520);
    waveform_area.set_hexpand(true);
    waveform_area.set_vexpand(true);
    waveform_frame.set_child(Some(&waveform_area));

    main_paned.set_start_child(Some(&waveform_box));
    main_paned.set_resize_start_child(true);
    main_paned.set_shrink_start_child(false);

    let detail_paned = gtk::Paned::new(Orientation::Horizontal);
    detail_paned.set_wide_handle(true);
    detail_paned.set_hexpand(true);
    detail_paned.set_vexpand(true);
    detail_paned.set_position(430);

    let (capture_frame, capture_summary_view) =
        make_text_panel("Capture Metadata", "No capture yet.", WrapMode::None);
    capture_frame.set_hexpand(true);
    capture_frame.set_vexpand(true);
    detail_paned.set_start_child(Some(&capture_frame));
    detail_paned.set_resize_start_child(true);
    detail_paned.set_shrink_start_child(false);

    let (debug_frame, debug_summary_view) = make_text_panel(
        "Capture Debug",
        "No capture debug data yet.",
        WrapMode::None,
    );
    debug_frame.set_hexpand(true);
    debug_frame.set_vexpand(true);
    detail_paned.set_end_child(Some(&debug_frame));
    detail_paned.set_resize_end_child(true);
    detail_paned.set_shrink_end_child(false);

    main_paned.set_end_child(Some(&detail_paned));
    main_paned.set_resize_end_child(true);
    main_paned.set_shrink_end_child(false);

    notebook.append_page(&capture_page, Some(&gtk::Label::new(Some("Capture"))));

    let analysis_page = gtk::Box::new(Orientation::Vertical, 10);
    analysis_page.set_hexpand(true);
    analysis_page.set_vexpand(true);

    let analysis_hint_label = gtk::Label::new(None);
    analysis_hint_label.set_xalign(0.0);
    analysis_hint_label.set_wrap(true);
    analysis_page.append(&analysis_hint_label);

    let analysis_status_label = gtk::Label::new(None);
    analysis_status_label.set_xalign(0.0);
    analysis_status_label.add_css_class("dim-label");
    analysis_status_label.set_wrap(true);
    analysis_page.append(&analysis_status_label);

    let analysis_error_label = gtk::Label::new(None);
    analysis_error_label.set_xalign(0.0);
    analysis_error_label.set_wrap(true);
    analysis_error_label.set_visible(false);
    analysis_page.append(&analysis_error_label);

    let analysis_controls_frame = gtk::Frame::new(Some("Offline Analysis Controls"));
    analysis_page.append(&analysis_controls_frame);

    let analysis_controls = gtk::Box::new(Orientation::Vertical, 10);
    analysis_controls.set_margin_top(12);
    analysis_controls.set_margin_bottom(12);
    analysis_controls.set_margin_start(12);
    analysis_controls.set_margin_end(12);
    analysis_controls_frame.set_child(Some(&analysis_controls));

    let control_row = gtk::Box::new(Orientation::Horizontal, 10);
    analysis_controls.append(&control_row);

    let analysis_channel_combo = gtk::ComboBoxText::new();
    analysis_channel_combo.set_hexpand(false);
    append_labeled(&control_row, "Channel", &analysis_channel_combo);

    let analysis_start_spin = make_spin_button();
    append_labeled(&control_row, "Start", &analysis_start_spin);

    let analysis_end_spin = make_spin_button();
    append_labeled(&control_row, "End", &analysis_end_spin);

    let analysis_uart_baud_entry = gtk::Entry::new();
    analysis_uart_baud_entry.set_width_chars(10);
    analysis_uart_baud_entry.set_text(&format!("{DEFAULT_UART_BAUD:.0}"));
    append_labeled(&control_row, "UART baud", &analysis_uart_baud_entry);

    let analysis_uart_invert_check = gtk::CheckButton::with_label("Invert UART");
    control_row.append(&analysis_uart_invert_check);

    let analysis_run_button = gtk::Button::with_label("Run Analysis");
    analysis_run_button.add_css_class("suggested-action");
    control_row.append(&analysis_run_button);

    let plot_grid = gtk::Grid::new();
    plot_grid.set_hexpand(true);
    plot_grid.set_vexpand(true);
    plot_grid.set_row_spacing(10);
    plot_grid.set_column_spacing(10);
    analysis_page.append(&plot_grid);

    let (analysis_signal_frame, analysis_signal_area) =
        make_plot_frame("Selected Signal", 700, 240);
    plot_grid.attach(&analysis_signal_frame, 0, 0, 1, 1);

    let (analysis_spectrum_frame, analysis_spectrum_area) = make_plot_frame("Spectrum", 700, 240);
    plot_grid.attach(&analysis_spectrum_frame, 1, 0, 1, 1);

    let (analysis_histogram_frame, analysis_histogram_area) =
        make_plot_frame("Histogram", 700, 240);
    plot_grid.attach(&analysis_histogram_frame, 0, 1, 1, 1);

    let (analysis_spectrogram_frame, analysis_spectrogram_area) =
        make_plot_frame("Spectrogram", 700, 240);
    plot_grid.attach(&analysis_spectrogram_frame, 1, 1, 1, 1);

    let analysis_detail_box = gtk::Box::new(Orientation::Horizontal, 10);
    analysis_detail_box.set_hexpand(true);
    analysis_detail_box.set_vexpand(true);
    analysis_page.append(&analysis_detail_box);

    let (metrics_frame, analysis_metrics_view) = make_text_panel(
        "Measurements",
        "Run offline analysis to populate derived signal metrics.",
        WrapMode::WordChar,
    );
    analysis_detail_box.append(&metrics_frame);

    let (digital_frame, analysis_digital_view) = make_text_panel(
        "Digital / Events",
        "Run offline analysis to inspect thresholded edge timing.",
        WrapMode::WordChar,
    );
    analysis_detail_box.append(&digital_frame);

    let (uart_frame, analysis_uart_view) = make_text_panel(
        "UART / Decoder",
        "Run offline analysis to inspect built-in decoder output.",
        WrapMode::WordChar,
    );
    analysis_detail_box.append(&uart_frame);

    notebook.append_page(&analysis_page, Some(&gtk::Label::new(Some("Analysis"))));

    window.set_content(Some(&content));

    let widgets = UiWidgets {
        connection_label,
        status_label,
        error_label,
        connect_button,
        disconnect_button,
        capture_once_button,
        live_button,
        refresh_saved_button,
        load_saved_button,
        saved_capture_combo,
        saved_capture_info_label,
        waveform_hint_label,
        waveform_area,
        device_summary_view,
        capture_summary_view,
        debug_summary_view,
        analysis_hint_label,
        analysis_status_label,
        analysis_error_label,
        analysis_channel_combo,
        analysis_start_spin,
        analysis_end_spin,
        analysis_uart_baud_entry,
        analysis_uart_invert_check,
        analysis_run_button,
        analysis_signal_area,
        analysis_histogram_area,
        analysis_spectrum_area,
        analysis_spectrogram_area,
        analysis_metrics_view,
        analysis_digital_view,
        analysis_uart_view,
    };

    hook_renderers(&widgets, model.clone());
    hook_buttons(&widgets, model.clone(), worker.clone());
    hook_worker_events(&widgets, model.clone(), event_rx);
    hook_shutdown(&window, worker.clone());
    refresh_ui_from_model(&widgets, &model);
    if let Err(error) = worker.refresh_saved_captures() {
        let mut state = model.borrow_mut();
        state.on_failure(error);
    }
    refresh_ui_from_model(&widgets, &model);
    window.present();
}

fn hook_buttons(widgets: &UiWidgets, model: Rc<RefCell<UiModel>>, worker: WorkerHandle) {
    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.connect_button.clone();
        button.connect_clicked(move |_| {
            let mut state = model.borrow_mut();
            if !state.begin_connect() {
                return;
            }
            if let Err(error) = worker.connect() {
                state.on_failure(error);
            }
            drop(state);
            refresh_ui_from_model(&widgets, &model);
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.disconnect_button.clone();
        button.connect_clicked(move |_| {
            let mut state = model.borrow_mut();
            if !state.begin_disconnect() {
                return;
            }
            if let Err(error) = worker.disconnect() {
                state.on_failure(error);
            }
            drop(state);
            refresh_ui_from_model(&widgets, &model);
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.capture_once_button.clone();
        button.connect_clicked(move |_| {
            let mut state = model.borrow_mut();
            if !state.begin_capture_once() {
                return;
            }
            if let Err(error) = worker.capture_once() {
                state.on_failure(error);
            }
            drop(state);
            refresh_ui_from_model(&widgets, &model);
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.live_button.clone();
        button.connect_clicked(move |_| {
            let mut state = model.borrow_mut();
            let result = if state.is_live_running() {
                if !state.begin_stop_live() {
                    return;
                }
                worker.stop_live_capture()
            } else {
                if !state.begin_start_live() {
                    return;
                }
                worker.start_live_capture()
            };

            if let Err(error) = result {
                state.on_failure(error);
            }
            drop(state);
            refresh_ui_from_model(&widgets, &model);
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.refresh_saved_button.clone();
        button.connect_clicked(move |_| {
            if let Err(error) = worker.refresh_saved_captures() {
                let mut state = model.borrow_mut();
                state.on_failure(error);
                drop(state);
                refresh_ui_from_model(&widgets, &model);
            }
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.load_saved_button.clone();
        let combo = widgets.saved_capture_combo.clone();
        button.connect_clicked(move |_| {
            let Some(meta_id) = combo.active_id() else {
                let mut state = model.borrow_mut();
                state.on_failure("Select a saved capture first.".to_owned());
                drop(state);
                refresh_ui_from_model(&widgets, &model);
                return;
            };

            let mut state = model.borrow_mut();
            if !state.begin_load_saved_capture() {
                return;
            }
            if let Err(error) = worker.load_saved_capture(PathBuf::from(meta_id.as_str())) {
                state.on_failure(error);
            }
            drop(state);
            refresh_ui_from_model(&widgets, &model);
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let worker = worker.clone();
        let button = widgets.analysis_run_button.clone();
        button.connect_clicked(move |_| {
            let request = {
                let state = model.borrow();
                match build_analysis_request_from_widgets(&state, &widgets) {
                    Ok(request) => request,
                    Err(error) => {
                        drop(state);
                        let mut state = model.borrow_mut();
                        state.on_analysis_input_error(error);
                        drop(state);
                        refresh_ui_from_model(&widgets, &model);
                        return;
                    }
                }
            };

            let capture = {
                let state = model.borrow();
                let Some(capture) = state.last_capture().cloned() else {
                    drop(state);
                    let mut state = model.borrow_mut();
                    state.on_analysis_input_error(
                        "Load a saved capture before running offline analysis.".to_owned(),
                    );
                    drop(state);
                    refresh_ui_from_model(&widgets, &model);
                    return;
                };
                capture
            };

            let mut state = model.borrow_mut();
            if !state.begin_run_analysis(request.clone()) {
                return;
            }
            if let Err(error) = worker.run_analysis(capture, request) {
                state.on_failure(error);
            }
            drop(state);
            refresh_ui_from_model(&widgets, &model);
        });
    }

    {
        let model = model.clone();
        let widgets = widgets.clone();
        let combo = widgets.analysis_channel_combo.clone();
        combo.connect_changed(move |_| {
            let state = model.borrow();
            sync_analysis_range_controls(&widgets, &state, true);
        });
    }
}

fn hook_worker_events(
    widgets: &UiWidgets,
    model: Rc<RefCell<UiModel>>,
    event_rx: mpsc::Receiver<WorkerEvent>,
) {
    let widgets = widgets.clone();
    glib::timeout_add_local(Duration::from_millis(40), move || {
        let mut changed = false;
        while let Ok(event) = event_rx.try_recv() {
            changed = true;
            {
                let mut state = model.borrow_mut();
                match event {
                    WorkerEvent::Connected { device_summary } => state.on_connected(device_summary),
                    WorkerEvent::LiveStarted => state.on_live_started(),
                    WorkerEvent::LiveRetrying {
                        error,
                        consecutive_failures,
                    } => state.on_live_retrying(error, consecutive_failures),
                    WorkerEvent::LiveStopped => state.on_live_stopped(),
                    WorkerEvent::Disconnected { warning } => state.on_disconnected(warning),
                    WorkerEvent::CaptureReady(result) => state.on_capture_ready(result),
                    WorkerEvent::LiveCaptureReady(result) => state.on_live_capture_ready(result),
                    WorkerEvent::SavedCapturesScanned(captures) => {
                        state.on_saved_captures_scanned(captures)
                    }
                    WorkerEvent::SavedCaptureLoaded {
                        entry,
                        note,
                        result,
                    } => state.on_saved_capture_ready(entry.label, note, result),
                    WorkerEvent::AnalysisReady { result } => state.on_analysis_ready(result),
                    WorkerEvent::Failed(error) => state.on_failure(error),
                }
            }
            refresh_ui_from_model(&widgets, &model);
        }

        if changed {
            widgets.waveform_area.queue_draw();
            widgets.analysis_signal_area.queue_draw();
            widgets.analysis_histogram_area.queue_draw();
            widgets.analysis_spectrum_area.queue_draw();
            widgets.analysis_spectrogram_area.queue_draw();
        }

        ControlFlow::Continue
    });
}

fn hook_shutdown(window: &adw::ApplicationWindow, worker: WorkerHandle) {
    window.connect_close_request(move |_| {
        let _ = worker.quit();
        glib::Propagation::Proceed
    });
}

fn hook_renderers(widgets: &UiWidgets, model: Rc<RefCell<UiModel>>) {
    widgets.waveform_area.set_draw_func({
        let model = model.clone();
        move |_, cr, width, height| {
            let state = model.borrow();
            let capture = state.last_capture().map(|result| &result.capture);
            waveform::draw_capture(cr, width, height, capture);
        }
    });

    widgets.analysis_signal_area.set_draw_func({
        let model = model.clone();
        move |_, cr, width, height| {
            let state = model.borrow();
            analysis_plot::draw_signal_plot(cr, width, height, state.analysis_result());
        }
    });

    widgets.analysis_histogram_area.set_draw_func({
        let model = model.clone();
        move |_, cr, width, height| {
            let state = model.borrow();
            analysis_plot::draw_histogram_plot(cr, width, height, state.analysis_result());
        }
    });

    widgets.analysis_spectrum_area.set_draw_func({
        let model = model.clone();
        move |_, cr, width, height| {
            let state = model.borrow();
            analysis_plot::draw_spectrum_plot(cr, width, height, state.analysis_result());
        }
    });

    widgets
        .analysis_spectrogram_area
        .set_draw_func(move |_, cr, width, height| {
            let state = model.borrow();
            analysis_plot::draw_spectrogram_plot(cr, width, height, state.analysis_result());
        });
}

fn refresh_ui_from_model(widgets: &UiWidgets, model: &Rc<RefCell<UiModel>>) {
    let state = model.borrow();
    refresh_ui(widgets, &state);
}

fn refresh_ui(widgets: &UiWidgets, state: &UiModel) {
    widgets.connection_label.set_text(state.connection_text());
    widgets.status_label.set_text(state.status_text());
    widgets.waveform_hint_label.set_text(&state.waveform_hint());
    widgets
        .saved_capture_info_label
        .set_text(&state.saved_captures_summary());

    set_text_view_text(&widgets.device_summary_view, state.device_summary());
    if state.can_interact_with_saved_captures() {
        sync_saved_capture_combo(&widgets.saved_capture_combo, state.saved_captures());
    }
    widgets
        .saved_capture_combo
        .set_sensitive(state.can_interact_with_saved_captures());

    if let Some(error) = state.error_text() {
        widgets.error_label.set_text(error);
        widgets.error_label.set_visible(true);
    } else {
        widgets.error_label.set_visible(false);
        widgets.error_label.set_text("");
    }

    widgets.connect_button.set_sensitive(state.can_connect());
    widgets
        .disconnect_button
        .set_sensitive(state.can_disconnect());
    widgets
        .capture_once_button
        .set_sensitive(state.can_capture_once());
    widgets.live_button.set_label(state.live_button_label());
    widgets
        .live_button
        .set_sensitive(state.can_start_live() || state.can_stop_live());
    widgets
        .refresh_saved_button
        .set_sensitive(state.can_interact_with_saved_captures());
    widgets.load_saved_button.set_sensitive(
        state.can_interact_with_saved_captures() && !state.saved_captures().is_empty(),
    );

    if let Some(result) = state.last_capture() {
        set_text_view_text(
            &widgets.capture_summary_view,
            &format_capture_summary(&result.capture),
        );
        set_text_view_text(
            &widgets.debug_summary_view,
            &format_debug_summary(&result.debug, state.last_capture_note()),
        );
    } else {
        set_text_view_text(&widgets.capture_summary_view, "No capture yet.");
        set_text_view_text(&widgets.debug_summary_view, "No capture debug data yet.");
    }

    widgets.analysis_hint_label.set_text(&state.analysis_hint());
    widgets
        .analysis_status_label
        .set_text(state.analysis_status_text());
    if let Some(error) = state.analysis_error_text() {
        widgets.analysis_error_label.set_text(error);
        widgets.analysis_error_label.set_visible(true);
    } else {
        widgets.analysis_error_label.set_visible(false);
        widgets.analysis_error_label.set_text("");
    }

    sync_analysis_channel_combo(&widgets.analysis_channel_combo, state.last_capture());
    sync_analysis_range_controls(widgets, state, false);
    widgets
        .analysis_channel_combo
        .set_sensitive(state.can_run_analysis());
    widgets
        .analysis_start_spin
        .set_sensitive(state.can_run_analysis());
    widgets
        .analysis_end_spin
        .set_sensitive(state.can_run_analysis());
    widgets
        .analysis_uart_baud_entry
        .set_sensitive(state.can_run_analysis());
    widgets
        .analysis_uart_invert_check
        .set_sensitive(state.can_run_analysis());
    widgets
        .analysis_run_button
        .set_sensitive(state.can_run_analysis());

    if let Some(report) = state.analysis_result() {
        set_text_view_text(
            &widgets.analysis_metrics_view,
            &format_measurements_summary(report, state.analysis_request()),
        );
        set_text_view_text(
            &widgets.analysis_digital_view,
            &format_digital_summary(report),
        );
        set_text_view_text(&widgets.analysis_uart_view, &format_uart_summary(report));
    } else {
        set_text_view_text(
            &widgets.analysis_metrics_view,
            "Run offline analysis to populate derived signal metrics.",
        );
        set_text_view_text(
            &widgets.analysis_digital_view,
            "Run offline analysis to inspect thresholded edge timing.",
        );
        set_text_view_text(
            &widgets.analysis_uart_view,
            "Run offline analysis to inspect built-in decoder output.",
        );
    }

    widgets.waveform_area.queue_draw();
    widgets.analysis_signal_area.queue_draw();
    widgets.analysis_histogram_area.queue_draw();
    widgets.analysis_spectrum_area.queue_draw();
    widgets.analysis_spectrogram_area.queue_draw();
}

fn sync_saved_capture_combo(combo: &gtk::ComboBoxText, captures: &[SavedCaptureEntry]) {
    let previous = combo.active_id().map(|id| id.to_string());
    combo.remove_all();
    for capture in captures {
        let id = capture.id();
        combo.append(Some(&id), &capture.label);
    }

    if let Some(previous) = previous.as_deref() {
        if combo.set_active_id(Some(previous)) {
            return;
        }
    }

    if !captures.is_empty() {
        combo.set_active(Some(0));
    }
}

fn sync_analysis_channel_combo(
    combo: &gtk::ComboBoxText,
    capture: Option<&owon_device::CaptureOnceResult>,
) {
    let previous = combo.active_id().map(|id| id.to_string());
    combo.remove_all();

    if let Some(capture) = capture {
        for channel in &capture.capture.channels {
            combo.append(
                Some(&channel.channel.short_name().to_ascii_lowercase()),
                channel.channel.short_name(),
            );
        }
    }

    if let Some(previous) = previous.as_deref() {
        if combo.set_active_id(Some(previous)) {
            return;
        }
    }

    if combo.active_id().is_none() && combo.active().is_none() {
        combo.set_active(Some(0));
    }
}

fn sync_analysis_range_controls(widgets: &UiWidgets, state: &UiModel, force_full_range: bool) {
    let Some(capture) = state.last_capture() else {
        widgets.analysis_start_spin.set_range(0.0, 0.0);
        widgets.analysis_end_spin.set_range(0.0, 0.0);
        widgets.analysis_start_spin.set_value(0.0);
        widgets.analysis_end_spin.set_value(0.0);
        return;
    };

    let Some(channel) = active_channel_from_combo(&widgets.analysis_channel_combo) else {
        return;
    };
    let Some(channel_capture) = capture.capture.channel(channel) else {
        return;
    };
    let max_index = channel_capture.samples.len().saturating_sub(1) as f64;
    widgets.analysis_start_spin.set_range(0.0, max_index);
    widgets.analysis_end_spin.set_range(0.0, max_index);
    widgets.analysis_end_spin.set_increments(1.0, 64.0);
    widgets.analysis_start_spin.set_increments(1.0, 64.0);

    if force_full_range || widgets.analysis_end_spin.value() < widgets.analysis_start_spin.value() {
        widgets.analysis_start_spin.set_value(0.0);
        widgets.analysis_end_spin.set_value(max_index);
    } else {
        let clamped_start = widgets.analysis_start_spin.value().clamp(0.0, max_index);
        let clamped_end = widgets
            .analysis_end_spin
            .value()
            .clamp(clamped_start, max_index);
        widgets.analysis_start_spin.set_value(clamped_start);
        widgets.analysis_end_spin.set_value(clamped_end);
    }

    if let Some(request) = state.analysis_request() {
        if request.channel == channel && !force_full_range {
            widgets
                .analysis_start_spin
                .set_value(request.selection.start as f64);
            widgets
                .analysis_end_spin
                .set_value(request.selection.end as f64);
        }
    }
}

fn build_analysis_request_from_widgets(
    state: &UiModel,
    widgets: &UiWidgets,
) -> Result<AnalysisRequest, String> {
    if !state.has_saved_capture_loaded() {
        return Err("Offline analysis currently runs only on saved captures.".to_owned());
    }

    let capture = state
        .last_capture()
        .ok_or_else(|| "Load a saved capture before running offline analysis.".to_owned())?;
    let channel = active_channel_from_combo(&widgets.analysis_channel_combo)
        .ok_or_else(|| "Choose an analysis channel first.".to_owned())?;
    let channel_capture = capture
        .capture
        .channel(channel)
        .ok_or_else(|| format!("Current capture does not contain {channel}"))?;
    let sample_count = channel_capture.samples.len();
    let start = widgets.analysis_start_spin.value().round().max(0.0) as usize;
    let end = widgets.analysis_end_spin.value().round().max(0.0) as usize;
    let selection = AnalysisSelection { start, end }.clamp(sample_count);
    if selection.len() < 2 {
        return Err("Analysis selection must include at least two samples.".to_owned());
    }

    let uart_baud = widgets
        .analysis_uart_baud_entry
        .text()
        .parse::<f64>()
        .map_err(|_| "UART baud must be a positive number.".to_owned())?;
    if !uart_baud.is_finite() || uart_baud <= 0.0 {
        return Err("UART baud must be a positive number.".to_owned());
    }

    let mut request = AnalysisRequest::default_for_capture(&capture.capture, channel);
    request.selection = selection;
    request.uart_baud = uart_baud;
    request.uart_invert = widgets.analysis_uart_invert_check.is_active();
    Ok(request)
}

fn active_channel_from_combo(combo: &gtk::ComboBoxText) -> Option<ChannelId> {
    match combo.active_id()?.as_str() {
        "ch1" => Some(ChannelId::Ch1),
        "ch2" => Some(ChannelId::Ch2),
        _ => None,
    }
}

fn make_text_panel(
    title: &str,
    initial_text: &str,
    wrap_mode: WrapMode,
) -> (gtk::Frame, gtk::TextView) {
    let view = gtk::TextView::new();
    view.set_editable(false);
    view.set_cursor_visible(false);
    view.set_monospace(true);
    view.set_wrap_mode(wrap_mode);
    view.set_hexpand(true);
    view.set_vexpand(true);
    view.set_left_margin(12);
    view.set_right_margin(12);
    view.set_top_margin(12);
    view.set_bottom_margin(12);
    view.buffer().set_text(initial_text);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(PolicyType::Automatic, PolicyType::Automatic);
    scroller.set_hexpand(true);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&view));

    let frame = gtk::Frame::new(Some(title));
    frame.set_hexpand(true);
    frame.set_vexpand(true);
    frame.set_child(Some(&scroller));

    (frame, view)
}

fn make_plot_frame(title: &str, width: i32, height: i32) -> (gtk::Frame, gtk::DrawingArea) {
    let area = gtk::DrawingArea::new();
    area.set_content_width(width);
    area.set_content_height(height);
    area.set_hexpand(true);
    area.set_vexpand(true);

    let frame = gtk::Frame::new(Some(title));
    frame.set_hexpand(true);
    frame.set_vexpand(true);
    frame.set_child(Some(&area));
    (frame, area)
}

fn make_spin_button() -> gtk::SpinButton {
    let adjustment = gtk::Adjustment::new(0.0, 0.0, 0.0, 1.0, 64.0, 0.0);
    let spin = gtk::SpinButton::new(Some(&adjustment), 1.0, 0);
    spin.set_numeric(true);
    spin.set_width_chars(8);
    spin
}

fn append_labeled<W: IsA<gtk::Widget>>(parent: &gtk::Box, label: &str, widget: &W) {
    let box_ = gtk::Box::new(Orientation::Vertical, 4);
    let label_widget = gtk::Label::new(Some(label));
    label_widget.set_xalign(0.0);
    box_.append(&label_widget);
    box_.append(widget);
    parent.append(&box_);
}

fn set_text_view_text(view: &gtk::TextView, text: &str) {
    view.buffer().set_text(text);
}

fn format_capture_summary(capture: &NormalizedCapture) -> String {
    let mut lines = vec![
        format!("Sample rate: {} Hz", capture.sample_rate_hz),
        format!("Sample period: {} ns", capture.sample_period_ns),
        format!("Channels: {}", capture.channels.len()),
    ];

    for channel in &capture.channels {
        let trigger = channel
            .trigger_index
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_owned());
        let frequency = channel
            .frequency_hz
            .map(|value| format!("{value:.3} Hz"))
            .unwrap_or_else(|| "unknown".to_owned());
        lines.push(format!(
            "{}: samples={} trigger={} frequency={}",
            channel.channel,
            channel.samples.len(),
            trigger,
            frequency,
        ));
    }

    lines.join("\n")
}

fn format_debug_summary(debug: &CaptureDebugInfo, note: Option<&str>) -> String {
    let mut lines = Vec::new();
    if let Some(note) = note {
        lines.push(format!("Note: {note}"));
        lines.push(String::new());
    }

    lines.extend([
        format!(
            "Requested channels: {}",
            debug
                .requested_channels
                .iter()
                .map(|channel| channel.short_name())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        format!("Volt range index: {}", debug.volt_range_index),
        format!("Timebase prescaler: {}", debug.timebase_prescaler),
        format!("Roll mode: {}", yes_no(debug.rollmode)),
        format!("Peak mode: {}", yes_no(debug.peakmode)),
        format!("Pre-trigger samples: {}", debug.pre_trigger_samples),
        format!("Post-trigger samples: {}", debug.post_trigger_samples),
        format!("Trigger holdoff: 0x{:04x}", debug.trigger_holdoff),
        format!("Edge level: 0x{:04x}", debug.edge_level),
        format!("Freqref: 0x{:02x}", debug.freqref),
        format!("Phase fine: {}", debug.phasefine),
        String::new(),
        "Calibrations:".to_owned(),
    ]);

    for calibration in &debug.calibrations {
        lines.push(format!(
            "  {} zero_offset={} voltage_gain={}",
            calibration.channel, calibration.zero_offset, calibration.voltage_gain,
        ));
    }

    lines.push(String::new());
    lines.push("Channels:".to_owned());
    for channel in &debug.channels {
        lines.push(format!(
            "  {} cursor={} time_sum={} period_num={} sample_offset={} sample_count={} u8={}..{} i8={}..{}",
            channel.channel,
            channel.cursor_from_right,
            channel.time_sum,
            channel.period_num,
            channel.sample_offset,
            channel.sample_count,
            channel.sample_min_u8,
            channel.sample_max_u8,
            channel.sample_min_i8,
            channel.sample_max_i8,
        ));
    }

    lines.join("\n")
}

fn format_measurements_summary(
    report: &AnalysisReport,
    request: Option<&AnalysisRequest>,
) -> String {
    let mut lines = vec![
        format!("Channel: {}", report.channel),
        format!(
            "Selection: {}..{} ({} samples)",
            report.selection.start,
            report.selection.end,
            report.raw_samples.len()
        ),
        format!("Sample rate: {:.3} Hz", report.sample_rate_hz),
        format!(
            "Min / Max: {} / {}",
            report.stats.min_u8, report.stats.max_u8
        ),
        format!("Mean: {:.3}", report.stats.mean_u8),
        format!("Stddev: {:.3}", report.stats.stddev_u8),
        format!("RMS (centered): {:.3}", report.stats.rms_centered),
        format!("Peak-to-peak: {}", report.stats.peak_to_peak_u8),
        format!(
            "DC offset from midscale: {:.3}",
            report.stats.dc_offset_from_midscale
        ),
        format!("Zero crossings: {}", report.stats.zero_crossings),
    ];

    if let Some(freq) = report.stats.estimated_frequency_hz {
        lines.push(format!("Coarse frequency estimate: {:.3} Hz", freq));
    } else {
        lines.push("Coarse frequency estimate: unavailable".to_owned());
    }

    if !report.spectrum.peaks.is_empty() {
        lines.push(String::new());
        lines.push("Dominant spectral peaks:".to_owned());
        for peak in &report.spectrum.peaks {
            lines.push(format!(
                "  {:.3} Hz magnitude {:.5}",
                peak.frequency_hz, peak.magnitude
            ));
        }
    }

    if let Some(request) = request {
        lines.push(String::new());
        lines.push(format!(
            "Analysis params: histogram_bins={} spectrum_bins={} spectrogram_window={} spectrogram_step={}",
            request.histogram_bins,
            request.spectrum_bins,
            request.spectrogram_window,
            request.spectrogram_step,
        ));
    }

    lines.join("\n")
}

fn format_digital_summary(report: &AnalysisReport) -> String {
    let mut lines = vec![
        format!("Threshold: {}", report.digital.threshold_u8),
        format!("High samples: {}", report.digital.high_sample_count),
        format!("Low samples: {}", report.digital.low_sample_count),
        format!("Rising edges: {}", report.digital.rising_edges),
        format!("Falling edges: {}", report.digital.falling_edges),
    ];

    if let Some(mean) = report.digital.mean_high_samples {
        lines.push(format!("Mean high width: {:.3} samples", mean));
    }
    if let Some(mean) = report.digital.mean_low_samples {
        lines.push(format!("Mean low width: {:.3} samples", mean));
    }
    if let Some(period) = report.digital.estimated_period_samples {
        lines.push(format!("Estimated period: {:.3} samples", period));
    }
    if let Some(freq) = report.digital.estimated_frequency_hz {
        lines.push(format!("Estimated digital frequency: {:.3} Hz", freq));
    }
    if let Some(note) = &report.digital.quality_note {
        lines.push(String::new());
        lines.push(format!("Quality note: {note}"));
    }

    if !report.digital.first_transitions.is_empty() {
        lines.push(String::new());
        lines.push("First transitions:".to_owned());
        for transition in &report.digital.first_transitions {
            let direction = match transition.direction {
                owon_analysis::EdgeDirection::Rising => "rising",
                owon_analysis::EdgeDirection::Falling => "falling",
            };
            lines.push(format!(
                "  sample {} -> {}",
                transition.sample_index, direction
            ));
        }
    }

    lines.join("\n")
}

fn format_uart_summary(report: &AnalysisReport) -> String {
    let uart = &report.uart;
    let mut lines = vec![
        format!("Baud: {:.3}", uart.baud),
        format!("Invert: {}", yes_no(uart.invert)),
        format!("Threshold: {}", uart.threshold_u8),
        format!("Frames: {}", uart.frame_count),
        format!("Good stop bits: {}", uart.good_stop_count),
        format!("Bad stop bits: {}", uart.bad_stop_count),
        format!(
            "ASCII: {}",
            if uart.ascii_rendered.is_empty() {
                "<none>"
            } else {
                &uart.ascii_rendered
            }
        ),
        format!(
            "HEX: {}",
            if uart.hex_rendered.is_empty() {
                "<none>"
            } else {
                &uart.hex_rendered
            }
        ),
    ];

    if let Some(note) = &uart.note {
        lines.push(String::new());
        lines.push(format!("Note: {note}"));
    }

    if !uart.frames.is_empty() {
        lines.push(String::new());
        lines.push("Frames:".to_owned());
        for frame in &uart.frames {
            lines.push(format!(
                "  sample {} byte=0x{:02x} ascii={} stop_ok={}",
                frame.start_sample,
                frame.data,
                frame
                    .ascii
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| ".".to_owned()),
                yes_no(frame.stop_ok)
            ));
        }
    }

    lines.join("\n")
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
