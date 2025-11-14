use std::time::Duration;

use eframe::egui;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, ClipVariant, InitializationStage, Result as SessionResult,
    SessionController, SessionHandle, SessionSnapshot,
};
use crate::types::{Recipe, RuntimeRecipe};
use crate::ui::components::control_strip::{ControlStrip, ControlStripOutput};
use crate::ui::components::phoneme_timeline::PhonemeTimeline;
use crate::ui::components::pitch::PitchView;
use crate::ui::components::range_selection::{RangeSelection, SelectionError};
use crate::ui::components::recipe_builder::{
    RecipeBuilder, RecipeBuilderOutput, RecipeBuilderState,
};
use crate::ui::components::spectrogram::{SpectrogramData, SpectrogramView};
use crate::ui::components::waveform::WaveformView;

const FRAME_WINDOW: usize = 400;
const SPECTROGRAM_COLS: usize = 64;
const FRAME_HOP_MS: f32 = 10.0;
const RECIPE_SUCCESS_DISPLAY_SECS: u64 = 3;

/// Session state lifecycle:
/// - State fields (range_selection, recipe_builder_state, staged_recipe) initialize to None
/// - User creates range selection → recipe_builder_state auto-populates
/// - User builds recipe and clicks Apply → staged_recipe populated
/// - User clicks Clear OR range_selection cleared → all three fields reset to None
/// - App restart (SessionApp::new()) → all state reinitializes to None
enum RecipeStatus {
    Applying,
    Success { shown_at: std::time::Instant },
}

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
    range_selection: Option<RangeSelection>,
    selection_error: Option<SelectionError>,
    recipe_builder_state: Option<RecipeBuilderState>,
    staged_recipe: Option<RuntimeRecipe>,
    recipe_status: Option<RecipeStatus>,
    previous_clip_variant: ClipVariant,
}

