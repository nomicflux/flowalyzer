use std::collections::VecDeque;
use std::time::Duration;

use crate::pronunciation::session::{
    AlignmentReport, ClipVariant, SessionConfig, SessionController, SessionHandle, SessionSnapshot,
};
use crate::types::{Recipe, RecipeStep};
use eframe::egui;
use eframe::egui::Color32;

const HISTORY_WINDOW_MS: usize = 30_000;
const FRAME_HOP_MS: usize = 10;
const HISTORY_CAPACITY_FRAMES: usize = HISTORY_WINDOW_MS / FRAME_HOP_MS;

pub struct SessionApp {
    handle: SessionHandle,
    controller: SessionController,
    snapshot: SessionSnapshot,
    control_error: Option<String>,
    histories: HistoryBuffers,
    recipe_start_input: String,
    recipe_end_input: String,
    reference_ready: bool,
}

impl SessionApp {
    pub fn new(handle: SessionHandle, controller: SessionController) -> Self {
        Self {
            snapshot: handle.initial_snapshot(),
            handle,
            controller,
            control_error: None,
            histories: HistoryBuffers::new(),
            recipe_start_input: "0.0".to_string(),
            recipe_end_input: "2.0".to_string(),
            reference_ready: false,
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: SessionSnapshot) {
        self.histories.accumulate(&snapshot.alignment);
        self.snapshot = snapshot;
        self.reference_ready = true;
    }

    pub fn clear_histories(&mut self) {
        self.histories.clear();
    }

    pub fn snapshot(&self) -> &SessionSnapshot {
        &self.snapshot
    }

    pub fn config(&self) -> &SessionConfig {
        self.handle.config()
    }

    fn poll_snapshots(&mut self) {
        for snapshot in self.handle.drain_snapshots() {
            self.apply_snapshot(snapshot);
        }
    }

    fn show_top_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let label = if self.snapshot.recording {
                "Stop Recording"
            } else {
                "Start Recording"
            };
            if ui.button(label).clicked() {
                self.control_error = if self.snapshot.recording {
                    self.controller.stop().err().map(|err| err.to_string())
                } else {
                    self.controller.start().err().map(|err| err.to_string())
                };
            }

            if ui.button("Replay Reference").clicked() {
                if let Err(err) = self.controller.replay_reference() {
                    self.control_error = Some(err.to_string());
                }
            }

            if self.snapshot.reference_playing && ui.button("Stop Replay").clicked() {
                if let Err(err) = self.controller.stop_replay() {
                    self.control_error = Some(err.to_string());
                }
            }

            if ui.button("Clear Histories").clicked() {
                self.clear_histories();
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Recipe start (s):");
            ui.text_edit_singleline(&mut self.recipe_start_input);
            ui.label("end (s):");
            ui.text_edit_singleline(&mut self.recipe_end_input);
            if ui.button("Apply Default Recipe").clicked() {
                match self.parse_recipe_bounds() {
                    Ok((start, end)) => {
                        if let Err(err) =
                            self.controller
                                .apply_recipe(start, end, Self::default_recipe())
                        {
                            self.control_error = Some(err.to_string());
                        }
                    }
                    Err(err) => self.control_error = Some(err),
                }
            }
        });

        ui.separator();

        if self.snapshot.has_flowalyzed_clip {
            let target = match self.snapshot.active_clip_variant {
                ClipVariant::Original => ClipVariant::Flowalyzed,
                ClipVariant::Flowalyzed => ClipVariant::Original,
            };
            if ui.button(format!("Switch to {:?}", target)).clicked() {
                if let Err(err) = self.controller.toggle_clip_variant(target) {
                    self.control_error = Some(err.to_string());
                }
            }
        } else {
            ui.label("Flowalyzed clip not available yet.");
        }

        if let Some(err) = &self.control_error {
            ui.colored_label(Color32::RED, err);
        }
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Recording: {}",
            if self.snapshot.recording { "Yes" } else { "No" }
        ));
        ui.label(format!(
            "Active variant: {:?}",
            self.snapshot.active_clip_variant
        ));
        ui.label(format!(
            "Flowalyzed available: {}",
            if self.snapshot.has_flowalyzed_clip {
                "Yes"
            } else {
                "No"
            }
        ));
        ui.label(if self.reference_ready {
            "Reference features: Ready"
        } else {
            "Reference features: Loading..."
        });
        ui.label(format!(
            "Reference playback: {}",
            if self.snapshot.reference_playing {
                "Playing"
            } else {
                "Stopped"
            }
        ));
        if let Some(error) = &self.snapshot.error {
            ui.colored_label(Color32::RED, format!("Runtime error: {}", error));
        }
        ui.separator();
        ui.label(format!(
            "Energy frames stored: {}",
            self.histories.reference_energy.len()
        ));
        ui.label(format!(
            "Pitch frames stored: {}",
            self.histories.reference_pitch.len()
        ));
    }

    fn show_visualizations(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Latest energy ref/learner: {:.3} / {:.3}",
            last_value(&self.histories.reference_energy),
            last_value(&self.histories.learner_energy)
        ));
        ui.label(format!(
            "Latest pitch ref/learner: {:.3} / {:.3}",
            last_value(&self.histories.reference_pitch),
            last_value(&self.histories.learner_pitch)
        ));
        ui.label(format!(
            "Latest similarity/contour: {:.3} / {:.3}",
            last_value(&self.histories.similarity),
            last_value(&self.histories.contour)
        ));
    }

    fn parse_recipe_bounds(&self) -> Result<(f64, f64), String> {
        let start = self
            .recipe_start_input
            .parse::<f64>()
            .map_err(|_| "Invalid recipe start".to_string())?;
        let end = self
            .recipe_end_input
            .parse::<f64>()
            .map_err(|_| "Invalid recipe end".to_string())?;
        if end <= start {
            return Err("Recipe end must be greater than start".to_string());
        }
        Ok((start, end))
    }

    fn default_recipe() -> Recipe {
        Recipe::new("flowalyzer-default").add_step(RecipeStep {
            repeat_count: 1,
            speed_factor: 1.0,
            silent: false,
        })
    }
}

