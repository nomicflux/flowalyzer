use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::audio::decoder;

pub mod alignment;
pub mod features;
pub mod session;
pub use session::{AlignmentReport, SessionSnapshot};

/// Convenient alias for results returned by pronunciation helpers.
pub type Result<T> = std::result::Result<T, PronunciationError>;

/// Lightweight error type for the pronunciation recipe helpers.
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

impl RecordedClip {
    pub fn from_samples(samples: Vec<f32>, sample_rate: u32) -> Self {
        let duration_secs = samples.len() as f64 / sample_rate as f64;
        Self {
            samples: Arc::from(samples.into_boxed_slice()),
            sample_rate,
            channels: 1,
            duration: Duration::from_secs_f64(duration_secs),
        }
    }
}

/// Load and normalize a clip from disk for recipe application.
pub fn load_clip(path: &Path) -> Result<RecordedClip> {
    let audio =
        decoder::decode_audio(path).map_err(|err| PronunciationError::new(err.to_string()))?;
    Ok(RecordedClip::from_samples(audio.samples, audio.sample_rate))
}
