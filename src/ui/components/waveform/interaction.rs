use eframe::egui;

use super::super::range_selection::{
    fraction_to_time, validate_selection, RangeSelection, SelectionOutput,
    MAX_SELECTION_DURATION_SECS,
};

pub fn update_existing_selection(
    ui: &mut egui::Ui,
    response: &egui::Response,
    rect: &egui::Rect,
    selection: &mut RangeSelection,
    id: &str,
    total_duration: f64,
) -> SelectionOutput {
    let mut temp_selection = *selection;
    let mut output =
        handle_selection_interaction(ui, response, rect, &mut temp_selection, id, total_duration);
    if output.changed {
        *selection = temp_selection;
        output.selection = Some(temp_selection);
        validate_and_set_error(&mut output, temp_selection);
    }
    output
}

pub fn create_new_selection_on_click(
    response: &egui::Response,
    rect: &egui::Rect,
    total_duration: f64,
) -> SelectionOutput {
    let Some(pointer_pos) = response.interact_pointer_pos() else {
        return SelectionOutput::default();
    };
    let pointer_x = pointer_pos.x;
    if pointer_x < rect.left() || pointer_x > rect.right() {
        return SelectionOutput::default();
    }
    let width = rect.width();
    let new_frac = (pointer_x - rect.left()) / width;
    let new_time = fraction_to_time(new_frac, total_duration);
    let new_selection = RangeSelection {
        start_sec: new_time,
        end_sec: new_time,
    };
    let mut output = SelectionOutput {
        changed: true,
        selection: Some(new_selection),
        ..SelectionOutput::default()
    };
    validate_and_set_error(&mut output, new_selection);
    output
}

fn validate_and_set_error(output: &mut SelectionOutput, selection: RangeSelection) {
    if let Err(validation_error) = validate_selection(
        selection.start_sec,
        selection.end_sec,
        MAX_SELECTION_DURATION_SECS,
    ) {
        output.validation_error = Some(validation_error);
    }
}

pub fn handle_selection_interaction(
    ui: &mut egui::Ui,
    response: &egui::Response,
    rect: &egui::Rect,
    selection: &mut RangeSelection,
    id: &str,
    total_duration: f64,
) -> SelectionOutput {
    let mut output = SelectionOutput::default();
    let Some(pointer_pos) = response.interact_pointer_pos() else {
        return output;
    };
    if response.drag_started() {
        output = handle_drag_start(ui, rect, selection, pointer_pos.x, id, total_duration);
    } else if response.dragged() {
        output = handle_drag_continue(ui, rect, selection, pointer_pos.x, id, total_duration);
    }
    output
}

fn calculate_handle_positions(
    selection: &RangeSelection,
    rect: &egui::Rect,
    total_duration: f64,
) -> (f32, f32) {
    use super::super::range_selection::time_to_fraction;
    let width = rect.width();
    let start_frac = time_to_fraction(selection.start_sec, total_duration);
    let end_frac = time_to_fraction(selection.end_sec, total_duration);
    let start_x = rect.left() + start_frac * width;
    let end_x = rect.left() + end_frac * width;
    (start_x, end_x)
}

fn handle_drag_start(
    ui: &mut egui::Ui,
    rect: &egui::Rect,
    selection: &mut RangeSelection,
    pointer_x: f32,
    id: &str,
    total_duration: f64,
) -> SelectionOutput {
    let mut output = SelectionOutput::default();
    let (start_x, end_x) = calculate_handle_positions(selection, rect, total_duration);
    let hover_radius = 8.0;
    let dist_to_start = (pointer_x - start_x).abs();
    let dist_to_end = (pointer_x - end_x).abs();
    if dist_to_start < hover_radius {
        store_drag_handle(ui, id, true);
    } else if dist_to_end < hover_radius {
        store_drag_handle(ui, id, false);
    } else if pointer_x >= rect.left() && pointer_x <= rect.right() {
        reset_selection_to_click(selection, pointer_x, rect, total_duration);
        output.changed = true;
    }
    output
}

fn handle_drag_continue(
    ui: &mut egui::Ui,
    rect: &egui::Rect,
    selection: &mut RangeSelection,
    pointer_x: f32,
    id: &str,
    total_duration: f64,
) -> SelectionOutput {
    let mut output = SelectionOutput::default();
    let Some(is_start) = get_drag_handle(ui, id) else {
        return output;
    };
    let width = rect.width();
    let new_frac = ((pointer_x - rect.left()) / width).clamp(0.0, 1.0);
    let new_time = fraction_to_time(new_frac, total_duration);
    if is_start {
        selection.start_sec = new_time.min(selection.end_sec);
    } else {
        selection.end_sec = new_time.max(selection.start_sec);
    }
    output.changed = true;
    output
}

fn store_drag_handle(ui: &mut egui::Ui, id: &str, is_start: bool) {
    ui.memory_mut(|mem| {
        mem.data
            .insert_temp(egui::Id::new((id, "drag_handle")), is_start);
    });
}

fn get_drag_handle(ui: &mut egui::Ui, id: &str) -> Option<bool> {
    ui.memory(|mem| {
        mem.data
            .get_temp::<bool>(egui::Id::new((id, "drag_handle")))
    })
}

fn reset_selection_to_click(
    selection: &mut RangeSelection,
    pointer_x: f32,
    rect: &egui::Rect,
    total_duration: f64,
) {
    let width = rect.width();
    let new_frac = (pointer_x - rect.left()) / width;
    let new_time = fraction_to_time(new_frac, total_duration);
    selection.start_sec = new_time;
    selection.end_sec = new_time;
}
