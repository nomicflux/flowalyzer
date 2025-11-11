use eframe::egui;

pub struct WaveformView<'a> {
    pub id: &'a str,
    pub samples: &'a [f32],
}

impl<'a> WaveformView<'a> {
    pub fn show(&self, ui: &mut egui::Ui) {
        if self.samples.is_empty() {
            ui.label("Waveform unavailable");
            return;
        }
        let size = egui::vec2(ui.available_width(), 140.0);
        let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());
        let rect = response.rect;
        if self.samples.len() < 2 {
            return;
        }
        let mut points = Vec::with_capacity(self.samples.len());
        let width = rect.width();
        let height = rect.height();
        for (index, &value) in self.samples.iter().enumerate() {
            let fraction = index as f32 / (self.samples.len() - 1) as f32;
            let x = rect.left() + fraction * width;
            let y = rect.center().y - value * height * 0.45;
            points.push(egui::pos2(x, y));
        }
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
        ));
        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::DARK_GRAY));
    }
}
