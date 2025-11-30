//! Transcription module - converts audio to text with timing using Whisper
//! Used only by the Flowalyzer binary.

use crate::types::{AudioData, Granularity, Segment, Transcript};
use anyhow::{Context, Result};
use std::path::Path;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

#[cfg(test)]
use crate::types::{FrameCount, FrameIndex, FrameRange};

/// Configuration for a transcription run
#[derive(Debug, Clone)]
pub struct TranscriptionSettings {
    pub model_path: String,
    pub language: Option<String>,
    pub detect_language: bool,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        let model_path = std::env::var("WHISPER_MODEL_PATH")
            .unwrap_or_else(|_| "./models/ggml-base.bin".to_string());
        Self {
            model_path,
            language: None,
            detect_language: true,
        }
    }
}

impl TranscriptionSettings {
    pub fn is_english_only_model(&self) -> bool {
        Self::path_is_english_only(&self.model_path)
    }

    fn path_is_english_only(path: &str) -> bool {
        Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".en.bin"))
            .unwrap_or(false)
    }
}

/// Transcribe audio to text with word-level timing
pub fn transcribe_audio(audio: &AudioData, settings: &TranscriptionSettings) -> Result<Transcript> {
    let ctx = WhisperContext::new_with_params(
        &settings.model_path,
        WhisperContextParameters::default(),
    )
    .context("Failed to load Whisper model. Download with: wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin -P ./models/")?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_translate(false);
    match settings.language.as_deref() {
        Some(language) => params.set_language(Some(language)),
        None if settings.detect_language => params.set_language(None),
        None => params.set_language(None),
    }

    let mut state = ctx
        .create_state()
        .context("Failed to create Whisper state")?;
    state
        .full(params, &audio.samples)
        .context("Failed to transcribe audio")?;

    let mut segments = Vec::new();
    for segment in state.as_iter() {
        let text = segment
            .to_str()
            .context("Failed to get segment text")?
            .to_string();

        let start_time = segment.start_timestamp() as f64 / 100.0;
        let end_time = segment.end_timestamp() as f64 / 100.0;
        segments.push(Segment {
            text,
            start_time,
            end_time,
            granularity: Granularity::Sentence,
        });
    }

    Ok(Transcript { segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires model file to be downloaded
    fn test_transcribe_audio() {
        let sample_rate = 16000;
        let duration = 1.0;
        let num_samples = (sample_rate as f64 * duration) as usize;

        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((t * 2.0 * std::f32::consts::PI * 440.0).sin() * 0.1);
        }

        let audio = AudioData {
            samples,
            sample_rate,
            frame_range: FrameRange::new(FrameIndex::ZERO, FrameCount::from(num_samples)),
        };

        let _result = transcribe_audio(&audio, &TranscriptionSettings::default());
    }
}
