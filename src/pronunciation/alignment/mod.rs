use crate::pronunciation::{AlignmentReport, PronunciationFeatures, Result};

/// Performs phoneme-level comparisons between learner and reference audio.
#[derive(Debug, Default)]
pub struct PhonemeAligner {}

impl PhonemeAligner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn align(
        &self,
        _reference: &PronunciationFeatures,
        _learner: &PronunciationFeatures,
    ) -> Result<AlignmentReport> {
        Ok(AlignmentReport::default())
    }
}
