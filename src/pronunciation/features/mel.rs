use anyhow::{Context, Result};
use aus::analysis;
use aus::analysis::mel::MelFilterbank;
use aus::spectrum;
use aus::WindowType;
use std::time::Instant;
use tracing::info;

use crate::audio::resample;
use crate::pronunciation::RecordedClip;

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

pub(crate) fn compute_spectrograms(clip: &RecordedClip) -> Result<SpectrogramBundle> {
    info!("ensuring sample rate");
    let start = Instant::now();
    let mono = ensure_sample_rate(clip)?;
    let resample_elapsed = start.elapsed();
    info!(
        elapsed_secs = resample_elapsed.as_secs_f64(),
        samples = mono.len(),
        "sample rate ensured"
    );

    info!("converting to f64");
    let start = Instant::now();
    let audio_f64: Vec<f64> = mono.iter().map(|&s| s as f64).collect();
    let convert_elapsed = start.elapsed();
    info!(
        elapsed_secs = convert_elapsed.as_secs_f64(),
        "conversion to f64 complete"
    );

    let fft_size = ((TARGET_SAMPLE_RATE as usize * WINDOW_MS) / 1000).max(1);
    let hop_size = ((TARGET_SAMPLE_RATE as usize * HOP_MS) / 1000).max(1);
    info!(
        fft_size,
        hop_size,
        audio_samples = audio_f64.len(),
        "computing STFT"
    );
    let start = Instant::now();
    let stft = spectrum::rstft(&audio_f64, fft_size, hop_size, WindowType::Hanning);
    let stft_elapsed = start.elapsed();
    info!(
        elapsed_secs = stft_elapsed.as_secs_f64(),
        stft_frames = stft.len(),
        "STFT computed"
    );

    info!("converting STFT to polar");
    let start = Instant::now();
    let (magnitude, _) = spectrum::complex_to_polar_rstft(&stft);
    let polar_elapsed = start.elapsed();
    info!(
        elapsed_secs = polar_elapsed.as_secs_f64(),
        "STFT converted to polar"
    );

    info!("computing power spectrogram");
    let start = Instant::now();
    let power = analysis::make_power_spectrogram(&magnitude);
    let power_elapsed = start.elapsed();
    info!(
        elapsed_secs = power_elapsed.as_secs_f64(),
        "power spectrogram computed"
    );

    info!("computing mel filterbank");
    let start = Instant::now();
    let freqs = spectrum::rfftfreq(fft_size, TARGET_SAMPLE_RATE);
    let filterbank = MelFilterbank::new(
        MIN_FREQ,
        (TARGET_SAMPLE_RATE as f64) / 2.0,
        MEL_BANDS,
        &freqs,
        true,
    );
    let filterbank_elapsed = start.elapsed();
    info!(
        elapsed_secs = filterbank_elapsed.as_secs_f64(),
        "mel filterbank computed"
    );

    info!("computing mel spectrogram");
    let start = Instant::now();
    let mel = analysis::mel::make_mel_spectrogram(&power, &filterbank);
    let mel_elapsed = start.elapsed();
    info!(
        elapsed_secs = mel_elapsed.as_secs_f64(),
        mel_frames = mel.len(),
        "mel spectrogram computed"
    );

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
