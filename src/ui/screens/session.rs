use std::time::Duration;

use eframe::egui;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, Result as SessionResult, SessionController, SessionHandle,
    SessionSnapshot,
};
use crate::ui::components::control_strip::{ControlStrip, ControlStripOutput};
use crate::ui::components::phoneme_timeline::PhonemeTimeline;
use crate::ui::components::pitch::PitchView;
use crate::ui::components::spectrogram::{SpectrogramData, SpectrogramView};
use crate::ui::components::waveform::WaveformView;

const FRAME_WINDOW: usize = 400;
const SPECTROGRAM_COLS: usize = 64;
const FRAME_HOP_MS: f32 = 10.0;

pub struct SessionApp {
    handle: SessionHandle,
    controller: SessionController,
    snapshot: SessionSnapshot,
    latency_budget_ms: u32,
    control_error: Option<String>,
    selected_phoneme: Option<usize>,
    reference_waveform: Vec<f32>,
    learner_waveform: Vec<f32>,
    spectrogram: Option<SpectrogramData>,
    reference_pitch: Vec<f32>,
    learner_pitch: Vec<f32>,
}

impl SessionApp {
    pub fn new(handle: SessionHandle) -> Self {
        let latency_budget_ms = handle.config().latency_budget_ms;
        let controller = handle.controller();
        let snapshot = handle.initial_snapshot();
        let mut app = Self {
            handle,
            controller,
            snapshot,
            latency_budget_ms,
            control_error: None,
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
        self.reference_waveform = normalize_series(tail_slice(
            &self.snapshot.alignment.reference_energy,
            FRAME_WINDOW,
        ));
        self.learner_waveform = normalize_series(tail_slice(
            &self.snapshot.alignment.learner_energy,
            FRAME_WINDOW,
        ));
        self.reference_pitch = tail_slice(&self.snapshot.alignment.reference_pitch, FRAME_WINDOW);
        self.learner_pitch = tail_slice(&self.snapshot.alignment.learner_pitch, FRAME_WINDOW);
        self.spectrogram = build_spectrogram_window(&self.snapshot.alignment, FRAME_WINDOW);
    }

    fn refresh_selection(&mut self) {
        let count = self.snapshot.alignment.phonemes.len();
        self.selected_phoneme = match (count, self.selected_phoneme) {
            (0, _) => None,
            (_, Some(index)) if index < count => Some(index),
            _ => Some(0),
        };
    }

    fn poll_updates(&mut self, ctx: &egui::Context) {
        let mut changed = false;
        for update in self.handle.drain_snapshots() {
            self.snapshot = update;
            self.sync_visuals();
            changed = true;
        }
        if changed {
            ctx.request_repaint();
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn show_top_panel(&mut self, ctx: &egui::Context, actions: &mut ControlActions) {
        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let strip = ControlStrip {
                    is_recording: self.snapshot.recording,
                    latency_ms: self.snapshot.latency_ms,
                    latency_budget_ms: self.latency_budget_ms,
                };
                let output = strip.show(ui);
                actions.merge(output.into());
                ui.separator();
                self.show_scores(ui);
            });
            ui.label("Shortcuts: Space toggles recording · R replays the reference clip.");
            ui.label(self.playback_status());
            self.show_latency_guidance(ui);
            self.error_banner(ui);
            if let Some(message) = &self.control_error {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), message);
            }
        });
    }

    fn handle_shortcuts(&self, ctx: &egui::Context) -> ControlActions {
        let mut actions = ControlActions::default();
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Space) {
                actions.toggle_recording = true;
            }
            if input.key_pressed(egui::Key::R) {
                actions.replay_reference = true;
            }
        });
        actions
    }

    fn apply_actions(&mut self, actions: ControlActions) {
        if actions.toggle_recording {
            self.toggle_recording();
        }
        if actions.replay_reference {
            self.replay_reference();
        }
    }

    fn toggle_recording(&mut self) {
        let outcome = if self.snapshot.recording {
            self.controller.stop()
        } else {
            self.controller.start()
        };
        self.handle_control_result(outcome);
    }

    fn replay_reference(&mut self) {
        if self.snapshot.recording {
            if let Err(err) = self.controller.stop() {
                self.control_error = Some(err.to_string());
                return;
            }
        }
        self.handle_control_result(self.controller.start());
    }

    fn handle_control_result(&mut self, result: SessionResult<()>) {
        self.control_error = result.err().map(|err| err.to_string());
    }

    fn playback_status(&self) -> String {
        let current = self.playback_position_ms();
        let total = self.snapshot.alignment.total_duration.as_secs_f32() * 1000.0;
        if total <= 0.0 {
            return format!("Playback {:.0} ms", current);
        }
        format!("Playback {:.0} / {:.0} ms", current.min(total), total)
    }

    fn playback_position_ms(&self) -> f32 {
        self.snapshot.alignment.reference_energy.len() as f32 * FRAME_HOP_MS
    }

    fn show_latency_guidance(&self, ui: &mut egui::Ui) {
        let latency = self.snapshot.latency_ms;
        let budget = self.latency_budget_ms as f32;
        if latency <= budget * 0.75 {
            return;
        }
        let message = if latency > budget {
            "Latency exceeds budget. Increase the capture latency range or close other audio apps."
        } else {
            "Latency approaching budget. Consider widening the latency window or pausing background audio."
        };
        ui.colored_label(egui::Color32::from_rgb(210, 160, 20), message);
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
        self.poll_updates(ctx);
        let mut actions = self.handle_shortcuts(ctx);
        self.show_top_panel(ctx, &mut actions);
        self.apply_actions(actions);
        self.show_timeline(ctx);
        self.show_main(ctx);
    }
}

