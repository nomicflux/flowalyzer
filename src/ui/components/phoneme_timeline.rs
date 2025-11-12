use eframe::egui::{self, Color32, RichText};

use crate::pronunciation::{AlignedPhoneme, AlignmentReport};

const SIMILARITY_WARN: f32 = 0.75;
const CONTOUR_WARN: f32 = 0.75;
const ARTICULATION_WARN: f32 = 0.4;

pub struct PhonemeTimeline<'a> {
    pub alignment: &'a AlignmentReport,
    pub selected: &'a mut Option<usize>,
}

impl<'a> PhonemeTimeline<'a> {
    pub fn show(self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            for (index, phoneme) in self.alignment.phonemes.iter().enumerate() {
                let selected = self.selected.is_some_and(|value| value == index);
                let response = ui.selectable_label(selected, entry_label(phoneme));
                if response.clicked() {
                    *self.selected = Some(index);
                }
                response.on_hover_ui(|ui| timeline_tooltip(ui, phoneme));
            }
        });
    }
}

fn entry_label(phoneme: &AlignedPhoneme) -> RichText {
    let text = format!(
        "{} ({:+.0} ms, C {:.2})",
        phoneme.symbol, phoneme.timing_delta_ms, phoneme.contour_similarity
    );
    if needs_attention(phoneme) {
        RichText::new(text).color(Color32::from_rgb(200, 60, 60))
    } else {
        RichText::new(text)
    }
}

pub(crate) fn needs_attention(phoneme: &AlignedPhoneme) -> bool {
    phoneme.similarity < SIMILARITY_WARN
        || phoneme.contour_similarity < CONTOUR_WARN
        || phoneme.articulation_variance > ARTICULATION_WARN
}

fn timeline_tooltip(ui: &mut egui::Ui, phoneme: &AlignedPhoneme) {
    ui.label(format!("Timing Δ: {:+.1} ms", phoneme.timing_delta_ms));
    ui.label(format!("Similarity score: {:.2}", phoneme.similarity));
    ui.label(format!(
        "Articulation variance: {:.2}",
        phoneme.articulation_variance
    ));
    ui.label(format!(
        "Contour similarity: {:.2}",
        phoneme.contour_similarity
    ));
}
