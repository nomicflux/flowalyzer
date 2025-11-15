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
                // Show overall summary first
                let (avg_similarity, avg_contour) = calculate_averages(data);
                let overall_match = (avg_similarity + avg_contour) / 2.0;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Overall match: {:.0}%", (overall_match * 100.0)))
                        .size(16.0)
                        .color(color_for_value(overall_match)));
                    ui.separator();
                    ui.label(format!("Pronunciation: {:.0}%", (avg_similarity * 100.0)));
                    ui.label(format!("Pitch: {:.0}%", (avg_contour * 100.0)));
                });
                
                // Show simplified visualization - aggregate into fewer, larger segments
                paint_spectrogram_simplified(ui, data);
                
                // Show legend
                ui.horizontal(|ui| {
                    ui.label("Match quality:");
                    ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(0, 0, 255)));
                    ui.label("Excellent");
                    ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(0, 255, 0)));
                    ui.label("Good");
                    ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(255, 255, 0)));
                    ui.label("Medium");
                    ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(255, 0, 0)));
                    ui.label("Poor");
                });
            }
        } else {
            ui.label("Spectrogram unavailable");
        }
    }
}

fn calculate_averages(data: &SpectrogramData) -> (f32, f32) {
    if data.rows < 2 || data.cols == 0 {
        return (0.0, 0.0);
    }
    let mut similarity_sum = 0.0;
    let mut contour_sum = 0.0;
    let mut similarity_count = 0;
    let mut contour_count = 0;
    
    for col in 0..data.cols {
        let sim = data.value(0, col);
        let cont = data.value(1, col);
        if sim > 0.0 {
            similarity_sum += sim;
            similarity_count += 1;
        }
        if cont > 0.0 {
            contour_sum += cont;
            contour_count += 1;
        }
    }
    
    let avg_sim = if similarity_count > 0 { similarity_sum / similarity_count as f32 } else { 0.0 };
    let avg_cont = if contour_count > 0 { contour_sum / contour_count as f32 } else { 0.0 };
    (avg_sim, avg_cont)
}

fn paint_spectrogram_simplified(ui: &mut egui::Ui, data: &SpectrogramData) {
    // Aggregate into fewer segments for clarity - show ~20 segments max
    const MAX_SEGMENTS: usize = 20;
    let segments = data.cols.min(MAX_SEGMENTS);
    let samples_per_segment = (data.cols as f32 / segments as f32).ceil() as usize;
    
    let label_width = 100.0;
    let heatmap_width = ui.available_width() - label_width;
    let size = egui::vec2(heatmap_width, 120.0);
    let (response, painter) = ui.allocate_painter(size, egui::Sense::hover());

    // Draw labels
    let cell_h = size.y / data.rows as f32;
    let labels = ["Pronunciation", "Pitch"];
    for (i, label) in labels.iter().enumerate() {
        if i < data.rows {
            let y = response.rect.min.y + (i as f32 + 0.5) * cell_h;
            painter.text(
                egui::pos2(response.rect.min.x - label_width + 5.0, y),
                egui::Align2::LEFT_CENTER,
                *label,
                egui::FontId::default(),
                egui::Color32::WHITE,
            );
        }
    }

    // Draw aggregated heatmap
    let cell_w = response.rect.width() / segments as f32;
    for row in 0..data.rows {
        for seg in 0..segments {
            // Average values in this segment
            let start_col = seg * samples_per_segment;
            let end_col = ((seg + 1) * samples_per_segment).min(data.cols);
            let mut sum = 0.0;
            let mut count = 0;
            for col in start_col..end_col {
                let val = data.value(row, col);
                if val > 0.0 {
                    sum += val;
                    count += 1;
                }
            }
            let avg_value = if count > 0 { sum / count as f32 } else { 0.0 };
            
            let color = color_for_value(avg_value);
            let pos = response.rect.min + egui::vec2(seg as f32 * cell_w, row as f32 * cell_h);
            let cell = egui::Rect::from_min_size(pos, egui::vec2(cell_w, cell_h));
            painter.rect_filled(cell, 0.0, color);
        }
    }
}


fn color_for_value(value: f32) -> egui::Color32 {
    // Color scheme: Blue (excellent) -> Green (good) -> Yellow (medium) -> Red (poor)
    // Value range: 0.0 (poor) to 1.0 (excellent match)
    let clamped = value.clamp(0.0, 1.0);
    if clamped >= 0.8 {
        // Blue: Excellent match (0.8-1.0)
        let t = (clamped - 0.8) / 0.2; // 0.8->0.0, 1.0->1.0
        egui::Color32::from_rgb(0, 0, (100.0 + 155.0 * t) as u8)
    } else if clamped >= 0.6 {
        // Green: Good match (0.6-0.8)
        let t = (clamped - 0.6) / 0.2; // 0.6->0.0, 0.8->1.0
        egui::Color32::from_rgb(0, (100.0 + 155.0 * t) as u8, 0)
    } else if clamped >= 0.4 {
        // Yellow: Medium match (0.4-0.6)
        let t = (clamped - 0.4) / 0.2; // 0.4->0.0, 0.6->1.0
        egui::Color32::from_rgb((255.0 * (1.0 - t)) as u8, 255, 0)
    } else {
        // Red: Poor match (0.0-0.4)
        let t = clamped / 0.4; // 0.0->0.0, 0.4->1.0
        egui::Color32::from_rgb(255, (100.0 * t) as u8, 0)
    }
}
