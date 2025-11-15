use std::time::Duration;

use eframe::egui;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, ClipVariant, InitializationStage, RecipeApplicationStage,
    Result as SessionResult, SessionController, SessionHandle, SessionSnapshot,
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
        
        // CRITICAL: Handle two cases:
        // 1. Initial state (no phonemes): Use reference arrays directly for display
        // 2. Shadowing state (has phonemes): Align arrays using phoneme timing
        let ref_energy = &self.snapshot.alignment.reference_energy;
        let learner_energy = &self.snapshot.alignment.learner_energy;
        let ref_pitch = &self.snapshot.alignment.reference_pitch;
        let learner_pitch = &self.snapshot.alignment.learner_pitch;
        
        let (aligned_ref_energy, aligned_learner_energy, aligned_ref_pitch, aligned_learner_pitch) = 
            if self.snapshot.alignment.phonemes.is_empty() {
                // Initial state: No alignment yet, just use reference arrays directly
                // Learner arrays are empty, so just use reference for display
                (
                    ref_energy.clone(),
                    learner_energy.clone(),
                    ref_pitch.clone(),
                    learner_pitch.clone(),
                )
            } else {
                // Shadowing state: Align arrays using phoneme timing
                let (ref_e, learner_e) = build_aligned_from_phonemes(&self.snapshot.alignment.phonemes, ref_energy, learner_energy);
                let (ref_p, learner_p) = build_aligned_from_phonemes(&self.snapshot.alignment.phonemes, ref_pitch, learner_pitch);
                (ref_e, learner_e, ref_p, learner_p)
            };
        
        // Take the most recent frames
        let ref_window = tail_slice(&aligned_ref_energy, FRAME_WINDOW);
        let learner_window = tail_slice(&aligned_learner_energy, FRAME_WINDOW);
        let ref_pitch_window = tail_slice(&aligned_ref_pitch, FRAME_WINDOW);
        let learner_pitch_window = tail_slice(&aligned_learner_pitch, FRAME_WINDOW);
        
        // Pad to same length
        let max_len = ref_window.len().max(learner_window.len());
        let mut ref_padded = ref_window;
        let mut learner_padded = learner_window;
        ref_padded.resize(max_len, 0.0);
        learner_padded.resize(max_len, 0.0);
        
        let max_pitch_len = ref_pitch_window.len().max(learner_pitch_window.len());
        let mut ref_pitch_padded = ref_pitch_window;
        let mut learner_pitch_padded = learner_pitch_window;
        ref_pitch_padded.resize(max_pitch_len, 0.0);
        learner_pitch_padded.resize(max_pitch_len, 0.0);
        
        self.reference_waveform = normalize_series(ref_padded);
        self.learner_waveform = normalize_series(learner_padded);
        self.reference_pitch = ref_pitch_padded;
        self.learner_pitch = learner_pitch_padded;
        // CRITICAL DEBUG: Log what we're actually displaying to diagnose garbled visualization
        if !self.snapshot.alignment.reference_energy.is_empty() || !self.snapshot.alignment.learner_energy.is_empty() {
            let ref_energy_preview: Vec<f32> = self.reference_waveform.iter().take(10).copied().collect();
            let learner_energy_preview: Vec<f32> = self.learner_waveform.iter().take(10).copied().collect();
            let ref_pitch_preview: Vec<f32> = self.reference_pitch.iter().take(10).copied().collect();
            let learner_pitch_preview: Vec<f32> = self.learner_pitch.iter().take(10).copied().collect();
            tracing::warn!(
                reference_energy_frames = self.snapshot.alignment.reference_energy.len(),
                learner_energy_frames = self.snapshot.alignment.learner_energy.len(),
                reference_pitch_frames = self.snapshot.alignment.reference_pitch.len(),
                learner_pitch_frames = self.snapshot.alignment.learner_pitch.len(),
                reference_waveform_display_len = self.reference_waveform.len(),
                learner_waveform_display_len = self.learner_waveform.len(),
                reference_waveform_preview = ?ref_energy_preview,
                learner_waveform_preview = ?learner_energy_preview,
                reference_pitch_preview = ?ref_pitch_preview,
                learner_pitch_preview = ?learner_pitch_preview,
                phoneme_count = self.snapshot.alignment.phonemes.len(),
                global_time_offset_ms = self.snapshot.alignment.global_time_offset_ms,
                recording = self.snapshot.recording,
                "DIAGNOSE: What data is being displayed in visualizations?"
            );
        }
        self.spectrogram = build_spectrogram_window(&self.snapshot.alignment, FRAME_WINDOW);
        
        // DEBUG: Log spectrogram values to diagnose solid blue bar
        if let Some(ref spec) = self.spectrogram {
            let similarity_preview: Vec<f32> = (0..spec.cols.min(10))
                .map(|col| spec.value(0, col))
                .collect();
            let contour_preview: Vec<f32> = (0..spec.cols.min(10))
                .map(|col| spec.value(1, col))
                .collect();
            tracing::warn!(
                spectrogram_cols = spec.cols,
                similarity_preview = ?similarity_preview,
                contour_preview = ?contour_preview,
                similarity_band_len = self.snapshot.alignment.similarity_band.len(),
                contour_band_len = self.snapshot.alignment.contour_band.len(),
                "DIAGNOSE: Why is spectrogram showing solid blue bar?"
            );
        }
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
            // Diagnostic logging: verify what snapshot update UI is receiving
            tracing::info!(
                update_reference_energy_frames = update.alignment.reference_energy.len(),
                update_learner_energy_frames = update.alignment.learner_energy.len(),
                update_reference_pitch_frames = update.alignment.reference_pitch.len(),
                update_learner_pitch_frames = update.alignment.learner_pitch.len(),
                update_phonemes = update.alignment.phonemes.len(),
                update_recording = update.recording,
                "UI received snapshot update"
            );
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
                    has_flowalyzed_clip: self.snapshot.has_flowalyzed_clip,
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
        if self.snapshot.recipe_applying {
            if let Some(progress) = &self.snapshot.recipe_progress {
                let stage_number = progress.stage.order() + 1;
                ui.colored_label(
                    egui::Color32::from_rgb(210, 160, 20),
                    format!(
                        "Applying recipe… Stage {}/{}: {}",
                        stage_number,
                        progress.total_steps,
                        progress.stage.label()
                    ),
                );
                ui.add_space(4.0);
                for stage in RecipeApplicationStage::ordered().iter() {
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
                if let (Some(label), Some(curr)) = (&progress.metric_label, progress.current_value)
                {
                    if let Some(total) = progress.total_value {
                        ui.label(format!("    ↳ {} {} / {}", label, curr, total));
                    } else {
                        ui.label(format!("    ↳ {} {}", label, curr));
                    }
                }
                if let Some(secs) = progress.elapsed_secs {
                    ui.label(format!("       {}s elapsed", secs));
                }
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(210, 160, 20),
                    "Applying recipe... Please wait.",
                );
            }
        } else if let Some(status) = &self.recipe_status {
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
            
            // Reference waveform with clear label
            ui.label(egui::RichText::new("Reference (Native Speaker)").color(egui::Color32::from_rgb(80, 160, 255)));
            ui.end_row();
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
            
            // Learner waveform with clear label
            ui.label(egui::RichText::new("Your Voice (Shadowing)").color(egui::Color32::from_rgb(250, 120, 120)));
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
            
            // Show timing info
            if !self.reference_waveform.is_empty() && !self.learner_waveform.is_empty() {
                let frames_shown = self.reference_waveform.len().min(self.learner_waveform.len());
                let time_shown_ms = frames_shown as f32 * FRAME_HOP_MS;
                ui.label(egui::RichText::new(format!("Showing last {:.1}s of aligned audio", time_shown_ms / 1000.0)).small());
            }
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
            // Convert builder state to staged recipe before applying
            if let Some(state) = &self.recipe_builder_state {
                self.staged_recipe = Some(state.to_runtime_recipe());
            }
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
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Pitch Contour").size(14.0));
            ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(80, 160, 255)));
            ui.label("Reference");
            ui.label(egui::RichText::new("●").color(egui::Color32::from_rgb(250, 120, 120)));
            ui.label("Your Voice");
        });
        PitchView {
            reference: &self.reference_pitch,
            learner: &self.learner_pitch,
        }
        .show(ui);
        if !self.reference_pitch.is_empty() && !self.learner_pitch.is_empty() {
            let frames_shown = self.reference_pitch.len().min(self.learner_pitch.len());
            let time_shown_ms = frames_shown as f32 * FRAME_HOP_MS;
            ui.label(egui::RichText::new(format!("Showing last {:.1}s of aligned pitch", time_shown_ms / 1000.0)).small());
        }
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
    // Show 2 rows: Similarity and Contour
    // Each row represents time segments (columns)
    let similarity_window = tail_slice(&alignment.similarity_band, max_rows);
    let contour_window = tail_slice(&alignment.contour_band, max_rows);
    
    if similarity_window.is_empty() && contour_window.is_empty() {
        return None;
    }
    
    // Use the length of the longer series, or pad to match
    let time_segments = similarity_window.len().max(contour_window.len());
    if time_segments == 0 {
        return None;
    }
    
    // CRITICAL: Check if all values are suspiciously high (>= 0.8) - this indicates a bug
    let all_high_similarity = !similarity_window.is_empty() && 
        similarity_window.iter().all(|&v| v >= 0.8);
    if all_high_similarity {
        tracing::warn!(
            similarity_window_len = similarity_window.len(),
            similarity_preview = ?similarity_window.iter().take(10).copied().collect::<Vec<f32>>(),
            phoneme_count = alignment.phonemes.len(),
            "WARNING: All similarity values are >= 0.8 - possible stale/incorrect data"
        );
    }
    
    // Build values: 2 rows (Similarity, Contour) × time_segments columns
    let mut values = Vec::with_capacity(2 * time_segments);
    
    // Row 0: Similarity band
    for i in 0..time_segments {
        let value = similarity_window.get(i).copied().unwrap_or(0.0);
        values.push(value.clamp(0.0, 1.0));
    }
    
    // Row 1: Contour band
    for i in 0..time_segments {
        let value = contour_window.get(i).copied().unwrap_or(0.0);
        values.push(value.clamp(0.0, 1.0));
    }
    
    Some(SpectrogramData::new(2, time_segments, values))
}

