use ndarray::{Array1, Axis};

use crate::pronunciation::{PronunciationError, PronunciationFeatures, Result};

/// Aggregated spectral templates representing reference phonemes.
#[derive(Debug, Clone)]
pub struct PhonemeTemplate {
    pub symbol: String,
    pub centroid: Array1<f32>,
    pub average_energy: f32,
    pub start_frame: usize,
    pub end_frame: usize,
}

/// Builds deterministic phoneme templates by evenly partitioning reference frames.
pub fn build_templates(
    features: &PronunciationFeatures,
    phonemes: &[&str],
) -> Result<Vec<PhonemeTemplate>> {
    ensure_inputs(features.frame_count, phonemes.len())?;
    let segments = partition_frames(features.frame_count, phonemes.len());
    segments
        .into_iter()
        .zip(phonemes.iter())
        .map(|((start, end), symbol)| template_for(symbol, features, start, end))
        .collect()
}

fn ensure_inputs(frame_count: usize, phoneme_count: usize) -> Result<()> {
    if phoneme_count == 0 {
        return Err(PronunciationError::new(
            "phoneme sequence must contain at least one entry",
        ));
    }
    if frame_count < phoneme_count {
        return Err(PronunciationError::new(format!(
            "reference has {frame_count} frames but {phoneme_count} phonemes"
        )));
    }
    Ok(())
}

fn partition_frames(frame_count: usize, phoneme_count: usize) -> Vec<(usize, usize)> {
    let base = frame_count / phoneme_count;
    let remainder = frame_count % phoneme_count;
    let mut segments = Vec::with_capacity(phoneme_count);
    let mut start = 0;
    for index in 0..phoneme_count {
        let span = base + usize::from(index < remainder);
        let end = start + span.max(1);
        segments.push((start, end));
        start = end;
    }
    segments
}

fn template_for(
    symbol: &str,
    features: &PronunciationFeatures,
    start: usize,
    end: usize,
) -> Result<PhonemeTemplate> {
    let mfcc_slice = features.mfcc.slice(ndarray::s![start..end, ..]);
    let centroid = mfcc_slice
        .mean_axis(Axis(0))
        .ok_or_else(|| PronunciationError::new("unable to compute MFCC centroid"))?;

    let energy_slice = features.energy.slice(ndarray::s![start..end]);
    let average_energy =
        energy_slice.iter().copied().sum::<f32>() / energy_slice.len().max(1) as f32;

    Ok(PhonemeTemplate {
        symbol: symbol.to_string(),
        centroid,
        average_energy,
        start_frame: start,
        end_frame: end,
    })
}
