use std::collections::VecDeque;
use std::time::Duration;

use crate::pronunciation::session::{
    AlignmentReport, SessionConfig, SessionController, SessionHandle, SessionSnapshot,
};
use eframe::egui;
use eframe::egui::{Color32, Painter, Pos2, Rect, Sense, Stroke, Vec2};

const HISTORY_WINDOW_MS: usize = 30_000;
const FRAME_HOP_MS: usize = 10;
const HISTORY_CAPACITY_FRAMES: usize = HISTORY_WINDOW_MS / FRAME_HOP_MS;

pub struct SessionApp {
    handle: SessionHandle,
    controller: SessionController,
    snapshot: SessionSnapshot,
    control_error: Option<String>,
    histories: HistoryBuffers,
    reference_ready: bool,
}

impl SessionApp {
    pub fn new(handle: SessionHandle, controller: SessionController) -> Self {
        let snapshot = handle.initial_snapshot().clone();
        Self {
            snapshot,
            handle,
            controller,
            control_error: None,
            histories: HistoryBuffers::new(),
            reference_ready: true,
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: SessionSnapshot) {
        let started_recording = !self.snapshot.recording && snapshot.recording;
        if started_recording {
            self.clear_histories();
        }
        self.histories.accumulate(&snapshot.alignment);
        self.snapshot = snapshot;
    }

    pub fn clear_histories(&mut self) {
        self.histories.clear();
    }

    pub fn snapshot(&self) -> &SessionSnapshot {
        &self.snapshot
    }

    fn recording(&self) -> bool {
        self.snapshot.recording
    }

    fn reference_playing(&self) -> bool {
        self.snapshot.reference_playing
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
            if ui
                .add_enabled(!self.reference_playing(), egui::Button::new(label))
                .clicked()
            {
                self.control_error = if self.recording() {
                    self.controller.stop().err().map(|err| err.to_string())
                } else {
                    self.controller.start().err().map(|err| err.to_string())
                };
            }

            if ui
                .add_enabled(!self.recording(), egui::Button::new("Shadow"))
                .clicked()
            {
                self.control_error = if self.recording() || self.reference_playing() {
                    Some("Shadow unavailable during recording or playback".to_string())
                } else {
                    self.controller.shadow().err().map(|err| err.to_string())
                };
            }

            if ui
                .add_enabled(!self.recording(), egui::Button::new("Replay Reference"))
                .clicked()
            {
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

        if let Some(err) = &self.control_error {
            ui.colored_label(Color32::RED, err);
        }
    }

    fn show_status(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Recording: {}",
            if self.recording() { "Yes" } else { "No" }
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
            false,
        );
        draw_history_plot(
            ui,
            "Pitch contour (Hz)",
            &self.histories.reference_pitch,
            &self.histories.learner_pitch,
            (Color32::LIGHT_GREEN, Color32::LIGHT_YELLOW),
            false,
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
            true,
        );
        draw_comparison_panel(ui, &self.histories.similarity, &self.histories.contour);
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

fn smooth_series(history: &VecDeque<f32>) -> Vec<f32> {
    if history.len() < 2 {
        return history.iter().copied().collect();
    }
    let mut smoothed = Vec::with_capacity(history.len());
    for idx in 0..history.len() {
        let start = idx.saturating_sub(1);
        let end = (idx + 2).min(history.len());
        let mut sum = 0.0;
        let mut count: f32 = 0.0;
        history
            .iter()
            .take(end)
            .skip(start)
            .for_each(|v| {
                sum += *v;
                count += 1.0;
            });
        smoothed.push(sum / count.max(1.0));
    }
    smoothed
}

fn shared_range(reference: &[f32], learner: &[f32]) -> (f32, f32) {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for v in reference.iter().chain(learner.iter()) {
        min = min.min(*v);
        max = max.max(*v);
    }
    if min.is_infinite() || max.is_infinite() {
        (0.0, 0.0)
    } else {
        (min, max)
    }
}

fn draw_history_plot(
    ui: &mut egui::Ui,
    title: &str,
    reference: &VecDeque<f32>,
    learner: &VecDeque<f32>,
    colors: (Color32, Color32),
    smooth: bool,
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
    let reference_series = if smooth {
        smooth_series(reference)
    } else {
        reference.iter().copied().collect()
    };
    let learner_series = if smooth {
        smooth_series(learner)
    } else {
        learner.iter().copied().collect()
    };
    let (min, max) = shared_range(&reference_series, &learner_series);
    draw_history_line(painter, inner, &reference_series, min, max, colors.0);
    draw_history_line(painter, inner, &learner_series, min, max, colors.1);
}

fn draw_history_line(
    painter: &Painter,
    rect: Rect,
    history: &[f32],
    min: f32,
    max: f32,
    color: Color32,
) {
    if history.len() < 2 || (max - min).abs() < 1e-6 {
        return;
    }

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
    ui.label("Spectrogram comparison (Similarity: higher is better; Contour: ~0 means matched)");
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
    let columns = max_columns;
    let sim_range = data_range(similarity);
    let contour_range = data_range(contour);
    for row in 0..rows {
        let source = if row == 0 { similarity } else { contour };
        let range = if row == 0 { sim_range } else { contour_range };
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
            range,
        );
    }

    ui.horizontal(|ui| {
        ui.colored_label(Color32::LIGHT_BLUE, "Similarity (scaled 0–1)");
        ui.colored_label(Color32::LIGHT_RED, "Contour (~0 match)");
    });
}

fn draw_row<F>(
    painter: &Painter,
    inner: Rect,
    row: usize,
    columns: usize,
    source: &VecDeque<f32>,
    color_fn: F,
    range: (f32, f32),
) where
    F: Fn(f32) -> Color32,
{
    if source.is_empty() {
        return;
    }
    let length = source.len();
    let available_columns = columns.min(length).max(1);
    let column_width = (inner.width() / available_columns as f32).max(1.0);
    let column_height = inner.height() / 2.0;
    let (min, max) = range;
    let span = (max - min).abs().max(1e-6);
    let mid = (max + min) * 0.5;
    for col in 0..available_columns {
        let sample_idx = ((col as f32) * length as f32 / available_columns as f32)
            .min((length - 1) as f32) as usize;
        let value = *source.get(sample_idx).unwrap();
        let normalized = 0.5 + 0.5 * (value - mid) / span;
        let color = color_fn(normalized);
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
            (0.0, Color32::from_rgb(26, 158, 92)),
            (0.5, Color32::from_rgb(244, 180, 66)),
            (1.0, Color32::from_rgb(180, 32, 32)),
        ],
    )
}

fn contour_color(value: f32) -> Color32 {
    gradient_color(
        value,
        &[
            (0.0, Color32::from_rgb(0, 32, 128)),
            (0.5, Color32::from_rgb(64, 192, 255)),
            (1.0, Color32::from_rgb(240, 96, 32)),
        ],
    )
}

fn gradient_color(value: f32, stops: &[(f32, Color32)]) -> Color32 {
    let v = value;
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
    stops.last().map(|(_, c)| *c).unwrap()
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let r = a.r() as f32 + (b.r() as f32 - a.r() as f32) * t;
    let g = a.g() as f32 + (b.g() as f32 - a.g() as f32) * t;
    let b = a.b() as f32 + (b.b() as f32 - a.b() as f32) * t;
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

fn data_range(values: &VecDeque<f32>) -> (f32, f32) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for v in values.iter() {
        min = min.min(*v);
        max = max.max(*v);
    }
    if !min.is_finite() || !max.is_finite() {
        (0.0, 0.0)
    } else {
        (min, max)
    }
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

    fn report_with_length(value: f32, frame_count: usize) -> AlignmentReport {
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

    fn snapshot_with_alignment(alignment: AlignmentReport, recording: bool) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            recording,
            reference_playing: false,
        }
    }

