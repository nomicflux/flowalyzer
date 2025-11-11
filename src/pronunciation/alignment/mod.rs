pub mod dictionary;
pub mod dtw;
pub mod templates;

pub use dictionary::{normalize_token, PronunciationDictionary, PronunciationVariants};
pub use dtw::{align_templates, frames_to_ms, AlignmentSegment, DtwAlignment};
pub use templates::{build_templates, PhonemeTemplate};

use std::time::Duration;

use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, PronunciationError, PronunciationFeatures, Result,
};

/// Performs phoneme-level comparisons between learner and reference audio.
#[derive(Debug, Default)]
pub struct PhonemeAligner {}

impl PhonemeAligner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn align(
        &self,
        transcript: &str,
        reference: &PronunciationFeatures,
        learner: &PronunciationFeatures,
    ) -> Result<AlignmentReport> {
        validate_features(reference, learner)?;
        let tokens = tokenize_transcript(transcript)?;
        let dictionary = PronunciationDictionary::shared();
        let pronunciations = dictionary.map_tokens(tokens.iter().copied())?;
        let phoneme_sequence = select_pronunciations(&tokens, &pronunciations)?;

        let templates = build_templates(reference, &phoneme_sequence)?;
        let alignment = align_templates(&templates, learner)?;
        let phonemes = assemble_phoneme_reports(&templates, &alignment);

        let global_time_offset_ms = average_timing_delta(&phonemes);
        let learner_path_cost = alignment.segments.iter().map(|segment| segment.cost).sum();
        let total_duration_ms = alignment
            .segments
            .last()
            .map(|segment| frames_to_ms(segment.learner_start_frame, segment.learner_end_frame).1)
            .unwrap_or_else(|| learner.frame_count as f32 * FRAME_HOP_MS);
        let total_duration = Duration::from_millis(total_duration_ms.round().max(0.0) as u64);
        let confidence = confidence_from_cost(alignment.total_cost);

        Ok(AlignmentReport {
            phonemes,
            total_duration,
            reference_path_cost: alignment.total_cost,
            learner_path_cost,
            global_time_offset_ms,
            confidence,
        })
    }
}

const FRAME_HOP_MS: f32 = 10.0;

fn validate_features(
    reference: &PronunciationFeatures,
    learner: &PronunciationFeatures,
) -> Result<()> {
    if reference.frame_count == 0 {
        return Err(PronunciationError::new(
            "reference features contain no frames for alignment",
        ));
    }
    if learner.frame_count == 0 {
        return Err(PronunciationError::new(
            "learner features contain no frames for alignment",
        ));
    }
    Ok(())
}

fn tokenize_transcript(transcript: &str) -> Result<Vec<&str>> {
    let tokens: Vec<&str> = transcript
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() {
        return Err(PronunciationError::new(
            "transcript must contain at least one token for alignment",
        ));
    }
    Ok(tokens)
}

fn select_pronunciations<'dict>(
    tokens: &[&str],
    pronunciations: &[PronunciationVariants<'dict>],
) -> Result<Vec<&'dict str>> {
    let mut sequence = Vec::new();
    for (token, variants) in tokens.iter().zip(pronunciations.iter()) {
        let variant = variants.first().ok_or_else(|| {
            PronunciationError::new(format!(
                "no pronunciation variants returned for \"{token}\""
            ))
        })?;
        sequence.extend_from_slice(variant);
    }
    if sequence.is_empty() {
        return Err(PronunciationError::new(
            "no phonemes derived from transcript tokens",
        ));
    }
    Ok(sequence)
}

fn assemble_phoneme_reports(
    templates: &[PhonemeTemplate],
    alignment: &DtwAlignment,
) -> Vec<AlignedPhoneme> {
    templates
        .iter()
        .zip(alignment.segments.iter())
        .map(|(template, segment)| {
            let (reference_start_ms, reference_end_ms) =
                frames_to_ms(template.start_frame, template.end_frame);
            let (learner_start_ms, learner_end_ms) =
                frames_to_ms(segment.learner_start_frame, segment.learner_end_frame);

            let reference_mid = (reference_start_ms + reference_end_ms) * 0.5;
            let learner_mid = (learner_start_ms + learner_end_ms) * 0.5;

            AlignedPhoneme {
                symbol: template.symbol.clone(),
                reference_start_ms,
                reference_end_ms,
                learner_start_ms,
                learner_end_ms,
                timing_delta_ms: learner_mid - reference_mid,
                similarity: segment.similarity,
                articulation_variance: segment.cost,
            }
        })
        .collect()
}

fn average_timing_delta(phonemes: &[AlignedPhoneme]) -> f32 {
    if phonemes.is_empty() {
        0.0
    } else {
        phonemes
            .iter()
            .map(|phoneme| phoneme.timing_delta_ms)
            .sum::<f32>()
            / phonemes.len() as f32
    }
}

fn confidence_from_cost(cost: f32) -> f32 {
    1.0 / (1.0 + cost.max(0.0))
}
