use std::collections::VecDeque;

use crate::pronunciation::alignment::align_features;
use crate::pronunciation::features::{compute_energy_frames, compute_pitch_frames, FeatureFrames};
use crate::pronunciation::session::AlignmentReport;

use super::SessionConfig;

const FRAME_HOP_SAMPLES: usize = 160; // 10ms at 16kHz
const BOUNDARY_BUFFER_CAPACITY: usize = 2; // max two chunks for boundary overlap

pub struct SessionEngine {
    reference_samples: Vec<f32>,
    boundary_chunks: VecDeque<Vec<f32>>,
    global_sample_counter: u64,
    chunk_samples: usize,
    sample_rate: u32,
}

impl SessionEngine {
    pub fn new(reference_samples: &[f32], config: &SessionConfig) -> Self {
        let chunk_samples = (config.sample_rate * config.chunk_duration_ms / 1_000).max(1) as usize;
        Self {
            reference_samples: reference_samples.to_vec(),
            boundary_chunks: VecDeque::with_capacity(BOUNDARY_BUFFER_CAPACITY),
            global_sample_counter: 0,
            chunk_samples,
            sample_rate: config.sample_rate,
        }
    }

    pub fn process_chunk(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        let context_prefix = self.prefix_samples();
        let mut learner_window = context_prefix;
        learner_window.extend_from_slice(learner_samples);

        let (ref_window, trimmed_len) = self.reference_window(&learner_window);

        let learner_frames_full = FeatureFrames {
            energy: compute_energy_frames(&learner_window),
            pitch: compute_pitch_frames(&learner_window),
        };
        let reference_frames_full = FeatureFrames {
            energy: compute_energy_frames(&ref_window),
            pitch: compute_pitch_frames(&ref_window),
        };

        let prefix_frames = prefix_frame_count(learner_samples.len());
        let learner_frames = trim_prefix(learner_frames_full, prefix_frames);
        let reference_frames = trim_prefix(reference_frames_full, prefix_frames.min(trimmed_len));

        let report = align_features(&reference_frames, &learner_frames, self.global_offset_ms());
        self.remember_boundary_chunk(learner_samples);
        self.advance_counter(learner_samples.len());
        report
    }

    pub fn chunk_samples(&self) -> usize {
        self.chunk_samples
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn reset(&mut self) {
        self.global_sample_counter = 0;
        self.boundary_chunks.clear();
    }

    fn prefix_samples(&self) -> Vec<f32> {
        let mut prefix = Vec::new();
        for chunk in &self.boundary_chunks {
            prefix.extend_from_slice(chunk);
        }
        prefix
    }

    fn reference_window(&self, learner_window: &[f32]) -> (Vec<f32>, usize) {
        let prefix_len = learner_window.len().saturating_sub(self.chunk_samples);
        let start = self.global_sample_counter.saturating_sub(prefix_len as u64) as usize;
        let end = (start + learner_window.len()).min(self.reference_samples.len());
        let window = self.reference_samples[start..end].to_vec();
        (window, end.saturating_sub(start))
    }

    fn remember_boundary_chunk(&mut self, samples: &[f32]) {
        if BOUNDARY_BUFFER_CAPACITY == 0 {
            return;
        }
        if self.boundary_chunks.len() == BOUNDARY_BUFFER_CAPACITY {
            self.boundary_chunks.pop_front();
        }
        self.boundary_chunks.push_back(samples.to_vec());
    }

    fn global_offset_ms(&self) -> f32 {
        (self.global_sample_counter as f32 / self.sample_rate as f32) * 1_000.0
    }

    fn advance_counter(&mut self, samples_processed: usize) {
        self.global_sample_counter += samples_processed as u64;
    }
}

fn prefix_frame_count(chunk_samples: usize) -> usize {
    chunk_samples.div_ceil(FRAME_HOP_SAMPLES)
}

fn trim_prefix(mut frames: FeatureFrames, prefix_frames: usize) -> FeatureFrames {
    let trim = prefix_frames.min(frames.energy.len());
    if trim > 0 {
        frames.energy.drain(0..trim);
        frames.pitch.drain(0..trim);
    }
    frames
}
