use flowalyzer::pronunciation::alignment::PhonemeAligner;
use flowalyzer::pronunciation::PronunciationFeatures;
use ndarray::{Array1, Array2};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Deserialize)]
struct FeatureFixture {
    mel: Vec<Vec<f32>>,
    spectral_flux: Vec<f32>,
    energy: Vec<f32>,
    mfcc: Vec<Vec<f32>>,
    deltas: Vec<Vec<f32>>,
    delta_deltas: Vec<Vec<f32>>,
}

impl FeatureFixture {
    fn into_features(self) -> PronunciationFeatures {
        let frame_count = self.mfcc.len();
        assert_eq!(frame_count, self.mel.len());
        assert_eq!(frame_count, self.spectral_flux.len());
        assert_eq!(frame_count, self.energy.len());
        assert_eq!(frame_count, self.deltas.len());
        assert_eq!(frame_count, self.delta_deltas.len());

        PronunciationFeatures {
            frame_count,
            mel_bands: self.mel.first().map_or(0, |row| row.len()),
            mel_spectrogram: array2(self.mel),
            spectral_flux: Array1::from_vec(self.spectral_flux),
            energy: Array1::from_vec(self.energy),
            mfcc: array2(self.mfcc),
            deltas: array2(self.deltas),
            delta_deltas: array2(self.delta_deltas),
        }
    }
}

#[test]
fn aligner_produces_alignment_report_with_expected_metrics() {
    let reference = load_features("reference_features");
    let learner = load_features("learner_features");
    let aligner = PhonemeAligner::new();

    let report = aligner
        .align("time", &reference, &learner)
        .expect("alignment should succeed");

    assert_eq!(report.phonemes.len(), 3);
    assert_eq!(report.phonemes[0].symbol, "T");
    assert!(report.reference_path_cost >= 0.0);
    assert!(report.learner_path_cost >= 0.0);
    assert!(report.confidence.is_finite());
    assert_eq!(report.total_duration, Duration::from_millis(70));

    let first = &report.phonemes[0];
    assert!((first.reference_start_ms - 0.0).abs() < 1e-3);
    assert!((first.reference_end_ms - 20.0).abs() < 1e-3);
    assert!((first.learner_start_ms - 0.0).abs() < 1e-3);
    assert!((first.learner_end_ms - 20.0).abs() < 1e-3);
    assert!((first.timing_delta_ms).abs() < 1e-3);

    let third = &report.phonemes[2];
    assert!((third.learner_end_ms - 70.0).abs() < 1e-3);
    assert!(third.similarity > 0.0);
}

fn load_features(name: &str) -> PronunciationFeatures {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/alignment");
    path.push(format!("{name}.json"));
    let raw = fs::read_to_string(path).expect("missing fixture");
    let fixture: FeatureFixture = serde_json::from_str(&raw).expect("invalid fixture");
    fixture.into_features()
}

fn array2(rows: Vec<Vec<f32>>) -> Array2<f32> {
    let row_count = rows.len();
    let col_count = rows.first().map_or(0, |row| row.len());
    let data: Vec<f32> = rows.into_iter().flatten().collect();
    Array2::from_shape_vec((row_count, col_count), data)
        .expect("fixture dimensions produced invalid matrix")
}
