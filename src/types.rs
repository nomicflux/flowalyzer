//! Core types for flowalyzer audio processing pipeline

use anyhow::{ensure, Result};
use serde::Deserialize;
use std::time::Duration;

/// Sample rate in Hz (e.g., 16_000).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SampleRate(u32);

impl SampleRate {
    pub fn new(hz: u32) -> Result<Self> {
        ensure!(hz > 0, "Sample rate must be positive");
        Ok(Self(hz))
    }

    pub fn hz(self) -> u32 {
        self.0
    }

    pub fn frames_from_duration(&self, duration: Duration) -> FrameCount {
        let nanos_per_second = 1_000_000_000u128;
        let nanos = duration.as_secs() as u128 * nanos_per_second + duration.subsec_nanos() as u128;
        let frames = nanos * self.0 as u128 / nanos_per_second;
        FrameCount::from(frames as u64)
    }

    pub fn duration_from_frames(&self, frames: FrameCount) -> Duration {
        let nanos_per_second = 1_000_000_000u128;
        let total_nanos = frames.as_u64() as u128 * nanos_per_second / self.0 as u128;
        let secs = (total_nanos / nanos_per_second) as u64;
        let nanos = (total_nanos % nanos_per_second) as u32;
        Duration::new(secs, nanos)
    }

    pub fn frames_from_seconds(&self, seconds: f64) -> FrameCount {
        let frames = (seconds * self.0 as f64).floor().max(0.0) as u64;
        FrameCount::from(frames)
    }

    pub fn seconds_from_frames(&self, frames: FrameCount) -> f64 {
        frames.as_u64() as f64 / self.0 as f64
    }

    pub fn frames_from_seconds_round(&self, seconds: f64) -> FrameCount {
        let frames = (seconds * self.0 as f64).round().max(0.0) as u64;
        FrameCount::from(frames)
    }
}

impl From<SampleRate> for u32 {
    fn from(rate: SampleRate) -> Self {
        rate.hz()
    }
}

/// Index of a single frame in the source timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameIndex(u64);

impl FrameIndex {
    pub const ZERO: Self = FrameIndex(0);

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn to_count(self) -> FrameCount {
        FrameCount::from(self.0)
    }

    pub fn saturating_sub(self, other: FrameIndex) -> FrameCount {
        FrameCount::from(self.0.saturating_sub(other.0))
    }
}

impl From<u64> for FrameIndex {
    fn from(value: u64) -> Self {
        FrameIndex(value)
    }
}

impl From<usize> for FrameIndex {
    fn from(value: usize) -> Self {
        FrameIndex(value as u64)
    }
}

/// Number of frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameCount(u64);

impl FrameCount {
    pub const ZERO: Self = FrameCount(0);

    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn saturating_sub(self, other: FrameCount) -> FrameCount {
        FrameCount(self.0.saturating_sub(other.0))
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<u64> for FrameCount {
    fn from(value: u64) -> Self {
        FrameCount(value)
    }
}

impl From<usize> for FrameCount {
    fn from(value: usize) -> Self {
        FrameCount(value as u64)
    }
}

impl std::ops::Add for FrameCount {
    type Output = FrameCount;

    fn add(self, rhs: FrameCount) -> Self::Output {
        FrameCount(self.0.saturating_add(rhs.0))
    }
}

/// Frame span with start and length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameRange {
    pub start: FrameIndex,
    pub length: FrameCount,
}

impl FrameRange {
    pub fn new(start: FrameIndex, length: FrameCount) -> Self {
        Self { start, length }
    }

    pub fn end(self) -> FrameIndex {
        FrameIndex::from(self.start.as_usize() + self.length.as_usize())
    }

    pub fn clamp_to(self, total_length: FrameCount) -> Self {
        let clamped_start = FrameIndex::from(self.start.as_usize().min(total_length.as_usize()));
        let clamped_end = self.end().as_usize().min(total_length.as_usize());
        let length = FrameCount::from(clamped_end.saturating_sub(clamped_start.as_usize()));
        Self {
            start: clamped_start,
            length,
        }
    }

    pub fn from_times(start_time: f64, end_time: f64, sample_rate: SampleRate) -> Self {
        let start_frames = sample_rate.frames_from_seconds_round(start_time);
        let end_frames = sample_rate.frames_from_seconds_round(end_time);
        let length = end_frames.saturating_sub(start_frames);
        Self {
            start: FrameIndex::from(start_frames.as_u64()),
            length,
        }
    }
}

/// Raw audio data representation (mono, f32 samples)
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Audio samples, normalized to [-1.0, 1.0]
    pub samples: Vec<f32>,
    /// Sample rate in Hz (e.g., 44100)
    pub sample_rate: u32,
    /// Frame range covered by this buffer
    pub frame_range: FrameRange,
}

