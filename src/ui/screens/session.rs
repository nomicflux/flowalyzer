use std::collections::VecDeque;
use std::time::Duration;

use crate::pronunciation::session::{
    AlignmentReport, ClipVariant, SessionConfig, SessionController, SessionHandle, SessionSnapshot,
};
use crate::types::{Recipe, RecipeStep};
use eframe::egui;
use eframe::egui::{Color32, Painter, Pos2, Rect, Sense, Stroke, Vec2};

const HISTORY_WINDOW_MS: usize = 30_000;
const FRAME_HOP_MS: usize = 10;
const HISTORY_CAPACITY_FRAMES: usize = HISTORY_WINDOW_MS / FRAME_HOP_MS;

pub struct SessionApp {
    handle: SessionHandle,
    controller: SessionController,
    snapshot: Option<SessionSnapshot>,
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
        let variant_changed = self
            .snapshot
            .as_ref()
            .map(|current| snapshot.active_clip_variant != current.active_clip_variant)
            .unwrap_or(false);
        let started_recording = self
            .snapshot
            .as_ref()
            .map(|current| !current.recording && snapshot.recording)
            .unwrap_or(snapshot.recording);
        if variant_changed || started_recording {
            self.clear_histories();
        }
        self.histories.accumulate(&snapshot.alignment);
        self.snapshot = Some(snapshot);
        self.reference_ready = true;
    }

    pub fn clear_histories(&mut self) {
        self.histories.clear();
    }

    pub fn snapshot(&self) -> Option<&SessionSnapshot> {
        self.snapshot.as_ref()
    }

    fn recording(&self) -> bool {
        self.snapshot.as_ref().map(|s| s.recording).unwrap_or(false)
    }

    fn reference_playing(&self) -> bool {
        self.snapshot
            .as_ref()
            .map(|s| s.reference_playing)
            .unwrap_or(false)
    }

    fn active_variant(&self) -> ClipVariant {
        self.snapshot
            .as_ref()
            .map(|s| s.active_clip_variant)
            .unwrap_or(ClipVariant::Original)
    }

    fn has_flowalyzed_clip(&self) -> bool {
        self.snapshot
            .as_ref()
            .map(|s| s.has_flowalyzed_clip)
            .unwrap_or(false)
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
            let label = if self.recording() {
                "Stop Recording"
            } else {
                "Start Recording"
            };
            if ui.button(label).clicked() {
                self.control_error = if self.recording() {
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

            if self.reference_playing() && ui.button("Stop Replay").clicked() {
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

        if self.has_flowalyzed_clip() {
            let target = match self.active_variant() {
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

        if let Some(snapshot) = &self.snapshot {
            if let Some(state) = &snapshot.recipe_state {
                ui.label(format!(
                    "Recipe: {} ({}/{})",
                    state.stage.label(),
                    state.completed_steps,
                    state.total_steps
                ));
            }
        }
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Recording: {}",
            if self.recording() { "Yes" } else { "No" }
        ));
        ui.label(format!("Active variant: {:?}", self.active_variant()));
        ui.label(format!(
            "Flowalyzed available: {}",
            if self.has_flowalyzed_clip() {
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
            if self.reference_playing() {
                "Playing"
            } else {
                "Stopped"
            }
        ));
        if let Some(snapshot) = &self.snapshot {
            if let Some(error) = &snapshot.error {
                ui.colored_label(Color32::RED, format!("Runtime error: {}", error));
            }
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
        draw_history_plot(
            ui,
            "Waveform energy",
            &self.histories.reference_energy,
            &self.histories.learner_energy,
            (Color32::LIGHT_BLUE, Color32::LIGHT_RED),
        );
        draw_history_plot(
            ui,
            "Pitch contour (Hz)",
            &self.histories.reference_pitch,
            &self.histories.learner_pitch,
            (Color32::LIGHT_GREEN, Color32::LIGHT_YELLOW),
        );
        draw_history_plot(
            ui,
            "Similarity / contour",
            &self.histories.similarity,
            &self.histories.contour,
            (
                Color32::from_rgb(0xE0, 0x9C, 0x35),
                Color32::from_rgb(0x8A, 0x2B, 0xE2),
            ),
        );
        draw_comparison_panel(ui, &self.histories.similarity, &self.histories.contour);
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
        append_chunk(&mut self.reference_energy, &alignment.reference_energy);
        append_chunk(&mut self.learner_energy, &alignment.learner_energy);
        append_chunk(&mut self.reference_pitch, &alignment.reference_pitch);
        append_chunk(&mut self.learner_pitch, &alignment.learner_pitch);
        append_chunk(&mut self.similarity, &alignment.similarity_band);
        append_chunk(&mut self.contour, &alignment.contour_band);
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

fn append_chunk(history: &mut VecDeque<f32>, chunk: &[f32]) {
    if chunk.is_empty() {
        return;
    }
    history.extend(chunk);
    trim_to_window(history);
}

fn trim_to_window(history: &mut VecDeque<f32>) {
    if history.len() > HISTORY_CAPACITY_FRAMES {
        let excess = history.len() - HISTORY_CAPACITY_FRAMES;
        history.drain(0..excess);
    }
}

fn draw_history_plot(
    ui: &mut egui::Ui,
    title: &str,
    reference: &VecDeque<f32>,
    learner: &VecDeque<f32>,
    colors: (Color32, Color32),
) {
    ui.label(title);
    let desired = Vec2::new(ui.available_width(), 120.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter();
    painter.rect(
        rect,
        4.0,
        Color32::from_gray(0x12),
        Stroke::new(1.0, Color32::from_gray(0x44)),
    );
    let inner = rect.shrink(6.0);
    draw_history_line(painter, inner, reference, colors.0);
    draw_history_line(painter, inner, learner, colors.1);
}

fn draw_history_line(painter: &Painter, rect: Rect, history: &VecDeque<f32>, color: Color32) {
    if history.len() < 2 {
        return;
    }

    let (min, max) = history
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), &value| {
            (min.min(value), max.max(value))
        });
    let range = (max - min).max(1e-4);
    let len = history.len();
    let mut points = Vec::with_capacity(len);
    for (idx, &value) in history.iter().enumerate() {
        let t = if len > 1 {
            idx as f32 / (len - 1) as f32
        } else {
            0.5
        };
        let x = rect.left() + t * rect.width();
        let normalized = (value - min) / range;
        let y = rect.bottom() - normalized * rect.height();
        points.push(Pos2::new(x, y));
    }

    for segment in points.windows(2) {
        painter.line_segment([segment[0], segment[1]], Stroke::new(1.5, color));
    }
}

fn draw_comparison_panel(ui: &mut egui::Ui, similarity: &VecDeque<f32>, contour: &VecDeque<f32>) {
    ui.label("Spectrogram comparison");
    let desired = Vec2::new(ui.available_width(), 120.0);
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    let painter = ui.painter();
    painter.rect(
        rect,
        4.0,
        Color32::from_gray(0x10),
        Stroke::new(1.0, Color32::from_gray(0x44)),
    );
    let inner = rect.shrink(4.0);
    const ROWS: usize = 2;
    let rows = ROWS;
    let max_columns = HISTORY_CAPACITY_FRAMES / 10; // ~300 columns at 10ms hop = 3s segments; still under 300
    let columns = max_columns.clamp(32, HISTORY_CAPACITY_FRAMES);
    for row in 0..rows {
        let source = if row == 0 { similarity } else { contour };
        draw_row(
            painter,
            inner,
            row,
            columns,
            source,
            if row == 0 {
                |value| similarity_color(value)
            } else {
                |value| contour_color(value)
            },
        );
    }

    ui.horizontal(|ui| {
        ui.colored_label(Color32::LIGHT_BLUE, "Similarity");
        ui.colored_label(Color32::LIGHT_RED, "Contour");
    });
}

fn draw_row<F>(
    painter: &Painter,
    inner: Rect,
    row: usize,
    columns: usize,
    source: &VecDeque<f32>,
    color_fn: F,
) where
    F: Fn(f32) -> Color32,
{
    if source.is_empty() {
        return;
    }
    let length = source.len();
    let column_width = (inner.width() / columns as f32).max(1.0);
    let column_height = inner.height() / 2.0;
    for col in 0..columns {
        let sample_idx =
            ((col as f32) * length as f32 / columns as f32).min((length - 1) as f32) as usize;
        let value = *source.get(sample_idx).unwrap_or(&0.0);
        let color = color_fn(value);
        let left = inner.left() + col as f32 * column_width;
        let right = left + column_width - 1.0;
        let top = inner.top() + row as f32 * column_height;
        let bottom = top + column_height - 1.0;
        let cell_rect = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom));
        painter.rect_filled(cell_rect, 2.0, color);
    }
}

fn similarity_color(value: f32) -> Color32 {
    gradient_color(
        value,
        &[
            (0.0, Color32::from_rgb(255, 0, 0)),
            (0.3, Color32::from_rgb(255, 165, 0)),
            (0.6, Color32::from_rgb(0, 255, 0)),
            (1.0, Color32::from_rgb(0, 0, 255)),
        ],
    )
}

fn contour_color(value: f32) -> Color32 {
    gradient_color(
        value,
        &[
            (0.0, Color32::from_rgb(0, 0, 128)),
            (0.5, Color32::from_rgb(128, 0, 255)),
            (1.0, Color32::from_rgb(255, 0, 0)),
        ],
    )
}

fn gradient_color(value: f32, stops: &[(f32, Color32)]) -> Color32 {
    let v = value.clamp(0.0, 1.0);
    if let Some((first_pos, first_color)) = stops.first() {
        if v <= *first_pos {
            return *first_color;
        }
    }
    for window in stops.windows(2) {
        if v >= window[0].0 && v <= window[1].0 {
            let t = (v - window[0].0) / (window[1].0 - window[0].0);
            return lerp_color(window[0].1, window[1].1, t);
        }
    }
    stops.last().map(|(_, c)| *c).unwrap_or(Color32::WHITE)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let ta = t.clamp(0.0, 1.0);
    let r = a.r() as f32 + (b.r() as f32 - a.r() as f32) * ta;
    let g = a.g() as f32 + (b.g() as f32 - a.g() as f32) * ta;
    let b = a.b() as f32 + (b.b() as f32 - a.b() as f32) * ta;
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pronunciation::session::PronunciationScores;
    use crate::pronunciation::session::SessionRuntime;
    use crate::pronunciation::RecordedClip;

    fn dummy_app() -> SessionApp {
        let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
        let config = SessionConfig::default();
        let (handle, controller) = SessionRuntime::spawn(clip, config);
        SessionApp::new(handle, controller)
    }

    fn report_with_value(value: f32) -> AlignmentReport {
        let frame_count = HISTORY_CAPACITY_FRAMES / 2;
        let hop_ms = FRAME_HOP_MS as f32;
        AlignmentReport {
            reference_energy: vec![value; frame_count],
            learner_energy: vec![value; frame_count],
            energy_error: vec![0.0; frame_count],
            reference_pitch: vec![value; frame_count],
            learner_pitch: vec![value; frame_count],
            similarity_band: vec![value; frame_count],
            contour_band: vec![value; frame_count],
            start_frame_idx: 0,
            end_frame_idx: frame_count,
            hop_ms,
            global_time_offset_ms: 0.0,
            total_duration: hop_ms * frame_count as f32,
        }
    }

    fn scores_with_value(value: f32) -> PronunciationScores {
        PronunciationScores {
            overall: value,
            timing: value,
            articulation: value,
            intonation: value,
        }
    }

    fn snapshot_with_alignment(
        alignment: AlignmentReport,
        recording: bool,
        variant: ClipVariant,
    ) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            scores: scores_with_value(0.0),
            recording,
            reference_playing: false,
            active_clip_variant: variant,
            has_flowalyzed_clip: false,
            recipe_state: None,
            error: None,
        }
    }

    #[test]
    fn histories_trim_to_window() {
        let mut app = dummy_app();
        for _ in 0..3 {
            let snapshot =
                snapshot_with_alignment(report_with_value(1.0), false, ClipVariant::Original);
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
        let snapshot =
            snapshot_with_alignment(report_with_value(0.5), false, ClipVariant::Original);
        app.apply_snapshot(snapshot);
        app.clear_histories();
        assert!(app.histories.reference_energy.is_empty());
        assert!(app.histories.similarity.is_empty());
    }

    #[test]
    fn histories_clear_on_variant_toggle() {
        let mut app = dummy_app();
        let snapshot =
            snapshot_with_alignment(report_with_value(0.5), false, ClipVariant::Original);
        app.apply_snapshot(snapshot);
        let toggled =
            snapshot_with_alignment(report_with_value(0.25), false, ClipVariant::Flowalyzed);
        app.apply_snapshot(toggled);
        let expected = HISTORY_CAPACITY_FRAMES / 2;
        assert_eq!(expected, app.histories.reference_energy.len());
        assert_eq!(expected, app.histories.similarity.len());
    }

    #[test]
    fn histories_clear_on_restart() {
        let mut app = dummy_app();
        let snapshot =
            snapshot_with_alignment(report_with_value(0.5), false, ClipVariant::Original);
        app.apply_snapshot(snapshot);
        let restarted =
            snapshot_with_alignment(report_with_value(0.5), true, ClipVariant::Original);
        app.apply_snapshot(restarted);
        assert_eq!(
            HISTORY_CAPACITY_FRAMES / 2,
            app.histories.reference_energy.len()
        );
    }
}
