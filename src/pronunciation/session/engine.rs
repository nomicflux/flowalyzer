use crate::pronunciation::alignment::align_features;
use crate::pronunciation::features::{FeatureConfig, FeatureExtractor, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

use super::SessionConfig;

pub struct SessionEngine {
    reference_features: ReferenceFeatures,
    tail: Vec<f32>,
    global_sample_counter: u64,
    sample_rate: u32,
    feature_cfg: FeatureConfig,
    extractor: FeatureExtractor,
}

impl SessionEngine {
    pub fn new(reference_samples: &[f32], config: &SessionConfig) -> Self {
        let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
        let extractor = FeatureExtractor::new();
        let reference_features =
            extractor.extract_reference(reference_samples, config.sample_rate, feature_cfg);
        Self {
            reference_features,
            tail: Vec::new(),
            global_sample_counter: 0,
            sample_rate: config.sample_rate,
            feature_cfg,
            extractor,
        }
    }

    pub fn seed_tail(&mut self, tail: &[f32]) {
        self.tail.clear();
        self.tail.extend_from_slice(tail);
        self.global_sample_counter = self.tail.len() as u64;
    }

    pub fn process_chunk(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        let required_tail_len = self.feature_cfg.frame_len_samples - self.feature_cfg.hop_samples;
        let learner_tail = &self.tail;
        let mut window = Vec::with_capacity(learner_tail.len() + learner_samples.len());
        window.extend_from_slice(learner_tail);
        window.extend_from_slice(learner_samples);
        let learner_features = self.extractor.extract_chunk(
            learner_tail,
            learner_samples,
            self.sample_rate,
            self.feature_cfg,
        );

        let start_frame_idx =
            (self.global_sample_counter / self.feature_cfg.hop_samples as u64) - 1;
        let report = align_features(
            &self.reference_features,
            &learner_features,
            start_frame_idx as usize,
            self.global_offset_ms(),
            self.sample_rate,
            self.feature_cfg.hop_samples,
        );
        self.tail = window[window.len().saturating_sub(required_tail_len)..].to_vec();
        self.global_sample_counter += learner_samples.len() as u64;
        report
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn reset(&mut self) {
        self.global_sample_counter = 0;
        self.tail.clear();
    }

    pub fn required_tail_len(&self) -> usize {
        self.feature_cfg.frame_len_samples - self.feature_cfg.hop_samples
    }

    fn global_offset_ms(&self) -> f32 {
        (self.global_sample_counter as f32 / self.sample_rate as f32) * 1_000.0
    }
}
