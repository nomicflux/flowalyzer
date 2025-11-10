use crate::pronunciation::{AlignmentReport, PronunciationScores, Result};

/// Aggregates alignment outcomes into learner-friendly metrics.
#[derive(Debug, Default)]
pub struct MetricCalculator {}

impl MetricCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn score(&self, _report: &AlignmentReport) -> Result<PronunciationScores> {
        Ok(PronunciationScores::default())
    }
}
