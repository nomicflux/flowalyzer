use eframe::egui;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, SessionController, SessionHandle, SessionSnapshot,
};
use crate::ui::components::phoneme_timeline::PhonemeTimeline;
use crate::ui::components::pitch::PitchView;
use crate::ui::components::spectrogram::{SpectrogramData, SpectrogramView};
use crate::ui::components::waveform::WaveformView;

pub struct SessionApp {
    handle: SessionHandle,
    controller: SessionController,
    snapshot: SessionSnapshot,
    selected_phoneme: Option<usize>,
    reference_waveform: Vec<f32>,
    learner_waveform: Vec<f32>,
    spectrogram: Option<SpectrogramData>,
    reference_pitch: Vec<f32>,
    learner_pitch: Vec<f32>,
}

impl SessionApp {
    pub fn new(handle: SessionHandle) -> Self {
        let controller = handle.controller();
        let snapshot = handle.initial_snapshot();
        let mut app = Self {
            handle,
            controller,
            snapshot,
            selected_phoneme: None,
            reference_waveform: Vec::new(),
            learner_waveform: Vec::new(),
            spectrogram: None,
            reference_pitch: Vec::new(),
            learner_pitch: Vec::new(),
        };
        app.sync_visuals();
        app
    }

    fn sync_visuals(&mut self) {
        self.refresh_selection();
        self.reference_waveform = to_waveform(&self.snapshot.alignment.reference_energy);
        self.learner_waveform = to_waveform(&self.snapshot.alignment.learner_energy);
        self.spectrogram = build_spectrogram(&self.snapshot.alignment);
        self.reference_pitch = self.snapshot.alignment.reference_pitch.clone();
        self.learner_pitch = self.snapshot.alignment.learner_pitch.clone();
    }

    fn refresh_selection(&mut self) {
        let count = self.snapshot.alignment.phonemes.len();
        self.selected_phoneme = match (count, self.selected_phoneme) {
            (0, _) => None,
            (_, Some(index)) if index < count => Some(index),
            _ => Some(0),
        };
    }

    fn poll_updates(&mut self) {
        while let Some(update) = self.handle.try_recv() {
            self.snapshot = update;
            self.sync_visuals();
        }
    }

    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.control_buttons(ui);
                ui.separator();
                self.show_scores(ui);
            });
            ui.label(format!("Latency: {:.1} ms", self.snapshot.latency_ms));
            self.error_banner(ui);
        });
    }

    fn control_buttons(&mut self, ui: &mut egui::Ui) {
        let label = if self.snapshot.recording {
            "Stop Recording"
        } else {
            "Start Recording"
        };
        if ui.button(label).clicked() {
            let action = if self.snapshot.recording {
                self.controller.stop()
            } else {
                self.controller.start()
            };
            if let Err(err) = action {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), err.to_string());
            }
        }
    }

    fn show_scores(&self, ui: &mut egui::Ui) {
        let scores = &self.snapshot.scores;
        ui.label(format!("Overall: {:.2}", scores.overall));
        ui.label(format!("Timing: {:.2}", scores.timing));
        ui.label(format!("Articulation: {:.2}", scores.articulation));
        ui.label(format!("Intonation: {:.2}", scores.intonation));
    }

    fn error_banner(&self, ui: &mut egui::Ui) {
        if let Some(message) = &self.snapshot.error {
            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), message);
        }
    }

    fn show_timeline(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("timeline")
            .resizable(false)
            .show(ctx, |ui| {
                PhonemeTimeline {
                    alignment: &self.snapshot.alignment,
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
            self.show_pitch(ui);
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

    fn show_pitch(&self, ui: &mut egui::Ui) {
        PitchView {
            reference: &self.reference_pitch,
            learner: &self.learner_pitch,
        }
        .show(ui);
    }

    fn selected_phoneme(&self) -> Option<&AlignedPhoneme> {
        self.selected_phoneme
            .and_then(|index| self.snapshot.alignment.phonemes.get(index))
    }
}

impl eframe::App for SessionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_updates();
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
    ui.label(format!(
        "Contour similarity: {:.2}",
        phoneme.contour_similarity
    ));
}

fn build_spectrogram(alignment: &AlignmentReport) -> Option<SpectrogramData> {
    let source = if alignment.contour_band.is_empty() {
        &alignment.similarity_band
    } else {
        &alignment.contour_band
    };
    let rows = source.len();
    if rows == 0 {
        return None;
    }
    let cols = 64;
    let mut values = Vec::with_capacity(rows * cols);
    for band in source {
        let base = band.clamp(0.0, 1.0);
        for col in 0..cols {
            let ratio = col as f32 / cols as f32;
            let emphasis = 1.0 - (ratio - 0.5).abs() * 2.0;
            values.push((base * emphasis.max(0.0)).clamp(0.0, 1.0));
        }
    }
    Some(SpectrogramData::new(rows, cols, values))
}

fn to_waveform(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let peak = samples
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f32, f32::max)
        .max(1e-6);
    samples
        .iter()
        .map(|value| (value / peak).clamp(-1.0, 1.0))
        .collect()
}
