//! Core types for flowalyzer audio processing pipeline

use anyhow::{ensure, Result};
use serde::Deserialize;

/// Raw audio data representation (mono, f32 samples)
#[derive(Debug, Clone)]
pub struct AudioData {
    /// Audio samples, normalized to [-1.0, 1.0]
    pub samples: Vec<f32>,
    /// Sample rate in Hz (e.g., 44100)
    pub sample_rate: u32,
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
    /// Indices of transcript segments that contributed to this chunk
    pub source_segment_ids: Vec<usize>,
}

/// Configuration for chunking strategy
#[derive(Debug, Clone, Copy)]
pub struct ChunkConfig {
    pub target_duration: f64, // target chunk duration in seconds
    pub min_duration: f64,    // minimum acceptable duration
    pub max_duration: f64,    // maximum acceptable duration
}

impl ChunkConfig {
    pub fn new(target_duration: f64) -> Self {
        Self {
            target_duration,
            min_duration: target_duration * 0.5, // 50% of target
            max_duration: target_duration * 1.5, // 150% of target
        }
    }
}

/// An audio chunk with timing metadata
#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub start_time: f64, // original position in source audio
    pub end_time: f64,
    pub metadata: Option<ChunkMetadata>,
}

/// Optional metadata for audio chunks
#[derive(Debug, Clone, Default)]
pub struct ChunkMetadata {
    /// Mean fundamental frequency (Hz)
    pub f0_mean: Option<f32>,
    /// Standard deviation of fundamental frequency
    pub f0_std: Option<f32>,
}

/// Operations that can be applied to audio chunks
#[derive(Debug, Clone)]
pub enum Operation {
    /// Repeat the chunk N times
    Repeat(u32),
    /// Speed multiplier (0.5 = half speed, 2.0 = double speed)
    Speed(f32),
    /// Insert silence after chunk (seconds)
    InsertSilence(f64),
    /// No operation
    Identity,
}

/// A single step in a recipe: repeat N times at specific speed, optionally add silence after
#[derive(Debug, Clone)]
pub struct RecipeStep {
    /// How many times to repeat the chunk
    pub repeat_count: u32,
    /// Speed multiplier for this step (0.5 = slow, 1.0 = normal, 1.5 = fast)
    pub speed_factor: f32,
    /// Add silence after this step (duration matches speed-adjusted chunk length)
    pub add_silence_after: bool,
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

    /// Create the standard language learning recipe (3x slow, 3x normal, 3x fast with silences)
    pub fn language_learning() -> Self {
        Self {
            name: "language-learning".to_string(),
            steps: vec![
                RecipeStep {
                    repeat_count: 3,
                    speed_factor: 0.5,
                    add_silence_after: true,
                },
                RecipeStep {
                    repeat_count: 3,
                    speed_factor: 1.0,
                    add_silence_after: true,
                },
                RecipeStep {
                    repeat_count: 3,
                    speed_factor: 1.5,
                    add_silence_after: true,
                },
            ],
        }
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
    #[serde(default, alias = "silence", alias = "addSilence")]
    pub add_silence_after: bool,
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
            add_silence_after: self.add_silence_after,
        }
    }
}

/// Selector for which chunks to apply an operation to
#[derive(Debug, Clone)]
pub enum ChunkSelector {
    /// Apply to all chunks
    All,
    /// Apply to specific chunk by index
    Index(usize),
    /// Apply to range of chunks [start, end)
    Range(usize, usize),
}

/// A rule defining an operation and which chunks it applies to
#[derive(Debug, Clone)]
pub struct ProcessingRule {
    pub selector: ChunkSelector,
    pub operation: Operation,
}

/// A complete processing plan with multiple rules
#[derive(Debug, Clone)]
pub struct ProcessingPlan {
    pub rules: Vec<ProcessingRule>,
}

impl ProcessingPlan {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(mut self, selector: ChunkSelector, operation: Operation) -> Self {
        self.rules.push(ProcessingRule {
            selector,
            operation,
        });
        self
    }
}

impl Default for ProcessingPlan {
    fn default() -> Self {
        Self::new()
    }
}
