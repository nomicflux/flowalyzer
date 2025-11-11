mod mel;
mod statistics;

use mel::compute_spectrograms;
use ndarray::Axis;
use statistics::assemble_features;

use crate::pronunciation::{PronunciationError, PronunciationFeatures, RecordedClip, Result};

/// Responsible for preparing spectral features from recorded audio.
#[derive(Debug, Default)]
pub struct FeatureExtractor {}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(&self, clip: &RecordedClip) -> Result<PronunciationFeatures> {
        let spectrograms =
            compute_spectrograms(clip).map_err(|err| PronunciationError::new(err.to_string()))?;
        let matrices = assemble_features(
            &spectrograms.mel,
            &spectrograms.magnitude,
            &spectrograms.power,
        )
        .map_err(|err| PronunciationError::new(err.to_string()))?;

        Ok(PronunciationFeatures {
            frame_count: matrices.mel.len_of(Axis(0)),
            mel_bands: matrices.mel.len_of(Axis(1)),
            mel_spectrogram: matrices.mel,
            spectral_flux: matrices.spectral_flux,
            energy: matrices.energy,
            mfcc: matrices.mfcc,
            deltas: matrices.deltas,
            delta_deltas: matrices.delta_deltas,
        })
    }
}
