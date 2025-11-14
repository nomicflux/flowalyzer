use anyhow::Result;
use aus::analysis;
use ndarray::{Array1, Array2, Axis};

use super::FeatureExtractionEvent;
use super::FeatureExtractionPhase;

const MFCC_COUNT: usize = 13;
const DELTA_WINDOW: usize = 2;
const EPSILON: f32 = 1e-12;

pub(crate) struct FeatureMatrices {
    pub mel: Array2<f32>,
    pub spectral_flux: Array1<f32>,
    pub energy: Array1<f32>,
    pub mfcc: Array2<f32>,
    pub deltas: Array2<f32>,
    pub delta_deltas: Array2<f32>,
}

pub(crate) fn assemble_features_with_progress<F>(
    mel_spectrogram: &[Vec<f64>],
    magnitude_spectrogram: &[Vec<f64>],
    power_spectrogram: &[Vec<f64>],
    frame_count: usize,
    reporter: &mut F,
) -> Result<FeatureMatrices>
where
    F: FnMut(FeatureExtractionEvent),
{
    let mel = array_from_vec2(mel_spectrogram);
    reporter(FeatureExtractionEvent::PhaseProgress {
        phase: FeatureExtractionPhase::FeatureMatrices,
        label: "Feature rows",
        current: frame_count as u32,
        total: frame_count as u32,
    });

    let spectral_flux =
        compute_spectral_flux_with_progress(magnitude_spectrogram, frame_count, reporter);
    let energy = compute_energy_with_progress(power_spectrogram, frame_count, reporter);

    let mfcc_raw = analysis::mel::mfcc_spectrogram(mel_spectrogram, MFCC_COUNT, None);
    let mfcc = array_from_vec2(&mfcc_raw);
    reporter(FeatureExtractionEvent::PhaseProgress {
        phase: FeatureExtractionPhase::FeatureMatrices,
        label: "Feature rows",
        current: frame_count as u32,
        total: frame_count as u32,
    });

    let deltas = compute_delta_matrix_with_progress(&mfcc, DELTA_WINDOW, frame_count, reporter);
    let delta_deltas =
        compute_delta_matrix_with_progress(&deltas, DELTA_WINDOW, frame_count, reporter);

    let mel = normalize_2d(&mel);
    let spectral_flux = normalize_1d(&spectral_flux);
    let energy = normalize_1d(&energy);
    let mfcc = normalize_2d(&mfcc);
    let deltas = normalize_2d(&deltas);
    let delta_deltas = normalize_2d(&delta_deltas);

    Ok(FeatureMatrices {
        mel,
        spectral_flux,
        energy,
        mfcc,
        deltas,
        delta_deltas,
    })
}

fn compute_spectral_flux_with_progress<F>(
    magnitude: &[Vec<f64>],
    total: usize,
    reporter: &mut F,
) -> Array1<f32>
where
    F: FnMut(FeatureExtractionEvent),
{
    if magnitude.is_empty() {
        return Array1::zeros(0);
    }
    let mut flux = Vec::with_capacity(magnitude.len());
    flux.push(0.0_f32);
    const REPORT_INTERVAL: usize = 256;
    for i in 1..magnitude.len() {
        let previous = &magnitude[i - 1];
        let current = &magnitude[i];
        let mut sum = 0.0;
        for (curr, prev) in current.iter().zip(previous.iter()) {
            let diff = (curr - prev).max(0.0);
            sum += diff * diff;
        }
        flux.push(sum.sqrt() as f32);
        if i > 0 && i % REPORT_INTERVAL == 0 {
            reporter(FeatureExtractionEvent::PhaseProgress {
                phase: FeatureExtractionPhase::FeatureMatrices,
                label: "Feature rows",
                current: i as u32,
                total: total as u32,
            });
        }
    }
    Array1::from_vec(flux)
}

fn compute_energy_with_progress<F>(
    power: &[Vec<f64>],
    total: usize,
    reporter: &mut F,
) -> Array1<f32>
where
    F: FnMut(FeatureExtractionEvent),
{
    let mut energies = Vec::with_capacity(power.len());
    const REPORT_INTERVAL: usize = 256;
    for (idx, frame) in power.iter().enumerate() {
        let sum: f64 = frame.iter().sum();
        energies.push(sum.sqrt() as f32);
        if idx > 0 && idx % REPORT_INTERVAL == 0 {
            reporter(FeatureExtractionEvent::PhaseProgress {
                phase: FeatureExtractionPhase::FeatureMatrices,
                label: "Feature rows",
                current: idx as u32,
                total: total as u32,
            });
        }
    }
    Array1::from_vec(energies)
}

fn compute_delta_matrix_with_progress<F>(
    input: &Array2<f32>,
    window: usize,
    total: usize,
    reporter: &mut F,
) -> Array2<f32>
where
    F: FnMut(FeatureExtractionEvent),
{
    if input.is_empty() {
        return Array2::zeros((0, 0));
    }
    let frames = input.len_of(Axis(0));
    let coeffs = input.len_of(Axis(1));
    let mut output = Array2::zeros((frames, coeffs));
    let denominator = 2.0_f32
        * (1..=window)
            .map(|n| (n * n) as f32)
            .sum::<f32>()
            .max(EPSILON);

    const REPORT_INTERVAL: usize = 256;
    for t in 0..frames {
        let mut numerator = Array1::zeros(coeffs);
        for n in 1..=window {
            let prev_idx = t.saturating_sub(n);
            let next_idx = (t + n).min(frames - 1);
            let prev = input.row(prev_idx);
            let next = input.row(next_idx);
            let diff = (&next - &prev).to_owned() * (n as f32);
            numerator += &diff;
        }
        output
            .row_mut(t)
            .assign(&(&numerator / denominator.max(EPSILON)));
        if t > 0 && t % REPORT_INTERVAL == 0 {
            reporter(FeatureExtractionEvent::PhaseProgress {
                phase: FeatureExtractionPhase::FeatureMatrices,
                label: "Feature rows",
                current: t as u32,
                total: total as u32,
            });
        }
    }

    output
}

fn array_from_vec2(data: &[Vec<f64>]) -> Array2<f32> {
    if data.is_empty() {
        return Array2::zeros((0, 0));
    }
    let rows = data.len();
    let cols = data[0].len();
    let mut flat = Vec::with_capacity(rows * cols);
    for row in data {
        flat.extend(row.iter().map(|v| *v as f32));
    }
    Array2::from_shape_vec((rows, cols), flat).expect("valid mel dimensions")
}

fn normalize_2d(input: &Array2<f32>) -> Array2<f32> {
    if input.is_empty() {
        return input.clone();
    }
    let mean = input.mean().unwrap_or(0.0);
    let variance = input.mapv(|v| (v - mean).powi(2)).sum() / (input.len() as f32).max(1.0);
    let std_dev = variance.sqrt().max(EPSILON);
    input.mapv(|v| (v - mean) / std_dev)
}

fn normalize_1d(input: &Array1<f32>) -> Array1<f32> {
    if input.is_empty() {
        return input.clone();
    }
    let mean = input.mean().unwrap_or(0.0);
    let variance =
        input.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / (input.len() as f32).max(1.0);
    let std_dev = variance.sqrt().max(EPSILON);
    input.mapv(|v| (v - mean) / std_dev)
}
