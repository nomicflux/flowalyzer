use eframe::egui;

use crate::pronunciation::AlignmentReport;

pub struct PhonemeTimeline<'a> {
    pub alignment: &'a AlignmentReport,
    pub selected: &'a mut Option<usize>,
}

impl<'a> PhonemeTimeline<'a> {
    pub fn show(self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            for (index, phoneme) in self.alignment.phonemes.iter().enumerate() {
                let label = format!("{} ({:+.0} ms)", phoneme.symbol, phoneme.timing_delta_ms);
                let selected = self.selected.is_some_and(|value| value == index);
                if ui.selectable_label(selected, label).clicked() {
                    *self.selected = Some(index);
                }
            }
        });
    }
}
