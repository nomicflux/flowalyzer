use std::error::Error;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tracing::error;

use crate::audio::{assembler, decoder};
use crate::operations::recipe;
use crate::types::{AudioChunk, AudioData, FrameCount, FrameIndex, FrameRange, Recipe};

pub mod alignment;
pub mod features;
pub mod session;
pub use session::{
    AlignedPhoneme, AlignmentReport, ClipVariant, PronunciationScores, RecipeApplicationProgress,
    RecipeApplicationStage, SessionSnapshot,
};

const MAX_CLIP_DURATION_SECS: u64 = 300; // 5 minutes

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
    if !path.exists() {
        let err_msg = format!("audio file {:?} does not exist", path);
        error!(path = %path.display(), "{}", err_msg);
        return Err(PronunciationError::new(err_msg));
    }
    let audio = decoder::decode_audio(path).map_err(|err| {
        let err_msg = err.to_string();
        error!(path = %path.display(), error = %err_msg, "failed to decode audio file");
        PronunciationError::new(err_msg)
    })?;
    let clip = clip_from_audio(audio)?;
    validate_clip_duration(&clip)?;
    Ok(clip)
}

/// Extract the requested range of audio and apply the recipe steps.
pub fn apply_recipe_to_range(
    clip: &RecordedClip,
    start_time: f64,
    end_time: f64,
    recipe: &Recipe,
) -> Result<RecordedClip> {
    let chunk = extract_audio_range(clip, start_time, end_time)?;
    let chunks = recipe::apply_recipe(&chunk, recipe);
    if chunks.is_empty() {
        return Err(PronunciationError::new("recipe produced no output chunks"));
    }
    let audio_data = assembler::assemble_audio(&chunks).ok_or_else(|| {
        PronunciationError::new(
            "failed to assemble audio chunks (sample rate mismatch or empty chunks)",
        )
    })?;
    let result = clip_from_audio(audio_data)?;
    validate_clip_duration(&result)?;
    Ok(result)
}

/// Pull a slice of audio samples between start and end times.
pub fn extract_audio_range(
    clip: &RecordedClip,
    start_time: f64,
    end_time: f64,
) -> Result<AudioChunk> {
    validate_time_range(clip, start_time, end_time)?;
    let start_sample = (start_time * clip.sample_rate as f64) as usize;
    let end_sample = (end_time * clip.sample_rate as f64) as usize;
    let start_sample = start_sample.min(clip.samples.len());
    let end_sample = end_sample.min(clip.samples.len());
    let samples = clip.samples[start_sample..end_sample].to_vec();
    let frame_range = FrameRange::new(
        FrameIndex::from(start_sample),
        FrameCount::from(samples.len()),
    );
    Ok(AudioChunk {
        samples,
        sample_rate: clip.sample_rate,
        start_time,
        end_time,
        frame_range,
    })
}

fn clip_from_audio(audio: AudioData) -> Result<RecordedClip> {
    Ok(RecordedClip::from_samples(audio.samples, audio.sample_rate))
}

fn validate_clip_duration(clip: &RecordedClip) -> Result<()> {
    let duration_secs = clip.duration.as_secs();
    if duration_secs > MAX_CLIP_DURATION_SECS {
        Err(PronunciationError::new(format!(
            "clip duration {} seconds exceeds maximum of {} seconds",
            duration_secs, MAX_CLIP_DURATION_SECS
        )))
    } else {
        Ok(())
    }
}

fn validate_time_range(clip: &RecordedClip, start_time: f64, end_time: f64) -> Result<()> {
    if start_time < 0.0 {
        return Err(PronunciationError::new("start_time must be non-negative"));
    }
    if end_time <= start_time {
        return Err(PronunciationError::new(
            "end_time must be greater than start_time",
        ));
    }
    let clip_duration = clip.duration.as_secs_f64();
    if end_time > clip_duration {
        return Err(PronunciationError::new(format!(
            "end_time exceeds clip duration ({} seconds)",
            clip_duration
        )));
    }
    Ok(())
}
