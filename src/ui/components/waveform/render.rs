use eframe::egui;

pub fn allocate_painter(
    ui: &mut egui::Ui,
    enable_selection: bool,
) -> (egui::Response, egui::Painter) {
    let size = egui::vec2(ui.available_width(), 140.0);
    let sense = if enable_selection {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::hover()
    };
    ui.allocate_painter(size, sense)
}

pub fn draw_waveform_line(samples: &[f32], rect: &egui::Rect, painter: &egui::Painter) {
    let points = build_waveform_points(samples, rect);
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, egui::Color32::LIGHT_BLUE),
    ));
}

pub fn build_waveform_points(samples: &[f32], rect: &egui::Rect) -> Vec<egui::Pos2> {
    let mut points = Vec::with_capacity(samples.len());
    let width = rect.width();
    let height = rect.height();
    for (index, &value) in samples.iter().enumerate() {
        let fraction = index as f32 / (samples.len() - 1) as f32;
        let x = rect.left() + fraction * width;
        let y = rect.center().y - value * height * 0.45;
        points.push(egui::pos2(x, y));
    }
    points
}
