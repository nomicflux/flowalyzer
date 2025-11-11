use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, PhonemeScore, PronunciationScores, Result,
};

const TIMING_TOLERANCE_MS: f32 = 120.0;
const ARTICULATION_TOLERANCE: f32 = 1.0;
const CONFIDENCE_WEIGHT: f32 = 0.3;
const TIMING_WEIGHT: f32 = 0.4;
const ARTICULATION_WEIGHT: f32 = 0.3;
const INTONATION_WEIGHT: f32 = 0.3;

/// Aggregates alignment outcomes into learner-friendly metrics.
#[derive(Debug, Default)]
pub struct MetricCalculator {}

impl MetricCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn score(&self, report: &AlignmentReport) -> Result<PronunciationScores> {
        if report.phonemes.is_empty() {
            return Ok(PronunciationScores {
                overall: report.confidence.clamp(0.0, 1.0),
                timing: 1.0,
                articulation: 1.0,
                intonation: 1.0,
                per_phoneme: Vec::new(),
            });
        }

        let timing = score_timing(&report.phonemes);
        let articulation = score_articulation(&report.phonemes);
        let intonation = score_intonation(&report.phonemes);
        let overall = overall_score(report.confidence, timing, articulation, intonation);
        Ok(PronunciationScores {
            overall,
            timing,
            articulation,
            intonation,
            per_phoneme: score_phonemes(&report.phonemes),
        })
    }
}

fn score_timing(phonemes: &[AlignedPhoneme]) -> f32 {
    if phonemes.is_empty() {
        return 1.0;
    }
    let mean = phonemes
        .iter()
        .map(|p| p.timing_delta_ms.abs())
        .sum::<f32>()
        / phonemes.len() as f32;
    (1.0 - (mean / TIMING_TOLERANCE_MS).min(1.0)).clamp(0.0, 1.0)
}

fn score_articulation(phonemes: &[AlignedPhoneme]) -> f32 {
    if phonemes.is_empty() {
        return 1.0;
    }
    let mean = phonemes
        .iter()
        .map(|p| p.articulation_variance)
        .sum::<f32>()
        / phonemes.len() as f32;
    (1.0 - (mean / ARTICULATION_TOLERANCE).min(1.0)).clamp(0.0, 1.0)
}

fn score_intonation(phonemes: &[AlignedPhoneme]) -> f32 {
    if phonemes.is_empty() {
        return 1.0;
    }
    (phonemes
        .iter()
        .map(|p| p.similarity.clamp(0.0, 1.0))
        .sum::<f32>()
        / phonemes.len() as f32)
        .clamp(0.0, 1.0)
}

fn score_phonemes(phonemes: &[AlignedPhoneme]) -> Vec<PhonemeScore> {
    phonemes
        .iter()
        .map(|phoneme| PhonemeScore {
            symbol: phoneme.symbol.clone(),
            timing: score_single_timing(phoneme),
            articulation: (1.0 - phoneme.articulation_variance).clamp(0.0, 1.0),
            intonation: phoneme.similarity.clamp(0.0, 1.0),
        })
        .collect()
}

fn score_single_timing(phoneme: &AlignedPhoneme) -> f32 {
    let delta = phoneme.timing_delta_ms.abs();
    (1.0 - (delta / TIMING_TOLERANCE_MS).min(1.0)).clamp(0.0, 1.0)
}

fn overall_score(confidence: f32, timing: f32, articulation: f32, intonation: f32) -> f32 {
    let composite = TIMING_WEIGHT * timing
        + ARTICULATION_WEIGHT * articulation
        + INTONATION_WEIGHT * intonation;
    (composite * (1.0 - CONFIDENCE_WEIGHT) + confidence.clamp(0.0, 1.0) * CONFIDENCE_WEIGHT)
        .clamp(0.0, 1.0)
}
