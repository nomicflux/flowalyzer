use eframe::egui;

use super::super::range_selection::{
    time_to_fraction, validate_selection, RangeSelection, SelectionError, SelectionOutput,
    MAX_SELECTION_DURATION_SECS,
};

pub fn render_selection_if_exists(
    waveform_view: &super::WaveformView,
    output: &SelectionOutput,
    rect: &egui::Rect,
    painter: &egui::Painter,
) {
    let sel = output
        .selection
        .or_else(|| waveform_view.selection.as_ref().map(|s| **s));
    let Some(sel) = sel else {
        return;
    };
    let validation_err = output.validation_error.or_else(|| {
        validate_selection(sel.start_sec, sel.end_sec, MAX_SELECTION_DURATION_SECS).err()
    });
    render_selection(
        sel,
        rect,
        painter,
        validation_err,
        waveform_view.total_duration,
    );
}

fn render_selection(
    selection: RangeSelection,
    rect: &egui::Rect,
    painter: &egui::Painter,
    validation_error: Option<SelectionError>,
    total_duration: f64,
) {
    let (start_x, end_x) = calculate_selection_x_positions(selection, rect, total_duration);
    let selection_rect = build_selection_rect(start_x, end_x, rect);
    draw_selection_overlay(painter, &selection_rect);
    if validation_error.is_some() {
        draw_selection_error_border(painter, &selection_rect);
    }
    draw_selection_handles(painter, start_x, end_x, rect);
}

fn calculate_selection_x_positions(
    selection: RangeSelection,
    rect: &egui::Rect,
    total_duration: f64,
) -> (f32, f32) {
    let width = rect.width();
    let start_frac = time_to_fraction(selection.start_sec, total_duration);
    let end_frac = time_to_fraction(selection.end_sec, total_duration);
    let start_x = rect.left() + start_frac * width;
    let end_x = rect.left() + end_frac * width;
    (start_x, end_x)
}

fn build_selection_rect(start_x: f32, end_x: f32, rect: &egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(start_x, rect.top()),
        egui::pos2(end_x, rect.bottom()),
    )
}

fn draw_selection_overlay(painter: &egui::Painter, selection_rect: &egui::Rect) {
    painter.rect_filled(
        *selection_rect,
        0.0,
        egui::Color32::from_rgba_unmultiplied(100, 150, 255, 60),
    );
}

fn draw_selection_error_border(painter: &egui::Painter, selection_rect: &egui::Rect) {
    painter.rect_stroke(
        *selection_rect,
        0.0,
        egui::Stroke::new(2.0, egui::Color32::RED),
    );
}

fn draw_selection_handles(painter: &egui::Painter, start_x: f32, end_x: f32, rect: &egui::Rect) {
    let handle_radius = 6.0;
    let center_y = rect.center().y;
    painter.circle_filled(
        egui::pos2(start_x, center_y),
        handle_radius,
        egui::Color32::LIGHT_BLUE,
    );
    painter.circle_filled(
        egui::pos2(end_x, center_y),
        handle_radius,
        egui::Color32::LIGHT_BLUE,
    );
}
