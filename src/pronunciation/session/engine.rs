use std::collections::VecDeque;

use crate::pronunciation::alignment::align_features;
use crate::pronunciation::features::{
    compute_energy_frames, compute_pitch_frames, FeatureFrames,
};
use crate::pronunciation::session::AlignmentReport;

use super::SessionConfig;

const FRAME_HOP_SAMPLES: usize = 160; // 10ms at 16kHz
const BOUNDARY_BUFFER_CAPACITY: usize = 2; // max two chunks for boundary overlap

pub struct SessionEngine {
    reference_energy: Vec<f32>,
    reference_pitch: Vec<f32>,
    boundary_chunks: VecDeque<Vec<f32>>,
    global_sample_counter: u64,
    chunk_samples: usize,
    sample_rate: u32,
}

impl SessionEngine {
    pub fn new(reference_samples: &[f32], config: &SessionConfig) -> Self {
        let chunk_samples = (config.sample_rate * config.chunk_duration_ms / 1_000).max(1) as usize;
        let reference_energy = compute_energy_frames(reference_samples);
        let reference_pitch = compute_pitch_frames(reference_samples);
        Self {
            reference_energy,
            reference_pitch,
            boundary_chunks: VecDeque::with_capacity(BOUNDARY_BUFFER_CAPACITY),
            global_sample_counter: 0,
            chunk_samples,
            sample_rate: config.sample_rate,
        }
    }

    pub fn process_chunk(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        let learner_frames = FeatureFrames {
            energy: compute_energy_frames(learner_samples),
            pitch: compute_pitch_frames(learner_samples),
        };
        let reference_frames = self.reference_window(learner_frames.energy.len());
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

    fn reference_window(&self, learner_frames: usize) -> FeatureFrames {
        let start_frame = (self.global_sample_counter as usize) / FRAME_HOP_SAMPLES;
        let start = start_frame.min(self.reference_energy.len());
        let end = (start + learner_frames).min(self.reference_energy.len());
        FeatureFrames {
            energy: self.reference_energy[start..end].to_vec(),
            pitch: self.reference_pitch[start..end].to_vec(),
        }
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
