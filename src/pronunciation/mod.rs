pub mod alignment;
pub mod cli;
pub mod features;
pub mod metrics;
pub mod ui;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ndarray::{Array1, Array2};

/// Convenient alias for results returned by pronunciation modules.
pub type Result<T> = std::result::Result<T, PronunciationError>;

/// Lightweight error type for the pronunciation pipeline scaffolding.
#[derive(Debug, Clone)]
pub struct PronunciationError {
    message: Arc<str>,
}

impl PronunciationError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: Arc::from(message.into()),
        }
    }
}

impl Display for PronunciationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for PronunciationError {}

/// Recorded audio clip placeholder.
#[derive(Debug, Clone, Default)]
pub struct RecordedClip {
    pub samples: Arc<[f32]>,
    pub sample_rate: u32,
    pub channels: u8,
    pub duration: Duration,
}

/// Feature batch placeholder backing future spectral analysis outputs.
#[derive(Debug, Clone)]
pub struct PronunciationFeatures {
    pub frame_count: usize,
    pub mel_bands: usize,
    pub mel_spectrogram: Array2<f32>,
    pub spectral_flux: Array1<f32>,
    pub energy: Array1<f32>,
    pub mfcc: Array2<f32>,
    pub deltas: Array2<f32>,
    pub delta_deltas: Array2<f32>,
}

impl Default for PronunciationFeatures {
    fn default() -> Self {
        Self {
            frame_count: 0,
            mel_bands: 0,
            mel_spectrogram: Array2::zeros((0, 0)),
            spectral_flux: Array1::zeros(0),
            energy: Array1::zeros(0),
            mfcc: Array2::zeros((0, 0)),
            deltas: Array2::zeros((0, 0)),
            delta_deltas: Array2::zeros((0, 0)),
        }
    }
}

/// Alignment report placeholder describing phoneme timing comparisons.
#[derive(Debug, Clone, Default)]
pub struct AlignmentReport {
    pub phonemes: Vec<AlignedPhoneme>,
    pub total_duration: Duration,
    pub confidence: f32,
}

#[derive(Debug, Clone, Default)]
pub struct AlignedPhoneme {
    pub symbol: String,
    pub timing_delta_ms: f32,
    pub similarity: f32,
}

/// Aggregate pronunciation scores produced from alignment results.
#[derive(Debug, Clone, Default)]
pub struct PronunciationScores {
    pub overall: f32,
    pub timing: f32,
    pub articulation: f32,
    pub intonation: f32,
}

/// Session configuration shared across CLI, analysis, and UI.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub reference_wav: Option<PathBuf>,
    pub transcript: Option<String>,
    pub analysis_window: Option<Range<u32>>,
    pub ui_enabled: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            reference_wav: None,
            transcript: None,
            analysis_window: None,
            ui_enabled: true,
        }
    }
}

/// Primary orchestration entry point for the pronunciation pipeline.
pub fn run_session(config: SessionConfig) -> Result<()> {
    let extractor = features::FeatureExtractor::new();
    let aligner = alignment::PhonemeAligner::new();
    let metrics = metrics::MetricCalculator::new();

    let reference_clip = RecordedClip::default();
    let learner_clip = RecordedClip::default();

    let reference_features = extractor.extract(&reference_clip)?;
    let learner_features = extractor.extract(&learner_clip)?;
    let alignment = aligner.align(&reference_features, &learner_features)?;
    let scores = metrics.score(&alignment)?;

    if config.ui_enabled {
        let _state = ui::prepare_visualization(&alignment, &scores)?;
        // A future phase will hand off `_state` to `crate::ui::launch_ui`.
    }
    Ok(())
}