impl SessionApp {
    pub fn new(handle: SessionHandle) -> Self {
        let latency_budget_ms = handle.config().latency_budget_ms;
        let controller = handle.controller();
        let snapshot = handle.initial_snapshot();
        let previous_clip_variant = snapshot.active_clip_variant;
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
            range_selection: None,
            selection_error: None,
            recipe_builder_state: None,
            staged_recipe: None,
            recipe_status: None,
            previous_clip_variant,
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
            let new_variant = update.active_clip_variant;
            if new_variant == ClipVariant::Flowalyzed
                && self.previous_clip_variant == ClipVariant::Original
                && matches!(self.recipe_status, Some(RecipeStatus::Applying))
            {
                self.recipe_status = Some(RecipeStatus::Success {
                    shown_at: std::time::Instant::now(),
                });
            }
            self.previous_clip_variant = new_variant;
            self.snapshot = update;
            self.sync_visuals();
            changed = true;
        }
        if let Some(RecipeStatus::Success { shown_at }) = self.recipe_status {
            if shown_at.elapsed().as_secs() >= RECIPE_SUCCESS_DISPLAY_SECS {
                self.recipe_status = None;
            }
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
                    reference_playing: self.snapshot.reference_playing,
                    latency_ms: self.snapshot.latency_ms,
                    latency_budget_ms: self.latency_budget_ms,
                    active_clip_variant: self.snapshot.active_clip_variant,
                    has_flowalyzed_clip: self.snapshot.active_clip_variant
                        == ClipVariant::Flowalyzed,
                };
                let output = strip.show(ui);
                actions.merge(output.into());
                ui.separator();
                self.show_scores(ui);
            });
            if self.snapshot.initializing {
                self.show_initialization_progress(ui);
            } else {
                ui.label("Shortcuts: Space toggles recording · R replays the reference clip.");
                ui.label(self.playback_status());
                self.show_latency_guidance(ui);
            }
            self.error_banner(ui);
            if let Some(message) = &self.control_error {
                ui.colored_label(egui::Color32::from_rgb(200, 60, 60), message);
            }
            self.show_selection_info(ui);
            self.show_recipe_summary(ui);
            self.show_clip_metadata(ui);
            self.show_recipe_status(ui);
        });
    }

    fn show_selection_info(&self, ui: &mut egui::Ui) {
        if let Some(sel) = self.range_selection {
            let span = sel.end_sec - sel.start_sec;
            ui.label(format!(
                "Selection: {:.2}s - {:.2}s (span: {:.2}s)",
                sel.start_sec, sel.end_sec, span
            ));
        }
        if let Some(err) = self.selection_error {
            let message = match err {
                SelectionError::ExceedsMaxDuration => "Selection span exceeds 5-minute maximum",
                SelectionError::InvalidRange => "Invalid selection range",
            };
            ui.colored_label(egui::Color32::from_rgb(200, 60, 60), message);
        }
    }

    fn show_recipe_summary(&self, ui: &mut egui::Ui) {
        if let (Some(recipe), Some(sel)) = (&self.staged_recipe, self.range_selection) {
            let summary = format_recipe_summary(recipe);
            let range_text = format!("Will apply to {:.2}s - {:.2}s", sel.start_sec, sel.end_sec);
            ui.colored_label(
                egui::Color32::from_rgb(30, 180, 80),
                format!("{} | {}", summary, range_text),
            );
        }
    }

    fn show_clip_metadata(&self, ui: &mut egui::Ui) {
        let (message, color) = match self.snapshot.active_clip_variant {
            ClipVariant::Original => (
                "Original clip is active",
                egui::Color32::from_rgb(180, 180, 180),
            ),
            ClipVariant::Flowalyzed => (
                "Flowalyzed clip is active",
                egui::Color32::from_rgb(100, 150, 255),
            ),
        };
        ui.colored_label(color, message);
    }

    fn show_recipe_status(&self, ui: &mut egui::Ui) {
        if let Some(status) = &self.recipe_status {
            match status {
                RecipeStatus::Applying => {
                    ui.colored_label(egui::Color32::from_rgb(210, 160, 20), "Applying recipe...");
                }
                RecipeStatus::Success { .. } => {
                    ui.colored_label(
                        egui::Color32::from_rgb(30, 180, 80),
                        "Recipe applied successfully",
                    );
                }
            }
        }
    }

    fn show_initialization_progress(&self, ui: &mut egui::Ui) {
        if let Some(progress) = &self.snapshot.init_progress {
            let stage_number = progress.stage.order() + 1;
            ui.colored_label(
                egui::Color32::from_rgb(210, 160, 20),
                format!(
                    "Initializing engine… Stage {}/{}: {}",
                    stage_number,
                    progress.total_steps,
                    progress.stage.label()
                ),
            );
            ui.add_space(4.0);
            for stage in InitializationStage::ordered().iter() {
                let marker = if stage.order() < progress.completed_steps as usize {
                    "✓"
                } else if *stage == progress.stage {
                    "…"
                } else {
                    "•"
                };
                ui.label(format!("{} {}", marker, stage.label()));
            }
            if progress.sub_stage_total > 0 {
                let current = progress
                    .sub_stage_index
                    .max(1)
                    .min(progress.sub_stage_total)
                    .max(1);
                if let Some(label) = &progress.sub_stage_label {
                    ui.label(format!(
                        "    ↳ {} (step {}/{})",
                        label, current, progress.sub_stage_total
                    ));
                }
            }
            if let (Some(label), Some(curr)) = (&progress.metric_label, progress.current_value) {
                if let Some(total) = progress.total_value {
                    ui.label(format!("    ↳ {} {} / {}", label, curr, total));
                }
                if let Some(secs) = progress.elapsed_secs {
                    ui.label(format!("       {}s elapsed", secs));
                }
            }
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(210, 160, 20),
                "Initializing engine... Please wait.",
            );
        }
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
        if actions.stop_replay {
            self.stop_replay();
        }
        if let Some(variant) = actions.toggle_to_variant {
            if let Err(err) = self.controller.toggle_clip_variant(variant) {
                self.control_error = Some(format!("Toggle error: {}", err));
            }
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
        // Replay reference works whether recording is active or not
        // The player is kept alive at the EngineRunner level
        self.handle_control_result(self.controller.replay_reference());
    }

    fn stop_replay(&mut self) {
        self.handle_control_result(self.controller.stop_replay());
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

    fn show_waveforms(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("waveforms").show(ui, |ui| {
            let total_duration = self.snapshot.alignment.total_duration.as_secs_f64();
            let mut waveform_view = WaveformView {
                id: "reference_waveform",
                samples: &self.reference_waveform,
                selection: self.range_selection.as_mut(),
                total_duration,
                enable_selection: !self.snapshot.recording,
            };
            let output = waveform_view.show(ui);

            if output.changed {
                self.range_selection = output.selection;
                self.selection_error = output.validation_error;
                self.update_recipe_builder_visibility();
            }

            ui.end_row();
            WaveformView {
                id: "learner_waveform",
                samples: &self.learner_waveform,
                selection: None,
                total_duration,
                enable_selection: false,
            }
            .show(ui);
            ui.end_row();
        });
    }

    fn update_recipe_builder_visibility(&mut self) {
        match self.range_selection {
            Some(_) if self.recipe_builder_state.is_none() => {
                self.recipe_builder_state = Some(RecipeBuilderState::new());
            }
            None => {
                self.recipe_builder_state = None;
                self.staged_recipe = None;
            }
            _ => {}
        }
    }

    fn show_recipe_builder(&mut self, ctx: &egui::Context) {
        if !self.should_show_recipe_builder() {
            return;
        }

        let (apply_requested, clear_requested) = self.render_recipe_builder_panel(ctx);

        if apply_requested {
            self.apply_recipe();
        }
        if clear_requested {
            self.clear_recipe_builder();
        }
    }

    fn should_show_recipe_builder(&self) -> bool {
        self.range_selection.is_some()
            && !self.snapshot.recording
            && self.recipe_builder_state.is_some()
    }

    fn render_recipe_builder_panel(&mut self, ctx: &egui::Context) -> (bool, bool) {
        egui::SidePanel::right("recipe_builder")
            .resizable(true)
            .show(ctx, |ui| {
                let state = self.recipe_builder_state.as_mut().unwrap();
                let output = render_recipe_builder(state, &self.staged_recipe, ui);
                (output.apply_requested, output.clear_requested)
            })
            .inner
    }

    fn apply_recipe(&mut self) {
        let (recipe, range) = match self.extract_recipe_and_range() {
            Some(pair) => pair,
            None => return,
        };
        if let Err(err) = self
            .controller
            .apply_recipe(range.start_sec, range.end_sec, recipe)
        {
            self.control_error = Some(format!("Recipe error: {}", err));
        } else {
            self.recipe_status = Some(RecipeStatus::Applying);
            self.clear_recipe_builder();
        }
    }

    fn extract_recipe_and_range(&self) -> Option<(Recipe, RangeSelection)> {
        let staged = self.staged_recipe.as_ref()?;
        let range = self.range_selection?;
        if staged.validate().is_err() {
            return None;
        }
        Some((staged.to_recipe(), range))
    }

    fn clear_recipe_builder(&mut self) {
        self.recipe_builder_state = None;
        self.staged_recipe = None;
        self.range_selection = None;
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
        self.show_recipe_builder(ctx);
        self.show_main(ctx);
    }
}

