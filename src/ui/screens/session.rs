use eframe::egui;

use crate::pronunciation::{AlignedPhoneme, AlignmentReport, PronunciationScores};
use crate::ui::components::control_panel::ControlPanel;
use crate::ui::components::phoneme_timeline::PhonemeTimeline;
use crate::ui::components::spectrogram::{SpectrogramData, SpectrogramView};
use crate::ui::components::waveform::WaveformView;

pub struct SessionApp {
    alignment: AlignmentReport,
    scores: PronunciationScores,
    selected_phoneme: Option<usize>,
    is_recording: bool,
    is_playing: bool,
    reference_waveform: Vec<f32>,
    learner_waveform: Vec<f32>,
    spectrogram: Option<SpectrogramData>,
}

impl SessionApp {
    pub fn new(alignment: AlignmentReport, scores: PronunciationScores) -> Self {
        let selected = if alignment.phonemes.is_empty() {
            None
        } else {
            Some(0)
        };
        Self {
            reference_waveform: generate_waveform(0),
            learner_waveform: generate_waveform(1),
            spectrogram: build_spectrogram(&alignment),
            alignment,
            scores,
            selected_phoneme: selected,
            is_recording: false,
            is_playing: false,
        }
    }

    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ControlPanel {
                is_recording: &mut self.is_recording,
                is_playing: &mut self.is_playing,
                scores: &self.scores,
            }
            .show(ui);
        });
    }

    fn show_timeline(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("timeline")
            .resizable(false)
            .show(ctx, |ui| {
                PhonemeTimeline {
                    alignment: &self.alignment,
                    selected: &mut self.selected_phoneme,
                }
                .show(ui);
                if let Some(phoneme) = self.selected_phoneme() {
                    ui.separator();
                    show_phoneme_details(ui, phoneme);
                }
            });
    }

    fn show_main(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_waveforms(ui);
            ui.separator();
            SpectrogramView {
                data: self.spectrogram.as_ref(),
            }
            .show(ui);
        });
    }

    fn show_waveforms(&self, ui: &mut egui::Ui) {
        egui::Grid::new("waveforms").show(ui, |ui| {
            WaveformView {
                id: "reference_waveform",
                samples: &self.reference_waveform,
            }
            .show(ui);
            ui.end_row();
            WaveformView {
                id: "learner_waveform",
                samples: &self.learner_waveform,
            }
            .show(ui);
            ui.end_row();
        });
    }

    fn selected_phoneme(&self) -> Option<&AlignedPhoneme> {
        self.selected_phoneme
            .and_then(|index| self.alignment.phonemes.get(index))
    }
}

impl eframe::App for SessionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.show_top_panel(ctx);
        self.show_timeline(ctx);
        self.show_main(ctx);
    }
}

fn show_phoneme_details(ui: &mut egui::Ui, phoneme: &AlignedPhoneme) {
    ui.heading(&phoneme.symbol);
    ui.label(format!("Timing Δ: {:+.1} ms", phoneme.timing_delta_ms));
    ui.label(format!("Similarity: {:.2}", phoneme.similarity));
    ui.label(format!(
        "Articulation variance: {:.2}",
        phoneme.articulation_variance
    ));
}

fn generate_waveform(seed: u32) -> Vec<f32> {
    let len = 512;
    (0..len)
        .map(|index| {
            let t = index as f32 / len as f32;
            (t * 10.0 + seed as f32).sin() * 0.5 + (t * 30.0).cos() * 0.25
        })
        .collect()
}

fn build_spectrogram(alignment: &AlignmentReport) -> Option<SpectrogramData> {
    let rows = alignment.phonemes.len();
    if rows == 0 {
        return None;
    }
    let cols = 64;
    let mut values = Vec::with_capacity(rows * cols);
    for phoneme in &alignment.phonemes {
        let base = phoneme.similarity.clamp(0.0, 1.0);
        for col in 0..cols {
            let ratio = col as f32 / cols as f32;
            let emphasis = 1.0 - (ratio - 0.5).abs() * 2.0;
            values.push((base * emphasis.max(0.0)).clamp(0.0, 1.0));
        }
    }
    Some(SpectrogramData::new(rows, cols, values))
}
