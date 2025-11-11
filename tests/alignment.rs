use std::fs::File;
use std::path::PathBuf;

use approx::assert_relative_eq;
use flowalyzer::pronunciation::alignment::AudioAligner;
use flowalyzer::pronunciation::metrics::MetricCalculator;
use flowalyzer::pronunciation::{
    AlignmentWeights, PronunciationError, PronunciationFeatures, Result,
};
use ndarray::{Array1, Array2};
use serde::Deserialize;

#[derive(Deserialize)]
struct AlignmentFixture {
    reference: RawFeatures,
    learner: RawFeatures,
}

#[derive(Deserialize)]
struct RawFeatures {
    frame_count: usize,
    mel_bands: usize,
    mel_spectrogram: Vec<Vec<f32>>,
    spectral_flux: Vec<f32>,
    energy: Vec<f32>,
    mfcc: Vec<Vec<f32>>,
    deltas: Vec<Vec<f32>>,
    delta_deltas: Vec<Vec<f32>>,
    pitch_contour: Vec<f32>,
}

#[test]
fn alignment_perfect_match_produces_high_scores() -> Result<()> {
    let (reference, learner) = load_fixture("perfect")?;
    let report = AudioAligner::new(AlignmentWeights::default()).align(&reference, &learner)?;
    let scores = MetricCalculator::new().score(&report)?;

    assert!(report.global_time_offset_ms.abs() < 1.0);
    assert!(scores.overall > 0.9);
    assert!(scores.timing > 0.95);
    assert!(scores.intonation > 0.9);
    assert!(!report.phonemes.is_empty());
    Ok(())
}

#[test]
fn alignment_detects_timing_shift() -> Result<()> {
    let (reference, learner) = load_fixture("timing_shift")?;
    let report = AudioAligner::new(AlignmentWeights::default()).align(&reference, &learner)?;
    let scores = MetricCalculator::new().score(&report)?;

    println!(
        "timing_shift offset={:.2} timing_score={:.2} overall={:.2}",
        report.global_time_offset_ms, scores.timing, scores.overall
    );
    assert!(report.global_time_offset_ms.abs() >= 5.0);
    assert!(scores.timing < 0.98);
    assert!(scores.overall < 0.99);
    Ok(())
}

#[test]
fn alignment_detects_articulation_variance() -> Result<()> {
    let (reference, learner) = load_fixture("articulation_shift")?;
    let report = AudioAligner::new(AlignmentWeights::default()).align(&reference, &learner)?;
    let scores = MetricCalculator::new().score(&report)?;

    println!(
        "articulation_shift offset={:.2} articulation_score={:.2}",
        report.global_time_offset_ms, scores.articulation
    );
    assert_relative_eq!(report.global_time_offset_ms, 0.0, epsilon = 5.0);
    assert!(scores.articulation < 0.7);
    Ok(())
}

fn load_fixture(name: &str) -> Result<(PronunciationFeatures, PronunciationFeatures)> {
    let path = fixture_path(name);
    let file = File::open(&path).map_err(|err| PronunciationError::new(err.to_string()))?;
    let raw: AlignmentFixture =
        serde_json::from_reader(file).map_err(|err| PronunciationError::new(err.to_string()))?;
    let reference = raw.reference.into_features()?;
    let learner = raw.learner.into_features()?;
    Ok((reference, learner))
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/alignment")
        .join(format!("{name}.json"))
}

impl RawFeatures {
    fn into_features(self) -> Result<PronunciationFeatures> {
        ensure_len(self.frame_count, self.spectral_flux.len(), "spectral_flux")?;
        ensure_len(self.frame_count, self.energy.len(), "energy")?;
        ensure_len(self.frame_count, self.pitch_contour.len(), "pitch_contour")?;
        let mel = to_array2(self.mel_spectrogram, self.frame_count, self.mel_bands)?;
        let mfcc_cols = column_count(&self.mfcc);
        let mfcc = to_array2(self.mfcc, self.frame_count, mfcc_cols)?;
        let deltas = to_array2(self.deltas, self.frame_count, mfcc_cols)?;
        let delta_deltas = to_array2(self.delta_deltas, self.frame_count, mfcc_cols)?;
        Ok(PronunciationFeatures {
            frame_count: self.frame_count,
            mel_bands: self.mel_bands,
            mel_spectrogram: mel,
            spectral_flux: Array1::from(self.spectral_flux),
            energy: Array1::from(self.energy),
            mfcc,
            deltas,
            delta_deltas,
            pitch_contour: Array1::from(self.pitch_contour),
        })
    }
}

fn ensure_len(expected: usize, actual: usize, label: &str) -> Result<()> {
    if expected != actual {
        return Err(PronunciationError::new(format!(
            "{label} length mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn column_count(rows: &[Vec<f32>]) -> usize {
    rows.first().map(|row| row.len()).unwrap_or(0)
}

fn to_array2(data: Vec<Vec<f32>>, rows: usize, cols: usize) -> Result<Array2<f32>> {
    ensure_len(rows, data.len(), "matrix rows")?;
    for (index, row) in data.iter().enumerate() {
        ensure_len(cols, row.len(), &format!("matrix cols row {index}"))?;
    }
    let flat: Vec<f32> = data.into_iter().flatten().collect();
    Array2::from_shape_vec((rows, cols), flat)
        .map_err(|err| PronunciationError::new(err.to_string()))
}