#[derive(Default)]
struct ControlActions {
    toggle_recording: bool,
    replay_reference: bool,
    stop_replay: bool,
    toggle_to_variant: Option<ClipVariant>,
}

impl ControlActions {
    fn merge(&mut self, other: ControlActions) {
        self.toggle_recording |= other.toggle_recording;
        self.replay_reference |= other.replay_reference;
        self.stop_replay |= other.stop_replay;
        if other.toggle_to_variant.is_some() {
            self.toggle_to_variant = other.toggle_to_variant;
        }
    }
}

impl From<ControlStripOutput> for ControlActions {
    fn from(output: ControlStripOutput) -> Self {
        ControlActions {
            toggle_recording: output.toggle_recording,
            replay_reference: output.replay_reference,
            stop_replay: output.stop_replay,
            toggle_to_variant: output.toggle_to_variant,
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

fn spectrogram_source(alignment: &AlignmentReport) -> &[f32] {
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

fn render_recipe_builder(
    state: &mut RecipeBuilderState,
    staged_recipe: &Option<RuntimeRecipe>,
    ui: &mut egui::Ui,
) -> RecipeBuilderOutput {
    let mut builder = RecipeBuilder {
        state,
        enabled: true,
    };
    let output = builder.show(ui);

    if staged_recipe.is_some() {
        ui.separator();
        show_staged_recipe_message(ui);
    }

    output
}

fn format_recipe_summary(recipe: &RuntimeRecipe) -> String {
    let name = recipe.name.as_deref().unwrap_or("Custom");
    let steps = recipe.steps.len();
    format!("{}: {} steps", name, steps)
}

fn show_staged_recipe_message(ui: &mut egui::Ui) {
    ui.colored_label(
        egui::Color32::from_rgb(30, 180, 80),
        "Recipe ready to apply (Phase 4 will add apply command)",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RuntimeRecipe, RuntimeRecipeStep};

    #[test]
    fn test_format_recipe_summary() {
        let recipe = RuntimeRecipe {
            name: Some("Language Learning".to_string()),
            steps: vec![
                RuntimeRecipeStep {
                    repeat_count: 3,
                    speed_factor: 0.5,
                    silent: false,
                },
                RuntimeRecipeStep {
                    repeat_count: 1,
                    speed_factor: 1.0,
                    silent: true,
                },
            ],
        };
        let summary = format_recipe_summary(&recipe);
        assert_eq!(summary, "Language Learning: 2 steps");
    }

    #[test]
    fn test_format_recipe_summary_unnamed() {
        let recipe = RuntimeRecipe {
            name: None,
            steps: vec![RuntimeRecipeStep {
                repeat_count: 1,
                speed_factor: 1.0,
                silent: false,
            }],
        };
        let summary = format_recipe_summary(&recipe);
        assert_eq!(summary, "Custom: 1 steps");
    }

    #[test]
    fn test_update_recipe_builder_visibility_shows_builder() {
        let selection = Some(RangeSelection {
            start_sec: 0.0,
            end_sec: 10.0,
        });
        let mut builder_state: Option<RecipeBuilderState> = None;

        match selection {
            Some(_) if builder_state.is_none() => {
                builder_state = Some(RecipeBuilderState::new());
            }
            None => {
                builder_state = None;
            }
            _ => {}
        }

        assert!(builder_state.is_some());
    }

    #[test]
    fn test_update_recipe_builder_visibility_clears_on_none() {
        let selection: Option<RangeSelection> = None;
        let mut builder_state = Some(RecipeBuilderState::new());
        let mut staged_recipe = Some(RuntimeRecipe {
            name: Some("Test".to_string()),
            steps: vec![],
        });

        match selection {
            Some(_) if builder_state.is_none() => {
                builder_state = Some(RecipeBuilderState::new());
            }
            None => {
                builder_state = None;
                staged_recipe = None;
            }
            _ => {}
        }

        assert!(builder_state.is_none());
        assert!(staged_recipe.is_none());
    }

    #[test]
    fn test_clear_recipe_builder_clears_all_fields() {
        let _selection = Some(RangeSelection {
            start_sec: 0.0,
            end_sec: 10.0,
        });
        let _builder_state = Some(RecipeBuilderState::new());
        let _staged_recipe = Some(RuntimeRecipe {
            name: Some("Test".to_string()),
            steps: vec![],
        });

        let selection: Option<RangeSelection> = None;
        let builder_state: Option<RecipeBuilderState> = None;
        let staged_recipe: Option<RuntimeRecipe> = None;

        assert!(selection.is_none());
        assert!(builder_state.is_none());
        assert!(staged_recipe.is_none());
    }

    #[test]
    fn test_extract_recipe_and_range_valid() {
        let staged_recipe = Some(RuntimeRecipe {
            name: Some("Test".to_string()),
            steps: vec![RuntimeRecipeStep {
                repeat_count: 1,
                speed_factor: 1.0,
                silent: false,
            }],
        });
        let range_selection = Some(RangeSelection {
            start_sec: 5.0,
            end_sec: 10.0,
        });

        let result = extract_recipe_and_range_helper(&staged_recipe, range_selection);
        assert!(result.is_some());
        let (recipe, range) = result.unwrap();
        assert_eq!(recipe.name, "Test");
        assert_eq!(recipe.steps.len(), 1);
        assert_eq!(range.start_sec, 5.0);
        assert_eq!(range.end_sec, 10.0);
    }

    #[test]
    fn test_extract_recipe_and_range_no_recipe() {
        let staged_recipe = None;
        let range_selection = Some(RangeSelection {
            start_sec: 5.0,
            end_sec: 10.0,
        });

        let result = extract_recipe_and_range_helper(&staged_recipe, range_selection);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_recipe_and_range_no_selection() {
        let staged_recipe = Some(RuntimeRecipe {
            name: Some("Test".to_string()),
            steps: vec![RuntimeRecipeStep {
                repeat_count: 1,
                speed_factor: 1.0,
                silent: false,
            }],
        });
        let range_selection = None;

        let result = extract_recipe_and_range_helper(&staged_recipe, range_selection);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_recipe_and_range_invalid_recipe() {
        let staged_recipe = Some(RuntimeRecipe {
            name: Some("Test".to_string()),
            steps: vec![],
        });
        let range_selection = Some(RangeSelection {
            start_sec: 5.0,
            end_sec: 10.0,
        });

        let result = extract_recipe_and_range_helper(&staged_recipe, range_selection);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_recipe_and_range_zero_repeat_count() {
        let staged_recipe = Some(RuntimeRecipe {
            name: Some("Invalid".to_string()),
            steps: vec![RuntimeRecipeStep {
                repeat_count: 0,
                speed_factor: 1.0,
                silent: false,
            }],
        });
        let range_selection = Some(RangeSelection {
            start_sec: 5.0,
            end_sec: 10.0,
        });

        let result = extract_recipe_and_range_helper(&staged_recipe, range_selection);
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_recipe_and_range_zero_speed() {
        let staged_recipe = Some(RuntimeRecipe {
            name: Some("Invalid".to_string()),
            steps: vec![RuntimeRecipeStep {
                repeat_count: 1,
                speed_factor: 0.0,
                silent: false,
            }],
        });
        let range_selection = Some(RangeSelection {
            start_sec: 5.0,
            end_sec: 10.0,
        });

        let result = extract_recipe_and_range_helper(&staged_recipe, range_selection);
        assert!(result.is_none());
    }

    fn extract_recipe_and_range_helper(
        staged_recipe: &Option<RuntimeRecipe>,
        range_selection: Option<RangeSelection>,
    ) -> Option<(Recipe, RangeSelection)> {
        let staged = staged_recipe.as_ref()?;
        let range = range_selection?;
        if staged.validate().is_err() {
            return None;
        }
        Some((staged.to_recipe(), range))
    }
}
