use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, PhonemeScore, PronunciationScores, Result,
};

const TIMING_TOLERANCE_MS: f32 = 120.0;
const ARTICULATION_TOLERANCE: f32 = 1.0;
const ENERGY_TOLERANCE: f32 = 0.35;
const CONFIDENCE_WEIGHT: f32 = 0.35;
const TIMING_WEIGHT: f32 = 0.4;
const ARTICULATION_WEIGHT: f32 = 0.3;
const PROSODY_WEIGHT: f32 = 0.3;

#[derive(Debug, Default)]
pub struct MetricCalculator {}

impl MetricCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn score(&self, report: &AlignmentReport) -> Result<PronunciationScores> {
        if report.phonemes.is_empty() {
            return Ok(empty_scores(report.confidence));
        }
        let timing = timing_score(&report.phonemes);
        let articulation = articulation_score(&report.phonemes);
        let prosody = prosody_score(report);
        let overall = overall_score(report.confidence, timing, articulation, prosody);
        Ok(PronunciationScores {
            overall,
            timing,
            articulation,
            intonation: prosody,
            per_phoneme: per_phoneme_scores(&report.phonemes),
        })
    }
}

fn empty_scores(confidence: f32) -> PronunciationScores {
    PronunciationScores {
        overall: confidence.clamp(0.0, 1.0),
        timing: 1.0,
        articulation: 1.0,
        intonation: 1.0,
        per_phoneme: Vec::new(),
    }
}

fn timing_score(phonemes: &[AlignedPhoneme]) -> f32 {
    average_band(phonemes.iter().map(|p| band_from_delta(p.timing_delta_ms)))
}

fn articulation_score(phonemes: &[AlignedPhoneme]) -> f32 {
    average_band(
        phonemes
            .iter()
            .map(|p| 1.0 - (p.articulation_variance / ARTICULATION_TOLERANCE).min(1.0)),
    )
}

fn prosody_score(report: &AlignmentReport) -> f32 {
    let contour_score = if report.contour_band.is_empty() {
        None
    } else {
        Some(average_band(report.contour_band.iter().copied()))
    };
    let energy_score = energy_alignment_score(report);
    contour_score
        .map(|contour| blend_contour_with_energy(contour, energy_score))
        .unwrap_or(
            energy_score.unwrap_or_else(|| average_band(report.similarity_band.iter().copied())),
        )
}

fn energy_alignment_score(report: &AlignmentReport) -> Option<f32> {
    if report.reference_energy.is_empty() || report.learner_energy.is_empty() {
        return None;
    }
    let len = report
        .reference_energy
        .len()
        .min(report.learner_energy.len());
    if len == 0 {
        return None;
    }
    let mut total = 0.0;
    for idx in 0..len {
        let delta =
            (report.reference_energy[idx] - report.learner_energy[idx]).abs() / ENERGY_TOLERANCE;
        total += 1.0 - delta.min(1.0);
    }
    Some((total / len as f32).clamp(0.0, 1.0))
}

fn blend_contour_with_energy(contour: f32, energy: Option<f32>) -> f32 {
    match energy {
        Some(energy_score) => (contour * 0.7 + energy_score * 0.3).clamp(0.0, 1.0),
        None => contour,
    }
}

fn average_band<I>(values: I) -> f32
where
    I: Iterator<Item = f32>,
{
    let mut total = 0.0;
    let mut count = 0.0;
    for value in values {
        total += value.clamp(0.0, 1.0);
        count += 1.0;
    }
    if count == 0.0 {
        1.0
    } else {
        (total / count).clamp(0.0, 1.0)
    }
}

fn band_from_delta(delta_ms: f32) -> f32 {
    1.0 - (delta_ms.abs() / TIMING_TOLERANCE_MS).min(1.0)
}

fn per_phoneme_scores(phonemes: &[AlignedPhoneme]) -> Vec<PhonemeScore> {
    phonemes
        .iter()
        .map(|phoneme| PhonemeScore {
            symbol: phoneme.symbol.clone(),
            timing: band_from_delta(phoneme.timing_delta_ms),
            articulation: (1.0 - phoneme.articulation_variance).clamp(0.0, 1.0),
            intonation: phoneme.contour_similarity.clamp(0.0, 1.0),
        })
        .collect()
}

fn overall_score(confidence: f32, timing: f32, articulation: f32, prosody: f32) -> f32 {
    let composite =
        TIMING_WEIGHT * timing + ARTICULATION_WEIGHT * articulation + PROSODY_WEIGHT * prosody;
    let blended =
        composite * (1.0 - CONFIDENCE_WEIGHT) + confidence.clamp(0.0, 1.0) * CONFIDENCE_WEIGHT;
    blended.clamp(0.0, 1.0)
}
