use crate::pronunciation::{AlignmentReport, PronunciationScores, Result};

/// Aggregates alignment outcomes into learner-friendly metrics.
#[derive(Debug, Default)]
pub struct MetricCalculator {}

impl MetricCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn score(&self, report: &AlignmentReport) -> Result<PronunciationScores> {
        let per_phoneme = report
            .phonemes
            .iter()
            .map(|phoneme| {
                let timing_score = (1.0 - phoneme.timing_delta_ms.abs() / 100.0).clamp(0.0, 1.0);
                let articulation_score = (1.0 - phoneme.articulation_variance).clamp(0.0, 1.0);
                crate::pronunciation::PhonemeScore {
                    symbol: phoneme.symbol.clone(),
                    timing: timing_score,
                    articulation: articulation_score,
                    intonation: phoneme.similarity.clamp(0.0, 1.0),
                }
            })
            .collect();
        Ok(PronunciationScores {
            overall: report.confidence,
            timing: 1.0,
            articulation: 1.0,
            intonation: 1.0,
            per_phoneme,
        })
    }
}
