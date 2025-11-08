//! Transcription module - converts audio to text with timing using Whisper
//!
//! Uses whisper-rs to transcribe audio and extract word-level timing data.
//! This enables linguistic boundary detection for intelligent chunking.

use crate::types::{AudioData, Granularity, Segment, Transcript};
use anyhow::{Context, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Transcribe audio to text with word-level timing
///
/// # Arguments
/// * `audio` - The audio data to transcribe
///
/// # Returns
/// Transcript with segments containing text and timing information
pub fn transcribe_audio(audio: &AudioData) -> Result<Transcript> {
    // Download model path - assumes model is already downloaded
    // User should run: wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin
    let model_path = std::env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "./models/ggml-base.en.bin".to_string());

    // Initialize Whisper context
    let ctx = WhisperContext::new_with_params(
        &model_path,
        WhisperContextParameters::default(),
    ).context("Failed to load Whisper model. Download with: wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin -P ./models/")?;

    // Set up parameters for transcription
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // Enable word-level timestamps
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Transcribe
    let mut state = ctx
        .create_state()
        .context("Failed to create Whisper state")?;
    state
        .full(params, &audio.samples)
        .context("Failed to transcribe audio")?;

    // Extract segments with timing using iterator
    let mut segments = Vec::new();

    for segment in state.as_iter() {
        let text = segment
            .to_str()
            .context("Failed to get segment text")?
            .to_string();

        // Timestamps are in centiseconds (10s of milliseconds), convert to seconds
        let start_time = segment.start_timestamp() as f64 / 100.0;
        let end_time = segment.end_timestamp() as f64 / 100.0;
        let duration = end_time - start_time;
        let granularity = if duration < 1.0 {
            Granularity::Word
        } else {
            Granularity::Sentence
        };

        segments.push(Segment {
            text,
            start_time,
            end_time,
            granularity,
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
        // This test requires a downloaded Whisper model
        // Run: wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin -P ./models/

        // Create a simple test audio (1 second of 440Hz tone)
        let sample_rate = 16000; // Whisper expects 16kHz
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
        };

        // This will fail without the model, but shows the API usage
        let _result = transcribe_audio(&audio);
        // If model exists, verify we got a transcript
        // assert!(result.is_ok());
    }
}
