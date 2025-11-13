use eframe::egui;

use super::range_selection::{RangeSelection, SelectionOutput};

mod interaction;
mod render;
mod selection_render;

use render::{allocate_painter, draw_waveform_line};
use selection_render::render_selection_if_exists;

pub struct WaveformView<'a> {
    pub id: &'a str,
    pub samples: &'a [f32],
    pub selection: Option<&'a mut RangeSelection>,
    pub total_duration: f64,
    pub enable_selection: bool,
}

impl<'a> WaveformView<'a> {
    pub fn show(&mut self, ui: &mut egui::Ui) -> SelectionOutput {
        if self.samples.is_empty() {
            ui.label("Waveform unavailable");
            return SelectionOutput::default();
        }
        let (response, painter) = allocate_painter(ui, self.enable_selection);
        let rect = response.rect;
        if self.samples.len() < 2 {
            return SelectionOutput::default();
        }
        draw_waveform_line(self.samples, &rect, &painter);
        let output = self.process_selection_interactions(ui, &response, &rect);
        render_selection_if_exists(self, &output, &rect, &painter);
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::DARK_GRAY));
        output
    }

    fn process_selection_interactions(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: &egui::Rect,
    ) -> SelectionOutput {
        if !self.enable_selection {
            return SelectionOutput::default();
        }
        let current_selection = self.selection.as_ref().map(|s| **s);
        let id = self.id;
        let total_duration = self.total_duration;
        if let Some(selection) = self.selection.as_mut() {
            interaction::update_existing_selection(
                ui,
                response,
                rect,
                selection,
                id,
                total_duration,
            )
        } else if response.clicked() {
            self.create_new_selection_on_click(response, rect)
        } else {
            SelectionOutput {
                selection: current_selection,
                ..SelectionOutput::default()
            }
        }
    }

    fn create_new_selection_on_click(
        &self,
        response: &egui::Response,
        rect: &egui::Rect,
    ) -> SelectionOutput {
        interaction::create_new_selection_on_click(response, rect, self.total_duration)
    }
}
