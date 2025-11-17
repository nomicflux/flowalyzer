use crate::pronunciation::alignment::align_chunk;
use crate::pronunciation::features::{compute_energy_frames, compute_pitch_frames};
use crate::pronunciation::session::AlignmentReport;

use super::{ChunkMemory, ChunkMemoryLimit, SessionConfig};

pub struct SessionEngine {
    reference_samples: Vec<f32>,
    chunk_memory: ChunkMemory,
    pending_chunk: Option<Vec<f32>>,
    delay_processing: bool,
    global_sample_counter: u64,
    chunk_samples: usize,
    sample_rate: u32,
}

impl SessionEngine {
    pub fn new(reference_samples: &[f32], config: &SessionConfig) -> Self {
        let chunk_samples = (config.sample_rate * config.chunk_duration_ms / 1_000).max(1) as usize;
        let delay_processing =
            matches!(config.chunk_memory_limit, ChunkMemoryLimit::PreviousAndNext);
        Self {
            reference_samples: reference_samples.to_vec(),
            chunk_memory: ChunkMemory::new(config.chunk_memory_limit.max_chunks()),
            pending_chunk: None,
            delay_processing,
            global_sample_counter: 0,
            chunk_samples,
            sample_rate: config.sample_rate,
        }
    }

    pub fn ingest_chunk(&mut self, learner_samples: Vec<f32>) -> Option<AlignmentReport> {
        if self.delay_processing {
            if let Some(pending) = self.pending_chunk.take() {
                let report = self.process_now(&pending);
                self.pending_chunk = Some(learner_samples);
                Some(report)
            } else {
                self.pending_chunk = Some(learner_samples);
                None
            }
        } else {
            Some(self.process_now(&learner_samples))
        }
    }

    pub fn flush_pending(&mut self) -> Option<AlignmentReport> {
        self.pending_chunk
            .take()
            .map(|pending| self.process_now(&pending))
    }

    fn process_now(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        let reference_slice = self.reference_slice(learner_samples.len());
        let reference_energy = compute_energy_frames(reference_slice);
        let learner_energy = compute_energy_frames(learner_samples);
        let reference_pitch = compute_pitch_frames(reference_slice);
        let learner_pitch = compute_pitch_frames(learner_samples);
        let report = align_chunk(
            &reference_energy,
            &learner_energy,
            &reference_pitch,
            &learner_pitch,
            self.global_offset_ms(),
        );
        self.advance_counter(learner_samples.len());
        self.chunk_memory.remember(learner_samples);
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
        self.pending_chunk = None;
        self.chunk_memory.clear();
    }

    pub fn previous_chunk(&self) -> Option<&[f32]> {
        self.chunk_memory.previous()
    }

    fn reference_slice(&self, samples_requested: usize) -> &[f32] {
        let start = self.global_sample_counter as usize;
        let end = (start + samples_requested).min(self.reference_samples.len());
        &self.reference_samples[start..end]
    }

    fn global_offset_ms(&self) -> f32 {
        (self.global_sample_counter as f32 / self.sample_rate as f32) * 1_000.0
    }

    fn advance_counter(&mut self, samples_processed: usize) {
        self.global_sample_counter += samples_processed as u64;
    }
}
