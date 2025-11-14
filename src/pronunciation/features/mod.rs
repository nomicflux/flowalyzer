mod contour;
mod mel;
mod statistics;

use mel::compute_spectrograms_with_progress;
use ndarray::Axis;
use statistics::assemble_features_with_progress;
use std::time::Instant;
use tracing::info;

use crate::pronunciation::{PronunciationError, PronunciationFeatures, RecordedClip, Result};

/// Responsible for preparing spectral features from recorded audio.
#[derive(Debug, Default)]
pub struct FeatureExtractor {}

#[derive(Clone, Copy, Debug)]
pub enum FeatureExtractionPhase {
    Spectrograms,
    FeatureMatrices,
    PitchContour,
}

impl FeatureExtractionPhase {
    pub fn label(&self) -> &'static str {
        match self {
            FeatureExtractionPhase::Spectrograms => "Computing spectrograms",
            FeatureExtractionPhase::FeatureMatrices => "Assembling feature matrices",
            FeatureExtractionPhase::PitchContour => "Extracting pitch contour",
        }
    }

    pub fn order(&self) -> usize {
        match self {
            FeatureExtractionPhase::Spectrograms => 1,
            FeatureExtractionPhase::FeatureMatrices => 2,
            FeatureExtractionPhase::PitchContour => 3,
        }
    }

    pub fn total() -> usize {
        3
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FeatureExtractionEvent {
    PhaseStart(FeatureExtractionPhase),
    PhaseProgress {
        phase: FeatureExtractionPhase,
        label: &'static str,
        current: u32,
        total: u32,
    },
    PhaseElapsed {
        phase: FeatureExtractionPhase,
        elapsed_secs: u32,
    },
}

impl FeatureExtractor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extract(&self, clip: &RecordedClip) -> Result<PronunciationFeatures> {
        self.extract_with_progress(clip, &mut |_| {})
    }

    pub fn extract_with_progress<F>(
        &self,
        clip: &RecordedClip,
        reporter: &mut F,
    ) -> Result<PronunciationFeatures>
    where
        F: FnMut(FeatureExtractionEvent),
    {
        info!("computing spectrograms");
        reporter(FeatureExtractionEvent::PhaseStart(
            FeatureExtractionPhase::Spectrograms,
        ));
        let start = Instant::now();
        let spectrograms = compute_spectrograms_with_progress(clip, reporter)
            .map_err(|err| PronunciationError::new(err.to_string()))?;
        let spectrograms_elapsed = start.elapsed();
        let elapsed_secs = spectrograms_elapsed.as_secs() as u32;
        if elapsed_secs > 0 {
            reporter(FeatureExtractionEvent::PhaseElapsed {
                phase: FeatureExtractionPhase::Spectrograms,
                elapsed_secs,
            });
        }
        info!(
            elapsed_secs = spectrograms_elapsed.as_secs_f64(),
            mel_frames = spectrograms.mel.len(),
            "spectrograms computed"
        );

        info!("assembling feature matrices");
        reporter(FeatureExtractionEvent::PhaseStart(
            FeatureExtractionPhase::FeatureMatrices,
        ));
        let start = Instant::now();
        let frame_count = spectrograms.mel.len();
        let matrices = assemble_features_with_progress(
            &spectrograms.mel,
            &spectrograms.magnitude,
            &spectrograms.power,
            frame_count,
            reporter,
        )
        .map_err(|err| PronunciationError::new(err.to_string()))?;
        let matrices_elapsed = start.elapsed();
        let elapsed_secs = matrices_elapsed.as_secs() as u32;
        if elapsed_secs > 0 {
            reporter(FeatureExtractionEvent::PhaseElapsed {
                phase: FeatureExtractionPhase::FeatureMatrices,
                elapsed_secs,
            });
        }
        info!(
            elapsed_secs = matrices_elapsed.as_secs_f64(),
            "feature matrices assembled"
        );

        let frame_count = matrices.mel.len_of(Axis(0));
        info!(frame_count, "extracting pitch contour");
        reporter(FeatureExtractionEvent::PhaseStart(
            FeatureExtractionPhase::PitchContour,
        ));
        let pitch_start = Instant::now();
        let pitch_contour = contour::extract_pitch_contour_with_reporting(
            clip,
            frame_count,
            reporter,
            pitch_start,
        )?;
        let pitch_elapsed = pitch_start.elapsed();
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