    #[test]
    fn histories_accumulate_full_reference_span() {
        let mut app = dummy_app();
        let frames = 450; // 4.5s at 10ms hop
        let snapshot = snapshot_with_alignment(report_with_length(1.0, frames), false);
        app.apply_snapshot(snapshot);

        assert_eq!(frames, app.histories.reference_energy.len());
        assert_eq!(frames, app.histories.learner_energy.len());
        assert_eq!(frames, app.histories.reference_pitch.len());
        assert_eq!(frames, app.histories.learner_pitch.len());
        assert_eq!(frames, app.histories.similarity.len());
        assert_eq!(frames, app.histories.contour.len());
    }

    #[test]
    fn histories_trim_when_exceeding_window() {
        let mut app = dummy_app();
        let frames = HISTORY_CAPACITY_FRAMES + 50;
        let snapshot = snapshot_with_alignment(report_with_length(0.5, frames), false);
        app.apply_snapshot(snapshot);

        assert_eq!(
            HISTORY_CAPACITY_FRAMES,
            app.histories.reference_energy.len()
        );
    }

    #[test]
    fn apply_snapshot_accumulates_across_chunks_while_recording() {
        let mut app = dummy_app();
        let first = snapshot_with_alignment(report_with_length(0.3, 200), true);
        let second = snapshot_with_alignment(report_with_length(0.4, 220), true);

        app.apply_snapshot(first);
        app.apply_snapshot(second);

        let expected = 200 + 220;
        assert_eq!(expected, app.histories.reference_energy.len());
        assert_eq!(expected, app.histories.learner_energy.len());
    }

