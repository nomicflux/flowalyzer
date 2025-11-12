mod contour;
mod mel;
mod statistics;

use mel::compute_spectrograms;
use ndarray::Axis;
use statistics::assemble_features;
use std::time::Instant;
use tracing::info;

use crate::pronunciation::{PronunciationError, PronunciationFeatures, RecordedClip, Result};

/// Responsible for preparing spectral features from recorded audio.
#[derive(Debug, Default)]
pub struct FeatureExtractor {}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(&self, clip: &RecordedClip) -> Result<PronunciationFeatures> {
        info!("computing spectrograms");
        let start = Instant::now();
        let spectrograms =
            compute_spectrograms(clip).map_err(|err| PronunciationError::new(err.to_string()))?;
        let spectrograms_elapsed = start.elapsed();
        info!(
            elapsed_secs = spectrograms_elapsed.as_secs_f64(),
            mel_frames = spectrograms.mel.len(),
            "spectrograms computed"
        );

        info!("assembling feature matrices");
        let start = Instant::now();
        let matrices = assemble_features(
            &spectrograms.mel,
            &spectrograms.magnitude,
            &spectrograms.power,
        )
        .map_err(|err| PronunciationError::new(err.to_string()))?;
        let matrices_elapsed = start.elapsed();
        info!(
            elapsed_secs = matrices_elapsed.as_secs_f64(),
            "feature matrices assembled"
        );

        let frame_count = matrices.mel.len_of(Axis(0));
        info!(frame_count, "extracting pitch contour");
        let start = Instant::now();
        let pitch_contour = contour::extract_pitch_contour(clip, frame_count)?;
        let pitch_elapsed = start.elapsed();
        info!(
            elapsed_secs = pitch_elapsed.as_secs_f64(),
            pitch_frames = pitch_contour.len(),
            "pitch contour extracted"
        );

        Ok(PronunciationFeatures {
            frame_count,
            mel_bands: matrices.mel.len_of(Axis(1)),
            mel_spectrogram: matrices.mel,
            spectral_flux: matrices.spectral_flux,
            energy: matrices.energy,
            mfcc: matrices.mfcc,
            deltas: matrices.deltas,
            delta_deltas: matrices.delta_deltas,
            pitch_contour,
        })
    }
}
