use eframe::egui::{self, Color32, Stroke};

const CONTOUR_DELTA_WARN: f32 = 0.2;

pub struct PitchView<'a> {
    pub reference: &'a [f32],
    pub learner: &'a [f32],
}

impl<'a> PitchView<'a> {
    pub fn show(self, ui: &mut egui::Ui) {
        if self.reference.is_empty() && self.learner.is_empty() {
            ui.label("Pitch contour unavailable");
            return;
        }
        let (min, max) = data_bounds(self.reference, self.learner);
        let size = egui::vec2(ui.available_width(), 160.0);
        let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
        draw_frame(&painter, response.rect);
        draw_series(
            &painter,
            response.rect,
            self.reference,
            min,
            max,
            Color32::from_rgb(80, 160, 255),
        );
        draw_series(
            &painter,
            response.rect,
            self.learner,
            min,
            max,
            Color32::from_rgb(250, 120, 120),
        );
        draw_contour_delta(
            &painter,
            response.rect,
            self.reference,
            self.learner,
            min,
            max,
        );
        response.on_hover_ui(|ui| {
            ui.label(format!(
                "Mean contour delta: {:.2}",
                average_contour_delta(self.reference, self.learner)
            ));
        });
    }
}

fn data_bounds(reference: &[f32], learner: &[f32]) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for value in reference.iter().chain(learner.iter()) {
        min = min.min(*value);
        max = max.max(*value);
    }
    if !min.is_finite() || !max.is_finite() {
        return (-1.0, 1.0);
    }
    if (max - min).abs() < 1e-3 {
        return (min - 1.0, max + 1.0);
    }
    (min, max)
}

fn draw_frame(painter: &egui::Painter, rect: egui::Rect) {
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::DARK_GRAY));
}

fn draw_series(
    painter: &egui::Painter,
    rect: egui::Rect,
    series: &[f32],
    min: f32,
    max: f32,
    color: Color32,
) {
    if series.is_empty() {
        return;
    }
    if series.len() == 1 {
        draw_point(painter, rect, series[0], min, max, color);
        return;
    }
    let range = (max - min).max(1e-6);
    let last_index = (series.len() - 1) as f32;
    let mut points = Vec::with_capacity(series.len());
    for (index, value) in series.iter().enumerate() {
        let x_ratio = index as f32 / last_index;
        let y_ratio = (value - min) / range;
        let pos = egui::pos2(
            rect.left() + x_ratio * rect.width(),
            rect.bottom() - y_ratio * rect.height(),
        );
        points.push(pos);
    }
    painter.add(egui::epaint::PathShape::line(
        points,
        Stroke::new(2.0, color),
    ));
}

fn draw_point(
    painter: &egui::Painter,
    rect: egui::Rect,
    value: f32,
    min: f32,
    max: f32,
    color: Color32,
) {
    let range = (max - min).max(1e-6);
    let y_ratio = (value - min) / range;
    let pos = egui::pos2(rect.center().x, rect.bottom() - y_ratio * rect.height());
    painter.circle_filled(pos, 4.0, color);
}

fn draw_contour_delta(
    painter: &egui::Painter,
    rect: egui::Rect,
    reference: &[f32],
    learner: &[f32],
    min: f32,
    max: f32,
) {
    let length = reference.len().min(learner.len());
    if length < 2 {
        return;
    }
    let range = (max - min).max(1e-6);
    let last_index = (length - 1) as f32;
    for idx in 0..length {
        let diff = (reference[idx] - learner[idx]).abs();
        if diff < CONTOUR_DELTA_WARN {
            continue;
        }
        let x_ratio = idx as f32 / last_index;
        let x = rect.left() + x_ratio * rect.width();
        let height = (diff / range).clamp(0.0, 1.0) * rect.height();
        let bar = egui::Rect::from_min_max(
            egui::pos2(x - 1.0, rect.bottom() - height),
            egui::pos2(x + 1.0, rect.bottom()),
        );
        painter.rect_filled(
            bar,
            0.0,
            Color32::from_rgba_premultiplied(255, 110, 110, 120),
        );
    }
}

fn average_contour_delta(reference: &[f32], learner: &[f32]) -> f32 {
    let length = reference.len().min(learner.len());
    if length == 0 {
        return 0.0;
    }
    let total: f32 = reference
        .iter()
        .zip(learner.iter())
        .map(|(a, b)| (a - b).abs())
        .sum();
    total / length as f32
}