    #[test]
    fn apply_snapshot_clears_on_recording_restart() {
        let mut app = dummy_app();
        let initial = snapshot_with_alignment(report_with_length(0.7, 180), true);
        let stopped = snapshot_with_alignment(report_with_length(0.2, 40), false);
        let restarted = snapshot_with_alignment(report_with_length(0.5, 150), true);

        app.apply_snapshot(initial);
        app.apply_snapshot(stopped);
        assert!(
            app.histories.reference_energy.len() >= 180,
            "should retain accumulated frames before restart"
        );

        app.apply_snapshot(restarted);
        assert_eq!(
            150,
            app.histories.reference_energy.len(),
            "restart should clear histories and accept new chunk length"
        );
        assert_eq!(150, app.histories.learner_energy.len());
    }

    #[test]
    fn similarity_heatmap_handles_scaled_range() {
        let ctx = egui::Context::default();
        let mut app = dummy_app();
        let frames = 50;
        let sim_values: Vec<f32> = (0..frames).map(|i| 0.2 + 0.8 * (i as f32 / frames as f32)).collect();
        let contour_values: Vec<f32> = (0..frames).map(|i| -0.5 + (i as f32 / frames as f32)).collect();
        let snapshot = AlignmentReport {
            reference_energy: vec![0.0; frames],
            learner_energy: vec![0.0; frames],
            energy_error: vec![0.0; frames],
            reference_pitch: vec![0.0; frames],
            learner_pitch: vec![0.0; frames],
            similarity_band: sim_values,
            contour_band: contour_values,
            start_frame_idx: 0,
            end_frame_idx: frames,
            hop_ms: FRAME_HOP_MS as f32,
            global_time_offset_ms: 0.0,
            total_duration: frames as f32 * FRAME_HOP_MS as f32,
        };
        app.apply_snapshot(snapshot_with_alignment(snapshot, true));

        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                draw_comparison_panel(ui, &app.histories.similarity, &app.histories.contour);
            });
        });
    }

    #[test]
    fn smoothing_applies_to_similarity_plot_series() {
        let data: Vec<f32> = vec![1.0, -1.0, 1.0];
        let deque = VecDeque::from(data.clone());
        let smoothed = super::smooth_series(&deque);
        assert_eq!(smoothed.len(), data.len());
        assert!(
            smoothed.iter().zip(data.iter()).any(|(s, o)| (s - o).abs() > 0.1),
            "smoothing should change jagged data"
        );
    }

    #[test]
    fn shared_range_spans_both_series() {
        let reference = vec![0.0, 1.0];
        let learner = vec![2.0, 3.0];
        let (min, max) = super::shared_range(&reference, &learner);
        assert_eq!(min, 0.0);
        assert_eq!(max, 3.0);
    }

    #[test]
    fn data_range_handles_empty_and_finite_values() {
        let empty = VecDeque::new();
        let (min, max) = super::data_range(&empty);
        assert_eq!((min, max), (0.0, 0.0));

        let mut values = VecDeque::new();
        values.extend([0.2, 0.5, 0.8]);
        let (min, max) = super::data_range(&values);
        assert!((min - 0.2).abs() < 1e-6);
        assert!((max - 0.8).abs() < 1e-6);
    }
}