impl Drop for SessionApp {
    fn drop(&mut self) {
        let _ = self.controller.shutdown();
    }
}

impl eframe::App for SessionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_snapshots();
        egui::TopBottomPanel::top("session_controls").show(ctx, |ui| {
            self.show_top_panel(ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_status(ui);
            ui.separator();
            self.show_visualizations(ui);
        });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

struct HistoryBuffers {
    reference_energy: VecDeque<f32>,
    learner_energy: VecDeque<f32>,
    reference_pitch: VecDeque<f32>,
    learner_pitch: VecDeque<f32>,
    similarity: VecDeque<f32>,
    contour: VecDeque<f32>,
}

impl HistoryBuffers {
    fn new() -> Self {
        Self {
            reference_energy: VecDeque::with_capacity(HISTORY_CAPACITY_FRAMES),
            learner_energy: VecDeque::with_capacity(HISTORY_CAPACITY_FRAMES),
            reference_pitch: VecDeque::with_capacity(HISTORY_CAPACITY_FRAMES),
            learner_pitch: VecDeque::with_capacity(HISTORY_CAPACITY_FRAMES),
            similarity: VecDeque::with_capacity(HISTORY_CAPACITY_FRAMES),
            contour: VecDeque::with_capacity(HISTORY_CAPACITY_FRAMES),
        }
    }

    fn accumulate(&mut self, alignment: &AlignmentReport) {
        append_history(&mut self.reference_energy, &alignment.reference_energy);
        append_history(&mut self.learner_energy, &alignment.learner_energy);
        append_history(&mut self.reference_pitch, &alignment.reference_pitch);
        append_history(&mut self.learner_pitch, &alignment.learner_pitch);
        append_history(&mut self.similarity, &alignment.similarity_band);
        append_history(&mut self.contour, &alignment.contour_band);
    }

    fn clear(&mut self) {
        self.reference_energy.clear();
        self.learner_energy.clear();
        self.reference_pitch.clear();
        self.learner_pitch.clear();
        self.similarity.clear();
        self.contour.clear();
    }
}

fn append_history(history: &mut VecDeque<f32>, chunk: &[f32]) {
    if chunk.is_empty() {
        return;
    }
    history.extend(chunk);
    trim_history(history);
}

fn trim_history(history: &mut VecDeque<f32>) {
    if history.len() > HISTORY_CAPACITY_FRAMES {
        let excess = history.len() - HISTORY_CAPACITY_FRAMES;
        history.drain(0..excess);
    }
}

fn last_value(history: &VecDeque<f32>) -> f32 {
    history.back().copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pronunciation::session::SessionRuntime;
    use crate::pronunciation::RecordedClip;

    fn dummy_app() -> SessionApp {
        let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
        let config = SessionConfig::default();
        let (handle, controller) = SessionRuntime::spawn(clip, config);
        SessionApp::new(handle, controller)
    }

    fn report_with_value(value: f32) -> AlignmentReport {
        AlignmentReport {
            reference_energy: vec![value; HISTORY_CAPACITY_FRAMES / 2],
            learner_energy: vec![value; HISTORY_CAPACITY_FRAMES / 2],
            reference_pitch: vec![value; HISTORY_CAPACITY_FRAMES / 2],
            learner_pitch: vec![value; HISTORY_CAPACITY_FRAMES / 2],
            similarity_band: vec![value; HISTORY_CAPACITY_FRAMES / 2],
            contour_band: vec![value; HISTORY_CAPACITY_FRAMES / 2],
            ..AlignmentReport::default()
        }
    }

    #[test]
    fn histories_trim_to_window() {
        let mut app = dummy_app();
        for _ in 0..3 {
            let snapshot = SessionSnapshot {
                alignment: report_with_value(1.0),
                ..SessionSnapshot::default()
            };
            app.apply_snapshot(snapshot);
        }
        assert_eq!(
            HISTORY_CAPACITY_FRAMES,
            app.histories.reference_energy.len()
        );
    }

    #[test]
    fn clear_histories_resets_state() {
        let mut app = dummy_app();
        let snapshot = SessionSnapshot {
            alignment: report_with_value(0.5),
            ..SessionSnapshot::default()
        };
        app.apply_snapshot(snapshot);
        app.clear_histories();
        assert!(app.histories.reference_energy.is_empty());
        assert!(app.histories.similarity.is_empty());
    }
}
