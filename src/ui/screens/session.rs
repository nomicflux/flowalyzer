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
        let _ = draw_history_plot(
            ui,
            "Waveform energy (reference scale)",
            &self.histories.reference_energy,
            &self.histories.learner_energy,
            (Color32::LIGHT_BLUE, Color32::LIGHT_RED),
            false,
            RangeMode::ReferenceOnly,
        );
        let _pitch_range = draw_history_plot(
            ui,
            "Pitch contour (Hz, ref+learner shared range)",
            &self.histories.reference_pitch,
            &self.histories.learner_pitch,
            (Color32::LIGHT_GREEN, Color32::LIGHT_YELLOW),
            false,
            RangeMode::ReferenceAndLearner,
        );
        let _ = draw_history_plot(
            ui,
            "Similarity / contour",
            &self.histories.similarity,
            &self.histories.contour,
            (
                Color32::from_rgb(0xE0, 0x9C, 0x35),
                Color32::from_rgb(0x8A, 0x2B, 0xE2),
            ),
            true,
            RangeMode::ReferenceOnly,
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

#[derive(Clone, Copy)]
enum RangeMode {
    ReferenceOnly,
    ReferenceAndLearner,
}

fn range_from_iter<'a, I>(iter: I) -> (f32, f32)
where
    I: IntoIterator<Item = &'a f32>,
{
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for v in iter {
        min = min.min(*v);
        max = max.max(*v);
    }
    if !min.is_finite() || !max.is_finite() {
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
    range_mode: RangeMode,
) -> (f32, f32) {
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
    let (min, max) = match range_mode {
        RangeMode::ReferenceOnly => range_from_iter(reference_series.iter()),
        RangeMode::ReferenceAndLearner => {
            range_from_iter(reference_series.iter().chain(learner_series.iter()))
        }
    };
    draw_history_line(painter, inner, &reference_series, min, max, colors.0);
    draw_history_line(painter, inner, &learner_series, min, max, colors.1);
    (min, max)
}

fn draw_history_line(
    painter: &Painter,
    rect: Rect,
    history: &[f32],
    min: f32,
    max: f32,
    color: Color32,
) {
    let points = history_points(history, min, max, rect);
    if points.len() < 2 {
        return;
    }
    for segment in points.windows(2) {
        painter.line_segment([segment[0], segment[1]], Stroke::new(1.5, color));
    }
}

fn history_points(history: &[f32], min: f32, max: f32, rect: Rect) -> Vec<Pos2> {
    if history.len() < 2 || (max - min).abs() < 1e-6 {
        return Vec::new();
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
    points
}

fn draw_comparison_panel(ui: &mut egui::Ui, similarity: &VecDeque<f32>, contour: &VecDeque<f32>) {
    ui.label("Spectrogram comparison (absolute ranges)");
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
    let sim_range = series_range(similarity, true);
    let contour_range = series_range(contour, true);
    for row in 0..rows {
        let source = if row == 0 { similarity } else { contour };
        let range = if row == 0 { sim_range } else { contour_range };
        draw_row(RowConfig {
            painter,
            inner,
            row,
            columns,
            source,
            color_fn: if row == 0 {
                |value| similarity_color(value)
            } else {
                |value| contour_color(value)
            },
            range,
            invert_scale: row == 0,
        });
    }

    ui.horizontal(|ui| {
        ui.colored_label(Color32::LIGHT_BLUE, "Similarity (session-scaled)");
        ui.colored_label(Color32::LIGHT_RED, "Contour (session-scaled)");
    });
}

struct RowConfig<'a, F> {
    painter: &'a Painter,
    inner: Rect,
    row: usize,
    columns: usize,
    source: &'a VecDeque<f32>,
    color_fn: F,
    range: (f32, f32),
    invert_scale: bool,
}

fn draw_row<F>(config: RowConfig<'_, F>)
where
    F: Fn(f32) -> Color32,
{
    if config.source.is_empty() {
        return;
    }
    let length = config.source.len();
    let available_columns = config.columns.min(length).max(1);
    let column_width = (config.inner.width() / available_columns as f32).max(1.0);
    let column_height = config.inner.height() / 2.0;
    let (min, max) = config.range;
    let span = (max - min).abs().max(1e-6);
    for col in 0..available_columns {
        let sample_idx = ((col as f32) * length as f32 / available_columns as f32)
            .min((length - 1) as f32) as usize;
        let value = *config.source.get(sample_idx).unwrap();
        let normalized = if config.invert_scale {
            ((max - value) / span).clamp(0.0, 1.0)
        } else {
            ((value - min) / span).clamp(0.0, 1.0)
        };
        let color = (config.color_fn)(normalized);
        let left = config.inner.left() + col as f32 * column_width;
        let right = left + column_width - 1.0;
        let top = config.inner.top() + config.row as f32 * column_height;
        let bottom = top + column_height - 1.0;
        let cell_rect = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom));
        config.painter.rect_filled(cell_rect, 2.0, color);
    }
}

fn similarity_color(value: f32) -> Color32 {
    if value < 0.0 {
        // Explicit "bad" range for negative values (mismatch)
        // Map -1.0 (or lower) to dark red, 0.0 to yellow/neutral
        // Actually, let's make negative distinct red.
        return Color32::from_rgb(180, 32, 32);
    }
    // Positive range: 0.0 (neutral/poor) -> 1.0 (good/perfect)
    gradient_color(
        value,
        &[
            (0.0, Color32::from_rgb(244, 180, 66)), // Yellow/Orange (Poor match)
            (0.5, Color32::from_rgb(160, 200, 60)), // Limeish
            (1.0, Color32::from_rgb(26, 158, 92)),  // Green (Perfect)
        ],
    )
}

fn contour_color(value: f32) -> Color32 {
    // 0.0 is perfect (dark/neutral), 1.0+ is error (bright/hot)
    gradient_color(
        value.abs(), // Contour error is magnitude
        &[
            (0.0, Color32::from_rgb(20, 20, 30)),   // Dark/Neutral (Perfect)
            (0.5, Color32::from_rgb(64, 192, 255)), // Blueish (Moderate)
            (1.0, Color32::from_rgb(240, 96, 32)),  // Orange/Red (Bad)
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

fn series_range(values: &VecDeque<f32>, include_zero: bool) -> (f32, f32) {
    if values.is_empty() {
        return (0.0, 1.0);
    }
    let mut min = values.iter().fold(f32::INFINITY, |m, v| m.min(*v));
    let mut max = values.iter().fold(f32::NEG_INFINITY, |m, v| m.max(*v));
    if include_zero {
        min = min.min(0.0);
        max = max.max(0.0);
    }
    if (max - min).abs() < 1e-6 {
        max = min + 1.0;
    }
    (min, max)
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
    fn energy_plot_uses_reference_range_for_both_series() {
        let ctx = egui::Context::default();
        let mut reference = VecDeque::new();
        reference.extend([0.25, 1.25]);
        let mut learner = VecDeque::new();
        learner.extend([5.0, 6.0]);

        let mut captured_range = None;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                captured_range = Some(draw_history_plot(
                    ui,
                    "Waveform energy",
                    &reference,
                    &learner,
                    (Color32::LIGHT_BLUE, Color32::LIGHT_RED),
                    false,
                    RangeMode::ReferenceOnly,
                ));
            });
        });

        let (min, max) = captured_range.expect("range should be produced");
        assert!((min - 0.25).abs() < 1e-6, "range should start from reference min");
        assert!((max - 1.25).abs() < 1e-6, "range should end at reference max");
    }

    #[test]
    fn learner_energy_stays_near_baseline_when_reference_sets_scale() {
        let rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
        let reference = vec![0.0, 1.0];
        let learner = vec![0.0, 0.01];
        let (min, max) = super::range_from_iter(reference.iter());

        let reference_points = super::history_points(&reference, min, max, rect);
        let learner_points = super::history_points(&learner, min, max, rect);

        assert_eq!(reference_points.len(), 2);
        assert_eq!(learner_points.len(), 2);

        assert!(
            (reference_points.first().unwrap().y - rect.bottom()).abs() < 1e-3,
            "reference min should map to plot baseline"
        );
        assert!(
            (reference_points.last().unwrap().y - rect.top()).abs() < 1e-3,
            "reference max should map to plot top"
        );

        let learner_span = learner_points
            .iter()
            .map(|p| p.y)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), y| {
                (lo.min(y), hi.max(y))
            });
        assert!(
            (learner_span.1 - learner_span.0) < 2.0,
            "near-zero learner energy should stay near baseline when scaled to reference"
        );
    }

    #[test]
    fn pitch_range_uses_reference_and_learner_spans() {
        let ctx = egui::Context::default();
        let mut reference = VecDeque::new();
        reference.extend([0.0, 110.0]);
        let mut learner = VecDeque::new();
        learner.extend([5.0, 200.0]);

        let mut captured_range_and_points = None;
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let range = draw_history_plot(
                        ui,
                        "Pitch contour (Hz)",
                        &reference,
                        &learner,
                        (Color32::LIGHT_GREEN, Color32::LIGHT_YELLOW),
                        false,
                        RangeMode::ReferenceAndLearner,
                    );
                captured_range_and_points = Some((range, ui.min_rect()));
            });
        });

        let ((min, max), rect) = captured_range_and_points.expect("range should be produced");
        assert!(
            (min - 0.0).abs() < 1e-6 && (max - 200.0).abs() < 1e-6,
            "pitch range should span both reference and learner values without voiced filtering"
        );

        // Rendered points should stay within plot bounds using shared range
        let rect = Rect::from_min_max(rect.left_top(), rect.right_bottom());
        let learner_points = super::history_points(&learner.make_contiguous(), min, max, rect);
        assert!(
            learner_points.iter().all(|p| p.y <= rect.bottom() && p.y >= rect.top()),
            "learner points should render within plot using shared range"
        );
    }
    #[test]
    fn similarity_color_maps_negative_to_bad_and_positive_to_good() {
        // Given negative similarity (mismatch)
        let mismatch = -1.0;
        // When mapped to color
        let color_bad = super::similarity_color(mismatch);
        // Then it should be reddish (high R, low G/B)
        assert!(color_bad.r() > 150, "bad score should be red");
        assert!(color_bad.g() < 100, "bad score should not be green");

        // Given positive similarity (match)
        let match_score = 1.0;
        // When mapped to color
        let color_good = super::similarity_color(match_score);
        // Then it should be greenish (low R, high G)
        assert!(color_good.g() > 150, "good score should be green");
        assert!(color_good.r() < 100, "good score should not be red");
    }

    #[test]
    fn contour_color_maps_zero_to_neutral_and_extremes_to_distinct() {
        // Given zero contour error (perfect match)
        let perfect = 0.0;
        let color_perfect = super::contour_color(perfect);
        // Then it should be neutral/dark (e.g., dark blue/black)
        assert!(color_perfect.b() >= 30, "zero contour should be dark/neutral base");

        // Given high contour error
        let error = 1.0;
        let color_error = super::contour_color(error);
        // Then it should be bright/distinct (e.g., orange/red)
        assert!(color_error.r() > 150, "high contour error should be bright/red");
        #[test]
    fn flat_lines_normalize_to_same_color_under_relative_scaling() {
        // This test demonstrates the bug:
        // Constant -1.0 (Silence) and Constant 1.0 (Perfect) both normalize to 0.0
        // if we use min/max scaling on the series itself.
        
        let silence = VecDeque::from(vec![-1.0, -1.0, -1.0]);
        let perfect = VecDeque::from(vec![1.0, 1.0, 1.0]);
        
        let (min_s, max_s) = super::series_range(&silence, true);
        let (min_p, max_p) = super::series_range(&perfect, true);
        
        // Both have span ~0 (or small epsilon)
        // In the current draw_row logic:
        // value - min / span
        // For silence: -1.0 - (-1.0) = 0.0
        // For perfect: 1.0 - 1.0 = 0.0
        
        // We can't easily test draw_row's internal pixel output without mocking Painter,
        // but we can test the math logic it uses.
        
        let normalize = |val: f32, min: f32, max: f32| {
            let span = (max - min).abs().max(1e-6);
            ((val - min) / span).clamp(0.0, 1.0)
        };
        
        let norm_silence = normalize(-1.0, min_s, max_s);
        let norm_perfect = normalize(1.0, min_p, max_p);
        
        assert_eq!(norm_silence, 0.0);
        assert_eq!(norm_perfect, 0.0);
        
        // This confirms they would render with the exact same color (the color for 0.0).
    }
}
}
