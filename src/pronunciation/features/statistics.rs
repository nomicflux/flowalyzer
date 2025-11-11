use anyhow::Result;
use aus::analysis;
use ndarray::{Array1, Array2, Axis};

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

pub(crate) fn assemble_features(
    mel_spectrogram: &[Vec<f64>],
    magnitude_spectrogram: &[Vec<f64>],
    power_spectrogram: &[Vec<f64>],
) -> Result<FeatureMatrices> {
    let mel = array_from_vec2(mel_spectrogram);
    let spectral_flux = compute_spectral_flux(magnitude_spectrogram);
    let energy = compute_energy(power_spectrogram);

    let mfcc_raw = analysis::mel::mfcc_spectrogram(mel_spectrogram, MFCC_COUNT, None);
    let mfcc = array_from_vec2(&mfcc_raw);
    let deltas = compute_delta_matrix(&mfcc, DELTA_WINDOW);
    let delta_deltas = compute_delta_matrix(&deltas, DELTA_WINDOW);

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

fn compute_spectral_flux(magnitude: &[Vec<f64>]) -> Array1<f32> {
    if magnitude.is_empty() {
        return Array1::zeros(0);
    }
    let mut flux = Vec::with_capacity(magnitude.len());
    flux.push(0.0_f32);
    for i in 1..magnitude.len() {
        let previous = &magnitude[i - 1];
        let current = &magnitude[i];
        let mut sum = 0.0;
        for (curr, prev) in current.iter().zip(previous.iter()) {
            let diff = (curr - prev).max(0.0);
            sum += diff * diff;
        }
        flux.push(sum.sqrt() as f32);
    }
    Array1::from_vec(flux)
}

fn compute_energy(power: &[Vec<f64>]) -> Array1<f32> {
    let mut energies = Vec::with_capacity(power.len());
    for frame in power {
        let sum: f64 = frame.iter().sum();
        energies.push(sum.sqrt() as f32);
    }
    Array1::from_vec(energies)
}

fn compute_delta_matrix(input: &Array2<f32>, window: usize) -> Array2<f32> {
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
    }

    output
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
