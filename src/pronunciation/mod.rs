pub mod alignment;
pub mod cli;
pub mod features;
pub mod metrics;
pub mod ui;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};

use crate::audio::{decoder, resample};
use crate::types::AudioData;

const TARGET_SAMPLE_RATE: u32 = 16_000;

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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PronunciationFeatures {
    pub frame_count: usize,
    pub mel_bands: usize,
    pub mel_spectrogram: Array2<f32>,
    pub spectral_flux: Array1<f32>,
    pub energy: Array1<f32>,
    pub mfcc: Array2<f32>,
    pub deltas: Array2<f32>,
    pub delta_deltas: Array2<f32>,
    pub pitch_contour: Array1<f32>,
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
            pitch_contour: Array1::zeros(0),
        }
    }
}

/// Alignment report describing coarse timing and similarity comparisons.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlignmentReport {
    pub phonemes: Vec<AlignedPhoneme>,
    pub total_duration: Duration,
    pub reference_path_cost: f32,
    pub learner_path_cost: f32,
    pub global_time_offset_ms: f32,
    pub confidence: f32,
    pub reference_energy: Vec<f32>,
    pub learner_energy: Vec<f32>,
    pub similarity_band: Vec<f32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AlignedPhoneme {
    pub symbol: String,
    pub reference_start_ms: f32,
    pub reference_end_ms: f32,
    pub learner_start_ms: f32,
    pub learner_end_ms: f32,
    pub timing_delta_ms: f32,
    pub similarity: f32,
    pub articulation_variance: f32,
}

/// Aggregate pronunciation scores produced from alignment results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PronunciationScores {
    pub overall: f32,
    pub timing: f32,
    pub articulation: f32,
    pub intonation: f32,
    pub per_phoneme: Vec<PhonemeScore>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhonemeScore {
    pub symbol: String,
    pub timing: f32,
    pub articulation: f32,
    pub intonation: f32,
}

/// Weighting factors applied during alignment cost computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentWeights {
    pub mfcc: f32,
    pub delta: f32,
    pub delta_delta: f32,
    pub mel: f32,
    pub energy: f32,
    pub flux: f32,
}

impl Default for AlignmentWeights {
    fn default() -> Self {
        Self {
            mfcc: 0.35,
            delta: 0.2,
            delta_delta: 0.1,
            mel: 0.1,
            energy: 0.2,
            flux: 0.05,
        }
    }
}

impl AlignmentWeights {
    pub fn load_from_assets(root: &Path) -> Result<Self> {
        let path = root.join("config/alignment_weights.json");
        let file = File::open(&path).map_err(|err| {
            PronunciationError::new(format!(
                "failed to open alignment weights at {:?}: {}",
                path, err
            ))
        })?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).map_err(|err| {
            PronunciationError::new(format!(
                "failed to deserialize alignment weights {:?}: {}",
                path, err
            ))
        })
    }
}

/// Capture configuration shared across workflows.
#[derive(Debug, Clone)]
pub struct CaptureSettings {
    pub device_name: Option<String>,
    pub sample_rate: u32,
    pub latency_ms: RangeInclusive<u32>,
}

impl CaptureSettings {
    pub fn new(
        device_name: Option<String>,
        sample_rate: u32,
        latency_ms: RangeInclusive<u32>,
    ) -> Self {
        Self {
            device_name,
            sample_rate,
            latency_ms,
        }
    }
}

/// Session configuration shared across CLI, analysis, and UI.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub reference_wav: PathBuf,
    pub learner_wav: PathBuf,
    pub assets_root: PathBuf,
    pub capture: CaptureSettings,
    pub alignment: AlignmentWeights,
    pub ui_enabled: bool,
}

impl SessionConfig {
    pub fn new(
        reference_wav: PathBuf,
        learner_wav: PathBuf,
        assets_root: PathBuf,
        capture: CaptureSettings,
        alignment: AlignmentWeights,
    ) -> Self {
        Self {
            reference_wav,
            learner_wav,
            assets_root,
            capture,
            alignment,
            ui_enabled: false,
        }
    }

    pub fn with_ui(mut self, enabled: bool) -> Self {
        self.ui_enabled = enabled;
        self
    }
}

#[derive(Debug, Clone)]
pub struct SessionOutcome {
    pub alignment: AlignmentReport,
    pub scores: PronunciationScores,
}

/// Primary orchestration entry point for the pronunciation pipeline.
pub fn run_session(config: SessionConfig) -> Result<SessionOutcome> {
    validate_config(&config)?;
    let reference_clip = load_clip(&config.reference_wav)?;
    let learner_clip = load_clip(&config.learner_wav)?;
    let extractor = features::FeatureExtractor::new();
    let reference_features = extractor.extract(&reference_clip)?;
    let learner_features = extractor.extract(&learner_clip)?;
    let alignment = alignment::AudioAligner::new(config.alignment.clone())
        .align(&reference_features, &learner_features)?;
    let scores = metrics::MetricCalculator::new().score(&alignment)?;
    let outcome = SessionOutcome { alignment, scores };
    if config.ui_enabled {
        let state = ui::prepare_visualization(&outcome.alignment, &outcome.scores)?;
        crate::ui::launch_ui(&config, &state.alignment, &state.scores)?;
    }
    Ok(outcome)
}

impl RecordedClip {
    fn from_samples(samples: Vec<f32>, sample_rate: u32) -> Self {
        let duration_secs = samples.len() as f64 / sample_rate as f64;
        Self {
            samples: Arc::from(samples.into_boxed_slice()),
            sample_rate,
            channels: 1,
            duration: Duration::from_secs_f64(duration_secs),
        }
    }
}

fn validate_config(config: &SessionConfig) -> Result<()> {
    if config.reference_wav.as_os_str().is_empty() {
        return Err(PronunciationError::new("reference WAV path missing"));
    }
    if config.learner_wav.as_os_str().is_empty() {
        return Err(PronunciationError::new("learner WAV path missing"));
    }
    if !config.assets_root.is_dir() {
        return Err(PronunciationError::new(
            "assets_root must point to an existing directory",
        ));
    }
    if config.capture.sample_rate == 0 {
        return Err(PronunciationError::new("sample_rate must be positive"));
    }
    let min_latency = *config.capture.latency_ms.start();
    let max_latency = *config.capture.latency_ms.end();
    if min_latency == 0 && max_latency == 0 {
        return Err(PronunciationError::new(
            "latency range must specify a positive window",
        ));
    }
    if max_latency < min_latency {
        return Err(PronunciationError::new(
            "latency range must have max >= min",
        ));
    }
    Ok(())
}

fn load_clip(path: &Path) -> Result<RecordedClip> {
    if !path.exists() {
        return Err(PronunciationError::new(format!(
            "audio file {:?} does not exist",
            path
        )));
    }
    let audio =
        decoder::decode_audio(path).map_err(|err| PronunciationError::new(err.to_string()))?;
    clip_from_audio(audio)
}

fn clip_from_audio(audio: AudioData) -> Result<RecordedClip> {
    let samples = resample::linear_resample(&audio.samples, audio.sample_rate, TARGET_SAMPLE_RATE)
        .map_err(|err| PronunciationError::new(err.to_string()))?;
    Ok(RecordedClip::from_samples(samples, TARGET_SAMPLE_RATE))
}
