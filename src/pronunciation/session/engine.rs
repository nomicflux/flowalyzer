use crate::pronunciation::alignment::align_features;
use crate::pronunciation::features::{FeatureConfig, FeatureExtractor, ReferenceFeatures};
use crate::pronunciation::session::AlignmentReport;

pub struct SessionEngine {
    reference_features: ReferenceFeatures,
    tail: Vec<f32>,
    global_sample_counter: u64,
    sample_rate: u32,
    feature_cfg: FeatureConfig,
    extractor: FeatureExtractor,
}

impl SessionEngine {
    pub fn new(reference_features: ReferenceFeatures, sample_rate: u32) -> Self {
        let feature_cfg = FeatureConfig::from_sample_rate(sample_rate);
        let extractor = FeatureExtractor::new();
        Self {
            reference_features,
            tail: Vec::new(),
            global_sample_counter: 0,
            sample_rate,
            feature_cfg,
            extractor,
        }
    }

    pub fn seed_tail(&mut self, samples: &[f32]) {
        let required_tail_len = self.feature_cfg.frame_len_samples - self.feature_cfg.hop_samples;
        let start = samples.len() - required_tail_len;
        self.tail = samples[start..].to_vec();
        self.global_sample_counter = samples.len() as u64;
    }

    pub fn process_chunk(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        let required_tail_len = self.feature_cfg.frame_len_samples - self.feature_cfg.hop_samples;
        let learner_tail = &self.tail;
        let mut window = Vec::with_capacity(learner_tail.len() + learner_samples.len());
        window.extend_from_slice(learner_tail);
        window.extend_from_slice(learner_samples);

        // Calculate phase offset: where does the window start relative to the global grid?
        // window_start = global_sample_counter - tail_len
        // phase_offset = window_start % hop
        let tail_len = learner_tail.len();
        let hop = self.feature_cfg.hop_samples;
        let window_start_global = self
            .global_sample_counter
            .checked_sub(tail_len as u64)
            .unwrap();
        let phase_offset = (window_start_global % hop as u64) as usize;

        let learner_features = self.extractor.extract_chunk(
            learner_tail,
            learner_samples,
            self.sample_rate,
            self.feature_cfg,
            phase_offset,
        );

        // Calculate start_frame_idx based on the first extracted frame.
        // Global pos of a frame starting at `chunk_start` (relative to chunk anchor defined in extract_chunk):
        // G_pos = global_sample_counter + chunk_start - hop.
        // See derivation in thought process.
        let start_frame_idx = if let Some(&first_start) = learner_features.frame_starts.first() {
            let global_pos = (self.global_sample_counter as i64 + first_start as i64 - hop as i64) as u64;
            (global_pos / hop as u64) as usize
        } else {
            (self.global_sample_counter / hop as u64) as usize
        };

        let report = align_features(
            &self.reference_features,
            &learner_features,
            start_frame_idx,
            self.global_offset_ms(),
            self.sample_rate,
            self.feature_cfg.hop_samples,
        );
        self.tail = window[window.len() - required_tail_len..].to_vec();
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

impl SessionEngine {
    pub fn tail_samples(&self) -> &[f32] {
        &self.tail
    }

    pub fn global_sample_counter(&self) -> u64 {
        self.global_sample_counter
    }
}
