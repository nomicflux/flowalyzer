use crate::pronunciation::{PronunciationFeatures, RecordedClip, Result};

/// Responsible for preparing spectral features from recorded audio.
#[derive(Debug, Default)]
pub struct FeatureExtractor {}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(&self, _clip: &RecordedClip) -> Result<PronunciationFeatures> {
        Ok(PronunciationFeatures::default())
    }
}
