use anyhow::{Context, Result};
use aus::analysis;
use aus::analysis::mel::MelFilterbank;
use aus::spectrum;
use aus::WindowType;
use std::time::Instant;
use tracing::info;

use crate::audio::resample;
use crate::pronunciation::RecordedClip;

use super::FeatureExtractionEvent;
use super::FeatureExtractionPhase;

pub(crate) const TARGET_SAMPLE_RATE: u32 = 16_000;
pub(crate) const WINDOW_MS: usize = 25;
pub(crate) const HOP_MS: usize = 10;
pub(crate) const MEL_BANDS: usize = 80;
const MIN_FREQ: f64 = 20.0;

pub(crate) struct SpectrogramBundle {
    pub mel: Vec<Vec<f64>>,
    pub magnitude: Vec<Vec<f64>>,
    pub power: Vec<Vec<f64>>,
}

pub(crate) fn compute_spectrograms_with_progress<F>(
    clip: &RecordedClip,
    reporter: &mut F,
) -> Result<SpectrogramBundle>
where
    F: FnMut(FeatureExtractionEvent),
{
    info!("ensuring sample rate");
    let mono = ensure_sample_rate(clip)?;
    info!(samples = mono.len(), "sample rate ensured");

    info!("converting to f64");
    let audio_f64: Vec<f64> = mono.iter().map(|&s| s as f64).collect();
    info!("conversion to f64 complete");

    let fft_size = ((TARGET_SAMPLE_RATE as usize * WINDOW_MS) / 1000).max(1);
    let hop_size = ((TARGET_SAMPLE_RATE as usize * HOP_MS) / 1000).max(1);
    let expected_stft_frames = if audio_f64.len() >= fft_size {
        (audio_f64.len() - fft_size) / hop_size + 1
    } else {
        0
    };

    info!(
        fft_size,
        hop_size,
        audio_samples = audio_f64.len(),
        expected_frames = expected_stft_frames,
        "computing STFT"
    );

    let stft_start = Instant::now();
    let stft = spectrum::rstft(&audio_f64, fft_size, hop_size, WindowType::Hanning);
    let stft_frames = stft.len();
    reporter(FeatureExtractionEvent::PhaseProgress {
        phase: FeatureExtractionPhase::Spectrograms,
        label: "STFT frames",
        current: stft_frames as u32,
        total: stft_frames as u32,
    });
    info!(
        elapsed_secs = stft_start.elapsed().as_secs_f64(),
        stft_frames, "STFT computed"
    );

    info!("converting STFT to polar");
    let (magnitude, _) = spectrum::complex_to_polar_rstft(&stft);
    reporter(FeatureExtractionEvent::PhaseProgress {
        phase: FeatureExtractionPhase::Spectrograms,
        label: "STFT frames",
        current: stft_frames as u32,
        total: stft_frames as u32,
    });
    info!("STFT converted to polar");

    info!("computing power spectrogram");
    let power = analysis::make_power_spectrogram(&magnitude);
    reporter(FeatureExtractionEvent::PhaseProgress {
        phase: FeatureExtractionPhase::Spectrograms,
        label: "STFT frames",
        current: stft_frames as u32,
        total: stft_frames as u32,
    });
    info!("power spectrogram computed");

    info!("computing mel filterbank");
    let freqs = spectrum::rfftfreq(fft_size, TARGET_SAMPLE_RATE);
    let filterbank = MelFilterbank::new(
        MIN_FREQ,
        (TARGET_SAMPLE_RATE as f64) / 2.0,
        MEL_BANDS,
        &freqs,
        true,
    );
    info!("mel filterbank computed");

    info!("computing mel spectrogram");
    let mel = analysis::mel::make_mel_spectrogram(&power, &filterbank);
    let mel_frames = mel.len();
    reporter(FeatureExtractionEvent::PhaseProgress {
        phase: FeatureExtractionPhase::Spectrograms,
        label: "STFT frames",
        current: mel_frames as u32,
        total: mel_frames as u32,
    });
    info!(mel_frames, "mel spectrogram computed");

    Ok(SpectrogramBundle {
        mel,
        magnitude,
        power,
    })
}

fn ensure_sample_rate(clip: &RecordedClip) -> Result<Vec<f32>> {
    if clip.sample_rate == TARGET_SAMPLE_RATE {
        Ok(clip.samples.to_vec())
    } else {
        resample::linear_resample(clip.samples.as_ref(), clip.sample_rate, TARGET_SAMPLE_RATE)
            .with_context(|| {
                format!(
                    "failed to resample audio from {} Hz to {} Hz",
                    clip.sample_rate, TARGET_SAMPLE_RATE
                )
            })
    }
}
