use eframe::egui;

pub struct SpectrogramData {
    pub rows: usize,
    pub cols: usize,
    values: Vec<f32>,
}

impl SpectrogramData {
    pub fn new(rows: usize, cols: usize, values: Vec<f32>) -> Self {
        Self { rows, cols, values }
    }

    pub fn value(&self, row: usize, col: usize) -> f32 {
        let index = row * self.cols + col;
        self.values.get(index).copied().unwrap_or(0.0)
    }

    pub fn is_empty(&self) -> bool {
        self.rows == 0 || self.cols == 0 || self.values.is_empty()
    }
}

pub struct SpectrogramView<'a> {
    pub data: Option<&'a SpectrogramData>,
}

impl<'a> SpectrogramView<'a> {
    pub fn show(self, ui: &mut egui::Ui) {
        if let Some(data) = self.data {
            if data.is_empty() {
                ui.label("Spectrogram unavailable");
            } else {
                paint_spectrogram(ui, data);
            }
        } else {
            ui.label("Spectrogram unavailable");
        }
    }
}

fn paint_spectrogram(ui: &mut egui::Ui, data: &SpectrogramData) {
    let size = egui::vec2(ui.available_width(), 200.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
    draw_heatmap(&painter, response.rect, data);
}

fn draw_heatmap(painter: &egui::Painter, rect: egui::Rect, data: &SpectrogramData) {
    let cell_w = rect.width() / data.cols as f32;
    let cell_h = rect.height() / data.rows as f32;
    for row in 0..data.rows {
        for col in 0..data.cols {
            let color = color_for_value(data.value(row, col));
            let pos = rect.min + egui::vec2(col as f32 * cell_w, row as f32 * cell_h);
            let cell = egui::Rect::from_min_size(pos, egui::vec2(cell_w, cell_h));
            painter.rect_filled(cell, 0.0, color);
        }
    }
}

fn color_for_value(value: f32) -> egui::Color32 {
    let clamped = value.clamp(0.0, 1.0);
    let hue = (1.0 - clamped) * 240.0;
    let hsv = egui::ecolor::Hsva::new(hue / 360.0, 0.8, clamped.max(0.2), 1.0);
    let rgba = hsv.to_rgba_unmultiplied();
    egui::Color32::from_rgba_premultiplied(
        (rgba[0] * 255.0) as u8,
        (rgba[1] * 255.0) as u8,
        (rgba[2] * 255.0) as u8,
        (rgba[3] * 255.0) as u8,
    )
}
