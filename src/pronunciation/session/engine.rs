use std::collections::VecDeque;

use crate::pronunciation::alignment::align_features;
use crate::pronunciation::features::{FeatureConfig, FeatureExtractor, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

use super::SessionConfig;

const BOUNDARY_BUFFER_CAPACITY: usize = 2; // max two chunks for boundary overlap

pub struct SessionEngine {
    reference_samples: Vec<f32>,
    boundary_chunks: VecDeque<Vec<f32>>,
    global_sample_counter: u64,
    sample_rate: u32,
    feature_cfg: FeatureConfig,
    extractor: FeatureExtractor,
}

impl SessionEngine {
    pub fn new(reference_samples: &[f32], config: &SessionConfig) -> Self {
        let chunk_samples =
            (config.sample_rate as usize * config.chunk_duration_ms as usize) / 1_000;
        assert!(
            chunk_samples >= 1,
            "chunk duration {} ms is too short to produce frames at {} Hz",
            config.chunk_duration_ms,
            config.sample_rate
        );
        let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
        assert!(
            chunk_samples >= feature_cfg.frame_len_samples,
            "chunk duration {} ms is too short for frame length {} samples",
            config.chunk_duration_ms,
            feature_cfg.frame_len_samples
        );
        Self {
            reference_samples: reference_samples.to_vec(),
            boundary_chunks: VecDeque::with_capacity(BOUNDARY_BUFFER_CAPACITY),
            global_sample_counter: 0,
            sample_rate: config.sample_rate,
            feature_cfg,
            extractor: FeatureExtractor::new(),
        }
    }

    pub fn process_chunk(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        assert!(!learner_samples.is_empty(), "chunk must not be empty");
        let learner_tail = self.learner_tail(learner_samples);
        let learner_features = self.extractor.extract_chunk(
            &learner_tail,
            learner_samples,
            self.sample_rate,
            self.feature_cfg,
        );

        let reference_features =
            self.reference_chunk_features(learner_samples.len(), learner_tail.len());

        let report = align_features(
            &reference_features,
            &learner_features,
            0,
            self.global_offset_ms(),
            self.sample_rate,
            self.feature_cfg.hop_samples,
        );
        self.remember_boundary_chunk(learner_samples);
        self.advance_counter(learner_samples.len());
        report
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn reset(&mut self) {
        self.global_sample_counter = 0;
        self.boundary_chunks.clear();
    }

    fn learner_tail(&self, upcoming_chunk: &[f32]) -> Vec<f32> {
        let needed = self.feature_cfg.frame_len_samples - self.feature_cfg.hop_samples;
        let mut tail: Vec<f32> = self
            .boundary_chunks
            .iter()
            .flat_map(|chunk| chunk.iter())
            .copied()
            .collect();
        if tail.len() < needed {
            let deficit = needed - tail.len();
            assert!(
                upcoming_chunk.len() >= deficit,
                "insufficient learner context (tail {} + chunk {} < required {})",
                tail.len(),
                upcoming_chunk.len(),
                needed
            );
            tail.extend_from_slice(&upcoming_chunk[..deficit]);
        }
        if tail.len() > needed {
            tail = tail[tail.len() - needed..].to_vec();
        }
        tail
    }

    fn reference_chunk_features(&self, learner_len: usize, tail_len: usize) -> ReferenceFeatures {
        let start = self.global_sample_counter as usize;
        let end = start + learner_len;
        assert!(
            end <= self.reference_samples.len(),
            "reference audio exhausted"
        );

        let chunk = self.reference_samples[start..end].to_vec();
        let mut tail: Vec<f32> = if start >= tail_len {
            self.reference_samples[start - tail_len..start].to_vec()
        } else {
            let deficit = tail_len - start;
            assert!(
                chunk.len() >= deficit,
                "reference chunk too short to cover tail deficit"
            );
            let mut collected = self.reference_samples[0..start].to_vec();
            collected.extend_from_slice(&chunk[..deficit]);
            collected
        };
        if tail.len() > tail_len {
            tail = tail[tail.len() - tail_len..].to_vec();
        }
        assert_eq!(tail.len(), tail_len, "reference tail length mismatch");

        let chunk_features =
            self.extractor
                .extract_chunk(&tail, &chunk, self.sample_rate, self.feature_cfg);
        ReferenceFeatures {
            energy: chunk_features.energy,
            pitch: chunk_features.pitch,
            frame_starts: chunk_features.frame_starts,
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
