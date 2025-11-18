mod energy;
mod pitch;

pub use energy::compute_energy_frames;
pub use pitch::{compute_pitch_frames, compute_pitch_frames_with_rate};

#[derive(Debug, Clone, Default)]
pub struct FeatureFrames {
    pub energy: Vec<f32>,
    pub pitch: Vec<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureExtractor;

impl FeatureExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(&self, samples: &[f32]) -> FeatureFrames {
        FeatureFrames {
            energy: compute_energy_frames(samples),
            pitch: compute_pitch_frames(samples),
        }
    }
}
