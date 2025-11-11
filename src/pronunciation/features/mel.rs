use anyhow::{Context, Result};
use aus::analysis;
use aus::analysis::mel::MelFilterbank;
use aus::spectrum;
use aus::WindowType;

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
    let mono = ensure_sample_rate(clip)?;
    let audio_f64: Vec<f64> = mono.iter().map(|&s| s as f64).collect();

    let fft_size = ((TARGET_SAMPLE_RATE as usize * WINDOW_MS) / 1000).max(1);
    let hop_size = ((TARGET_SAMPLE_RATE as usize * HOP_MS) / 1000).max(1);

    let stft = spectrum::rstft(&audio_f64, fft_size, hop_size, WindowType::Hanning);
    let (magnitude, _) = spectrum::complex_to_polar_rstft(&stft);
    let power = analysis::make_power_spectrogram(&magnitude);

    let freqs = spectrum::rfftfreq(fft_size, TARGET_SAMPLE_RATE);
    let filterbank = MelFilterbank::new(
        MIN_FREQ,
        (TARGET_SAMPLE_RATE as f64) / 2.0,
        MEL_BANDS,
        &freqs,
        true,
    );
    let mel = analysis::mel::make_mel_spectrogram(&power, &filterbank);

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
