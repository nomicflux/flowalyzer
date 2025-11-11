use eframe::egui;

use crate::pronunciation::PronunciationScores;

pub struct ControlPanel<'a> {
    pub is_recording: &'a mut bool,
    pub is_playing: &'a mut bool,
    pub scores: &'a PronunciationScores,
}

impl<'a> ControlPanel<'a> {
    pub fn show(self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            toggle_button(ui, "Record", self.is_recording);
            toggle_button(ui, "Play", self.is_playing);
            ui.separator();
            ui.label(format!("Overall: {:.2}", self.scores.overall));
            ui.label(format!("Timing: {:.2}", self.scores.timing));
            ui.label(format!("Articulation: {:.2}", self.scores.articulation));
            ui.label(format!("Intonation: {:.2}", self.scores.intonation));
        });
    }
}

fn toggle_button(ui: &mut egui::Ui, label: &str, state: &mut bool) {
    let text = if *state {
        format!("Stop {}", label)
    } else {
        format!("Start {}", label)
    };
    if ui.button(text).clicked() {
        *state = !*state;
    }
}