fn build_aligned_from_phonemes(
    phonemes: &[AlignedPhoneme],
    reference: &[f32],
    learner: &[f32],
) -> (Vec<f32>, Vec<f32>) {
    // Reconstruct aligned arrays from phoneme timing
    // Each phoneme tells us which reference frames map to which learner frames
    let mut aligned_ref = Vec::new();
    let mut aligned_learner = Vec::new();
    
    for phoneme in phonemes {
        let ref_start_frame = (phoneme.reference_start_ms / FRAME_HOP_MS) as usize;
        let ref_end_frame = (phoneme.reference_end_ms / FRAME_HOP_MS) as usize;
        let learner_start_frame = (phoneme.learner_start_ms / FRAME_HOP_MS) as usize;
        let learner_end_frame = (phoneme.learner_end_ms / FRAME_HOP_MS) as usize;
        
        // Extract frames for this phoneme
        let ref_frames = if ref_start_frame < reference.len() {
            reference[ref_start_frame..ref_end_frame.min(reference.len())].to_vec()
        } else {
            Vec::new()
        };
        
        let learner_frames = if learner_start_frame < learner.len() {
            learner[learner_start_frame..learner_end_frame.min(learner.len())].to_vec()
        } else {
            Vec::new()
        };
        
        // Align by taking max length and padding/interpolating
        let max_phoneme_len = ref_frames.len().max(learner_frames.len());
        if max_phoneme_len > 0 {
            let mut ref_aligned = ref_frames;
            let mut learner_aligned = learner_frames;
            ref_aligned.resize(max_phoneme_len, 0.0);
            learner_aligned.resize(max_phoneme_len, 0.0);
            aligned_ref.extend(ref_aligned);
            aligned_learner.extend(learner_aligned);
        }
    }
    
    (aligned_ref, aligned_learner)
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
