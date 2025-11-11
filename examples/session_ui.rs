use anyhow::Result;
use flowalyzer::pronunciation::{
    alignment::PhonemeAligner, metrics::MetricCalculator, PronunciationFeatures, SessionConfig,
};
use flowalyzer::ui::launch_ui;
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

fn main() -> Result<()> {
    let reference = load_fixture(include_str!(
        "../tests/fixtures/alignment/reference_features.json"
    ))?;
    let learner = load_fixture(include_str!(
        "../tests/fixtures/alignment/learner_features.json"
    ))?;

    let aligner = PhonemeAligner::new();
    let alignment = aligner.align("time", &reference, &learner)?;

    let scores = MetricCalculator::new().score(&alignment)?;
    let config = SessionConfig {
        transcript: Some("TIME".to_string()),
        ..SessionConfig::default()
    };

    launch_ui(&config, &alignment, &scores)?;
    Ok(())
}

fn load_fixture(data: &str) -> Result<PronunciationFeatures> {
    let fixture: FeatureFixture = serde_json::from_str(data)?;
    Ok(fixture.into_features())
}

impl FeatureFixture {
    fn into_features(self) -> PronunciationFeatures {
        let frame_count = self.mfcc.len();
        assert_eq!(frame_count, self.mel.len());
        PronunciationFeatures {
            frame_count,
            mel_bands: self.mel.first().map_or(0, |row| row.len()),
            mel_spectrogram: to_array2(self.mel),
            spectral_flux: Array1::from_vec(self.spectral_flux),
            energy: Array1::from_vec(self.energy),
            mfcc: to_array2(self.mfcc),
            deltas: to_array2(self.deltas),
            delta_deltas: to_array2(self.delta_deltas),
        }
    }
}

fn to_array2(rows: Vec<Vec<f32>>) -> Array2<f32> {
    let rows_len = rows.len();
    let cols_len = rows.first().map_or(0, |row| row.len());
    Array2::from_shape_vec((rows_len, cols_len), rows.into_iter().flatten().collect())
        .expect("invalid fixture dimensions")
}