/// Transcription output containing timestamped segments
#[derive(Debug, Clone)]
pub struct Transcript {
    pub segments: Vec<Segment>,
}

/// A segment of transcribed audio with timing information
#[derive(Debug, Clone)]
pub struct Segment {
    pub text: String,
    pub start_time: f64, // seconds
    pub end_time: f64,   // seconds
    pub granularity: Granularity,
}

/// Granularity of a transcript segment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Word,
    Sentence,
}

/// Time boundary for an audio chunk
#[derive(Debug, Clone)]
pub struct ChunkBoundary {
    pub start_time: f64, // seconds
    pub end_time: f64,   // seconds
    pub frame_range: FrameRange,
    /// Indices of transcript segments that contributed to this chunk
    pub source_segment_ids: Vec<usize>,
}

/// Configuration for chunking strategy
#[derive(Debug, Clone, Copy)]
pub struct ChunkConfig {
    pub target_frames: FrameCount,
    pub max_frames: FrameCount,
    pub overshoot_frames: FrameCount,
}

impl ChunkConfig {
    pub fn new_frames(
        target_frames: FrameCount,
        max_frames: FrameCount,
        overshoot_frames: FrameCount,
    ) -> Self {
        Self {
            target_frames,
            max_frames,
            overshoot_frames,
        }
    }

    pub fn from_seconds(target_duration: f64, sample_rate: SampleRate) -> Self {
        let target_frames = sample_rate.frames_from_seconds_round(target_duration);
        assert!(
            !target_frames.is_zero(),
            "target duration must produce at least one frame"
        );
        // Default to exact target bounds unless caller provides explicit limits.
        let max_frames = target_frames;
        let overshoot_frames = FrameCount::ZERO;
        Self::new_frames(target_frames, max_frames, overshoot_frames)
    }
}

/// An audio chunk with timing information
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub start_time: f64, // original position in source audio
    pub end_time: f64,
    pub frame_range: FrameRange,
}

/// A single step in a recipe: repeat N times at specific speed, optionally add silence after
#[derive(Debug, Clone)]
pub struct RecipeStep {
    /// How many times to repeat the chunk
    pub repeat_count: u32,
    /// Speed multiplier for this step (0.5 = slow, 1.0 = normal, 1.5 = fast)
    pub speed_factor: f32,
    /// When true, emit silence chunks instead of audio
    pub silent: bool,
}

/// A recipe is a sequence of steps to apply to each chunk
#[derive(Debug, Clone)]
pub struct Recipe {
    /// Name of this recipe
    pub name: String,
    /// Steps to apply in order
    pub steps: Vec<RecipeStep>,
}

impl Recipe {
    /// Create a new empty recipe
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
        }
    }

    /// Add a step to this recipe
    pub fn add_step(mut self, step: RecipeStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// Runtime-configurable recipe parsed from JSON input
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeRecipe {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub steps: Vec<RuntimeRecipeStep>,
}

impl RuntimeRecipe {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.steps.is_empty(),
            "Recipe must contain at least one step"
        );
        for (idx, step) in self.steps.iter().enumerate() {
            step.validate(idx)?;
        }
        Ok(())
    }

    pub fn to_recipe(&self) -> Recipe {
        let mut recipe = Recipe::new(self.name.clone().unwrap_or_else(|| "runtime".to_string()));
        for step in &self.steps {
            recipe = recipe.add_step(step.to_recipe_step());
        }
        recipe
    }
}

/// Runtime-configurable recipe step parsed from JSON
#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeRecipeStep {
    #[serde(alias = "repeat", alias = "repeatCount")]
    pub repeat_count: u32,
    #[serde(alias = "speed", alias = "factor")]
    pub speed_factor: f32,
    #[serde(default, alias = "silent")]
    pub silent: bool,
}

impl RuntimeRecipeStep {
    fn validate(&self, index: usize) -> Result<()> {
        ensure!(
            self.repeat_count > 0,
            "Recipe step {} repeat_count must be greater than zero",
            index
        );
        ensure!(
            self.speed_factor > 0.0,
            "Recipe step {} speed_factor must be positive",
            index
        );
        Ok(())
    }

    fn to_recipe_step(&self) -> RecipeStep {
        RecipeStep {
            repeat_count: self.repeat_count,
            speed_factor: self.speed_factor,
            silent: self.silent,
        }
    }
}
