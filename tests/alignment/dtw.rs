use std::fs;
use std::path::PathBuf;

use flowalyzer::pronunciation::alignment::{
    dtw::align_templates,
    dtw::frames_to_ms,
    templates::build_templates,
};
use flowalyzer::pronunciation::PronunciationFeatures;
use ndarray::{Array1, Array2};
use serde::Deserialize;

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
        let mel_bands = self.mel.first().map(|row| row.len()).unwrap_or(0);
        assert_eq!(frame_count, self.mel.len());
        assert_eq!(frame_count, self.spectral_flux.len());
        assert_eq!(frame_count, self.energy.len());
        assert_eq!(frame_count, self.deltas.len());
        assert_eq!(frame_count, self.delta_deltas.len());
        PronunciationFeatures {
            frame_count,
            mel_bands,
            mel_spectrogram: to_array2(self.mel),
            spectral_flux: Array1::from_vec(self.spectral_flux),
            energy: Array1::from_vec(self.energy),
            mfcc: to_array2(self.mfcc),
            deltas: to_array2(self.deltas),
            delta_deltas: to_array2(self.delta_deltas),
        }
    }
}

#[test]
fn dtw_alignment_segments_match_expected_ranges() {
    let reference = load_features("reference_features");
    let learner = load_features("learner_features");
    let phonemes = ["AA", "BB", "CC"];

    let templates = build_templates(&reference, &phonemes).unwrap();
    let alignment = align_templates(&templates, &learner).unwrap();

    assert_eq!(alignment.segments.len(), phonemes.len());

    let first = &alignment.segments[0];
    assert_eq!(first.symbol, "AA");
    assert_eq!((first.start_frame, first.end_frame), (0, 2));

    let second = &alignment.segments[1];
    assert_eq!(second.symbol, "BB");
    assert_eq!((second.start_frame, second.end_frame), (2, 5));

    let third = &alignment.segments[2];
    assert_eq!(third.symbol, "CC");
    assert_eq!((third.start_frame, third.end_frame), (5, 7));

    let total: f32 = alignment.segments.iter().map(|seg| seg.cost).sum();
    assert!((alignment.total_cost - total).abs() < 1e-5);
    assert!(alignment
        .segments
        .iter()
        .all(|seg| seg.similarity.is_finite() && seg.similarity <= 1.0 && seg.similarity > 0.0));

    let (start_ms, end_ms) =
        frames_to_ms(alignment.segments[1].start_frame, alignment.segments[1].end_frame);
    assert!((start_ms - 20.0).abs() < 1e-3);
    assert!((end_ms - 50.0).abs() < 1e-3);
}

fn load_features(name: &str) -> PronunciationFeatures {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/alignment");
    path.push(format!("{name}.json"));
    let data = fs::read_to_string(path).expect("fixture missing");
    let fixture: FeatureFixture = serde_json::from_str(&data).expect("invalid fixture");
    fixture.into_features()
}

fn to_array2(rows: Vec<Vec<f32>>) -> Array2<f32> {
    let row_count = rows.len();
    let col_count = rows.first().map(|row| row.len()).unwrap_or(0);
    let data: Vec<f32> = rows.into_iter().flatten().collect();
    Array2::from_shape_vec((row_count, col_count), data)
        .expect("fixture dimensions produced invalid matrix")
}

