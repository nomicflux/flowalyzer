pub mod dictionary;
pub mod dtw;
pub mod templates;

pub use dictionary::{normalize_token, PronunciationDictionary, PronunciationVariants};
pub use dtw::{align_templates, frames_to_ms, AlignmentSegment, DtwAlignment};
pub use templates::{build_templates, PhonemeTemplate};

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
