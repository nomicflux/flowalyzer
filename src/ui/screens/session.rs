use std::collections::VecDeque;

use crate::pronunciation::session::{
    AlignmentReport, SessionConfig, SessionController, SessionHandle, SessionSnapshot,
};

const HISTORY_WINDOW_MS: usize = 30_000;
const FRAME_HOP_MS: usize = 10;
const HISTORY_CAPACITY_FRAMES: usize = HISTORY_WINDOW_MS / FRAME_HOP_MS;

pub struct SessionApp {
    handle: SessionHandle,
    controller: SessionController,
    snapshot: SessionSnapshot,
    control_error: Option<String>,
    histories: HistoryBuffers,
}

impl SessionApp {
    pub fn new(handle: SessionHandle, controller: SessionController) -> Self {
        Self {
            snapshot: handle.initial_snapshot(),
            handle,
            controller,
            control_error: None,
            histories: HistoryBuffers::new(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: SessionSnapshot) {
        self.histories.accumulate(&snapshot.alignment);
        self.snapshot = snapshot;
    }

    pub fn clear_histories(&mut self) {
        self.histories.clear();
    }

    pub fn control_error(&self) -> Option<&str> {
        self.control_error.as_deref()
    }

    pub fn snapshot(&self) -> &SessionSnapshot {
        &self.snapshot
    }

    pub fn config(&self) -> &SessionConfig {
        self.handle.config()
    }
}

impl Drop for SessionApp {
    fn drop(&mut self) {
        let _ = self.controller.shutdown();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pronunciation::session::{AlignmentReport, SessionConfig, SessionRuntime};
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