#[derive(Default)]
struct ControlActions {
    toggle_recording: bool,
    replay_reference: bool,
}

impl ControlActions {
    fn merge(&mut self, other: ControlActions) {
        self.toggle_recording |= other.toggle_recording;
        self.replay_reference |= other.replay_reference;
    }
}

impl From<ControlStripOutput> for ControlActions {
    fn from(output: ControlStripOutput) -> Self {
        ControlActions {
            toggle_recording: output.toggle_recording,
            replay_reference: output.replay_reference,
        }
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

fn build_spectrogram_window(
    alignment: &AlignmentReport,
    max_rows: usize,
) -> Option<SpectrogramData> {
    let source = spectrogram_source(alignment);
    if source.is_empty() {
        return None;
    }
    let window = tail_slice(source, max_rows);
    if window.is_empty() {
        return None;
    }
    Some(SpectrogramData::new(
        window.len(),
        SPECTROGRAM_COLS,
        build_spectrogram_values(&window),
    ))
}

fn spectrogram_source<'a>(alignment: &'a AlignmentReport) -> &'a [f32] {
    if alignment.contour_band.is_empty() {
        &alignment.similarity_band
    } else {
        &alignment.contour_band
    }
}

fn build_spectrogram_values(window: &[f32]) -> Vec<f32> {
    let mut values = Vec::with_capacity(window.len() * SPECTROGRAM_COLS);
    for &band in window {
        push_spectrogram_row(band, &mut values);
    }
    values
}

fn push_spectrogram_row(band: f32, values: &mut Vec<f32>) {
    let clamped = band.clamp(0.0, 1.0);
    for col in 0..SPECTROGRAM_COLS {
        let ratio = col as f32 / SPECTROGRAM_COLS as f32;
        let emphasis = 1.0 - (ratio - 0.5).abs() * 2.0;
        values.push((clamped * emphasis.max(0.0)).clamp(0.0, 1.0));
    }
}

fn tail_slice(series: &[f32], max_len: usize) -> Vec<f32> {
    if max_len == 0 || series.is_empty() {
        return Vec::new();
    }
    let start = series.len().saturating_sub(max_len);
    series[start..].to_vec()
}

fn normalize_series(mut samples: Vec<f32>) -> Vec<f32> {
    if samples.is_empty() {
        return samples;
    }
    let peak = samples
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f32, f32::max)
        .max(1e-6);
    for value in &mut samples {
        *value = (*value / peak).clamp(-1.0, 1.0);
    }
    samples
}
