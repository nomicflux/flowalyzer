mod audio;
mod chunking;
mod operations;
mod transcription;
mod types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::{fs, path::Path};
use transcription::TranscriptionSettings;

/// Flowalyzer - Audio chunking and manipulation tool
///
/// Processes audio files by breaking them into chunks at linguistic boundaries
/// and applying operations (repeat, speed change, silence insertion).
#[derive(Parser, Debug)]
#[command(name = "flowalyzer")]
#[command(version = "0.1.0")]
#[command(about = "Audio chunking and manipulation tool", long_about = None)]
struct Args {
    /// Input audio file path (supports MP3, OGG, FLAC, WAV, etc.)
    #[arg(value_name = "INPUT")]
    input_file: PathBuf,

    /// Output directory where processed chunk files will be written
    #[arg(value_name = "OUTPUT_DIR")]
    output_dir: PathBuf,

    /// Target chunk duration in seconds (for linguistic boundary detection)
    #[arg(long, default_value_t = 2.0)]
    target_duration: f64,

    /// JSON recipe specification (inline JSON string)
    #[arg(long, value_name = "JSON", conflicts_with = "recipe_file")]
    recipe_json: Option<String>,

    /// Path to JSON recipe specification
    #[arg(long, value_name = "PATH", conflicts_with = "recipe_json")]
    recipe_file: Option<PathBuf>,

    /// Optional trim start time (seconds or HH:MM:SS.mmm)
    #[arg(long, value_name = "TIME")]
    start: Option<String>,

    /// Optional trim end time (seconds or HH:MM:SS.mmm)
    #[arg(long, value_name = "TIME")]
    end: Option<String>,

    /// Path to Whisper GGML model (defaults to ./models/ggml-base.bin or WHISPER_MODEL_PATH)
    #[arg(long, value_name = "PATH")]
    whisper_model: Option<PathBuf>,

    /// Override language detection with an explicit language code (e.g., 'es')
    #[arg(long, value_name = "LANG")]
    whisper_language: Option<String>,
}

impl Args {
    /// Validate CLI arguments
    fn validate(&self) -> Result<()> {
        // Check input file exists
        if !self.input_file.exists() {
            anyhow::bail!("Input file does not exist: {:?}", self.input_file);
        }

        // Check input file is readable
        if !self.input_file.is_file() {
            anyhow::bail!("Input path is not a file: {:?}", self.input_file);
        }

        // Check target duration is positive
        if self.target_duration <= 0.0 {
            anyhow::bail!(
                "Target duration must be positive, got: {}",
                self.target_duration
            );
        }

        if self.recipe_json.is_none() && self.recipe_file.is_none() {
            anyhow::bail!("Provide a recipe via --recipe-json or --recipe-file");
        }

        if self.output_dir.exists() && !self.output_dir.is_dir() {
            anyhow::bail!("Output path must be a directory: {:?}", self.output_dir);
        }

        Ok(())
    }

    fn runtime_recipe(&self) -> Result<types::RuntimeRecipe> {
        load_recipe_from_sources(self.recipe_file.as_deref(), self.recipe_json.as_deref())
    }

    fn trim_range(&self) -> Result<(Option<f64>, Option<f64>)> {
        let start = parse_optional_time(self.start.as_deref(), "start")?;
        let end = parse_optional_time(self.end.as_deref(), "end")?;

        if let (Some(s), Some(e)) = (start, end) {
            ensure!(e > s, "End time must be greater than start time");
        }

        Ok((start, end))
    }

    fn transcription_settings(&self) -> Result<TranscriptionSettings> {
        let mut settings = TranscriptionSettings::default();

        if let Some(model) = &self.whisper_model {
            settings.model_path = model.to_string_lossy().into_owned();
        }

        if let Some(language) = &self.whisper_language {
            let trimmed = language.trim();
            ensure!(
                !trimmed.is_empty(),
                "Whisper language override must not be empty"
            );
            settings.language = Some(trimmed.to_string());
            settings.detect_language = false;
        }

        settings.apply_model_defaults();

        Ok(settings)
    }
}

fn main() -> Result<()> {
    run(Args::parse())
}

fn run(args: Args) -> Result<()> {
    args.validate()
        .context("Failed to validate command-line arguments")?;
    let transcription_settings = args.transcription_settings()?;
    print_banner(&args, &transcription_settings);
    let recipe = load_recipe(&args)?;
    log_recipe(&recipe);
    let trim = args.trim_range()?;
    log_trim_request(trim);
    let audio = decode_and_trim(&args, trim)?;
    let transcript = transcribe_with_logging(&audio, &transcription_settings)?;
    let boundaries = plan_chunks(&audio, &transcript, args.target_duration);
    let chunks = slice_chunks(&audio, &boundaries);
    write_chunks(&chunks, &boundaries, &recipe, &args.output_dir)?;
    println!("\n✓ Processing complete!");
    Ok(())
}

fn print_banner(args: &Args, settings: &TranscriptionSettings) {
    println!("Flowalyzer v0.1.0 - Language Learning Audio Processor");
    println!("Input:  {:?}", args.input_file);
    println!("Output dir: {:?}", args.output_dir);
    println!("Target chunk duration: {} seconds", args.target_duration);
    println!("Whisper model: {}", settings.model_path);
    if settings.is_english_only_model()
        && settings.language.as_deref() == Some("en")
        && !settings.detect_language
    {
        println!("Whisper language: English (model is English-only; detection disabled)");
    } else {
        match (&settings.language, settings.detect_language) {
            (Some(language), _) => println!("Whisper language override: {}", language),
            (None, true) => println!("Whisper language detection: enabled"),
            (None, false) => println!("Whisper language detection: disabled"),
        }
    }
}

fn load_recipe(args: &Args) -> Result<types::Recipe> {
    let runtime = args
        .runtime_recipe()
        .context("Failed to load recipe specification")?;
    runtime.validate().context("Recipe validation failed")?;
    Ok(runtime.to_recipe())
}

fn log_recipe(recipe: &types::Recipe) {
    println!("Recipe: {} ({} steps)", recipe.name, recipe.steps.len());
}

fn log_trim_request(trim: (Option<f64>, Option<f64>)) {
    if let Some(start) = trim.0 {
        println!("Trim start: {:.3} seconds", start);
    }
    if let Some(end) = trim.1 {
        println!("Trim end: {:.3} seconds", end);
    }
}

fn decode_and_trim(args: &Args, trim: (Option<f64>, Option<f64>)) -> Result<types::AudioData> {
    println!("\n1. Decoding input audio...");
    let decoded =
        audio::decoder::decode_audio(&args.input_file).context("Failed to decode input audio")?;
    println!(
        "   Loaded {} samples at {} Hz",
        decoded.samples.len(),
        decoded.sample_rate
    );
    let total_duration = decoded.samples.len() as f64 / decoded.sample_rate as f64;
    let start = trim.0.unwrap_or(0.0);
    let end = trim.1.unwrap_or(total_duration);
    ensure!(
        start >= 0.0,
        "Trim start must be non-negative (got {:.3})",
        start
    );
    ensure!(
        start < total_duration,
        "Trim start ({:.3}) must be less than audio duration ({:.3})",
        start,
        total_duration
    );
    ensure!(
        end > start,
        "Trim end ({:.3}) must be greater than start ({:.3})",
        end,
        start
    );
    let effective_end = end.min(total_duration);
    if start > 0.0 || effective_end < total_duration {
        println!(
            "   Trimming audio to range {:.3}s - {:.3}s (duration {:.3}s)",
            start,
            effective_end,
            effective_end - start
        );
        return Ok(trim_audio_segment(&decoded, start, effective_end));
    }
    Ok(decoded)
}

fn transcribe_with_logging(
    audio: &types::AudioData,
    settings: &TranscriptionSettings,
) -> Result<types::Transcript> {
    println!("\n2. Transcribing audio with Whisper...");
    let transcript = transcription::transcribe_audio(audio, settings)
        .context("Failed to transcribe audio")?;
    println!("   Found {} segments", transcript.segments.len());
    log_transcript_preview(&transcript);
    Ok(transcript)
}

fn log_transcript_preview(transcript: &types::Transcript) {
    let sentence_segments = transcript
        .segments
        .iter()
        .filter(|segment| matches!(segment.granularity, types::Granularity::Sentence))
        .count();
    let word_segments = transcript
        .segments
        .iter()
        .filter(|segment| matches!(segment.granularity, types::Granularity::Word))
        .count();
    let preview: Vec<String> = transcript
        .segments
        .iter()
        .take(3)
        .map(|segment| {
            format_preview_text(&segment.text)
        })
        .collect();
    if preview.is_empty() {
        println!(
            "   Segment mix: {} sentence / {} word",
            sentence_segments, word_segments
        );
    } else {
        println!(
            "   Segment mix: {} sentence / {} word; preview: {}",
            sentence_segments,
            word_segments,
            preview.join(" | ")
        );
    }
}

fn format_preview_text(text: &str) -> String {
    const PREVIEW_CHAR_LIMIT: usize = 40;
    let sanitized = text.trim().replace('\n', " ");
    if sanitized.is_empty() {
        return sanitized;
    }
    let mut chars = sanitized.chars();
    let mut preview: String = chars
        .by_ref()
        .take(PREVIEW_CHAR_LIMIT)
        .collect();
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

fn plan_chunks(
    audio: &types::AudioData,
    transcript: &types::Transcript,
    target_duration: f64,
) -> Vec<types::ChunkBoundary> {
    println!("\n3. Calculating linguistic chunk boundaries...");
    let config = types::ChunkConfig::new(target_duration);
    let pauses = detect_pauses_for_chunking(audio, target_duration);
    let pause_count = pauses.len();
    println!(
        "   Pause detector: {} candidate pause{}",
        pause_count,
        if pause_count == 1 { "" } else { "s" }
    );
    let boundaries = chunking::calculate_chunk_boundaries(transcript, config, &pauses);
    println!("   Created {} chunks at natural breaks", boundaries.len());
    if !boundaries.is_empty() {
        let total_segments: usize = boundaries
            .iter()
            .map(|boundary| boundary.source_segment_ids.len())
            .sum();
        println!(
            "   Average transcript segments per chunk: {:.2}",
            total_segments as f64 / boundaries.len() as f64
        );
    }
    boundaries
}

fn detect_pauses_for_chunking(audio: &types::AudioData, target_duration: f64) -> Vec<f64> {
    let min_silence_duration = (target_duration * 0.2).clamp(0.15, 0.6);
    let window_duration = 0.05;
    let silence_threshold = 0.04;
    audio::pause_detector::detect_pauses(
        audio,
        min_silence_duration,
        silence_threshold,
        window_duration,
    )
}
fn slice_chunks(
    audio: &types::AudioData,
    boundaries: &[types::ChunkBoundary],
) -> Vec<types::AudioChunk> {
    println!("\n4. Slicing audio into chunks...");
    let chunks = audio::slicer::slice_audio(audio, boundaries);
    println!("   Sliced into {} audio chunks", chunks.len());
    chunks
}

fn write_chunks(
    chunks: &[types::AudioChunk],
    boundaries: &[types::ChunkBoundary],
    recipe: &types::Recipe,
    output_dir: &Path,
) -> Result<()> {
    println!("\n5. Applying recipe to each chunk and writing outputs...");
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output directory {:?}", output_dir))?;
    let mut written = 0usize;
    for (index, chunk) in chunks.iter().enumerate() {
        if write_single_chunk(index, chunk, &boundaries[index], recipe, output_dir)? {
            written += 1;
        }
        log_chunk_progress(index, chunks.len());
    }
    log_chunk_summary(written, output_dir);
    Ok(())
}

fn write_single_chunk(
    index: usize,
    chunk: &types::AudioChunk,
    boundary: &types::ChunkBoundary,
    recipe: &types::Recipe,
    output_dir: &Path,
) -> Result<bool> {
    let processed = operations::recipe::apply_recipe(chunk, recipe);
    if processed.is_empty() {
        eprintln!(
            "   Chunk {} produced no processed segments; skipping",
            index + 1
        );
        return Ok(false);
    }
    let chunk_dir = output_dir.join(format!("chunk_{:04}", index + 1));
    fs::create_dir_all(&chunk_dir)
        .with_context(|| format!("Failed to create chunk output directory {:?}", chunk_dir))?;
    let processed_audio = audio::assembler::assemble_audio(&processed)
        .ok_or_else(|| anyhow!("Failed to assemble processed audio for chunk {}", index + 1))?;
    let output_path = chunk_dir.join("processed.wav");
    audio::encoder::encode_audio(&processed_audio, &output_path).with_context(|| {
        format!(
            "Failed to encode processed audio for chunk {} at {:?}",
            index + 1,
            output_path
        )
    })?;
    println!(
        "   Wrote chunk {:04} to {:?} ({:.3}s → {:.3}s, {} transcript segments)",
        index + 1,
        output_path,
        chunk.start_time,
        chunk.end_time,
        boundary.source_segment_ids.len()
    );
    Ok(true)
}

fn log_chunk_progress(index: usize, total: usize) {
    if (index + 1).is_multiple_of(10) || index + 1 == total {
        println!("   Processed {}/{} chunks", index + 1, total);
    }
}

fn log_chunk_summary(written: usize, output_dir: &Path) {
    println!(
        "   Completed writing {} chunk files under {:?}",
        written, output_dir
    );
}

fn load_recipe_from_sources(
    path: Option<&Path>,
    json: Option<&str>,
) -> Result<types::RuntimeRecipe> {
    if let Some(p) = path {
        let data =
            fs::read_to_string(p).with_context(|| format!("Failed to read recipe file {:?}", p))?;
        return parse_runtime_recipe(&data);
    }

    if let Some(raw) = json {
        return parse_runtime_recipe(raw);
    }

    bail!("No recipe source provided"); // Should not happen due to validation
}

fn parse_runtime_recipe(raw: &str) -> Result<types::RuntimeRecipe> {
    let recipe: types::RuntimeRecipe =
        serde_json::from_str(raw).context("Failed to parse recipe JSON")?;
    Ok(recipe)
}

fn parse_optional_time(value: Option<&str>, label: &str) -> Result<Option<f64>> {
    match value {
        Some(raw) => {
            let seconds = parse_time_to_seconds(raw)
                .with_context(|| format!("Invalid {} time '{}'", label, raw))?;
            Ok(Some(seconds))
        }
        None => Ok(None),
    }
}

fn parse_time_to_seconds(raw: &str) -> Result<f64> {
    if raw.contains(':') {
        return parse_hms_time(raw);
    }

    let seconds: f64 = raw
        .parse()
        .with_context(|| format!("Failed to parse seconds value '{}'", raw))?;
    ensure!(seconds >= 0.0, "Time values must be non-negative");
    Ok(seconds)
}

fn parse_hms_time(raw: &str) -> Result<f64> {
    let parts: Vec<&str> = raw.split(':').collect();
    ensure!(
        (2..=3).contains(&parts.len()),
        "Time format must be MM:SS or HH:MM:SS"
    );

    let seconds = parts
        .last()
        .unwrap()
        .parse::<f64>()
        .with_context(|| format!("Invalid seconds component '{}'", parts.last().unwrap()))?;
    let minutes = parts[parts.len() - 2]
        .parse::<f64>()
        .with_context(|| format!("Invalid minutes component '{}'", parts[parts.len() - 2]))?;
    ensure!(minutes >= 0.0, "Minutes must be non-negative");
    ensure!(seconds >= 0.0, "Seconds must be non-negative");

    let hours = if parts.len() == 3 {
        let value = parts[0]
            .parse::<f64>()
            .with_context(|| format!("Invalid hours component '{}'", parts[0]))?;
        ensure!(value >= 0.0, "Hours must be non-negative");
        value
    } else {
        0.0
    };

    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}

fn trim_audio_segment(
    audio: &types::AudioData,
    start_seconds: f64,
    end_seconds: f64,
) -> types::AudioData {
    let sample_rate = audio.sample_rate;
    let sr = sample_rate as f64;
    let total_samples = audio.samples.len();

    let start_index = ((start_seconds * sr).floor().max(0.0)) as usize;
    let start_index = start_index.min(total_samples);
    let end_index = ((end_seconds * sr).ceil().max(start_index as f64)) as usize;
    let end_index = end_index.min(total_samples);

    let samples = audio.samples[start_index..end_index].to_vec();

    types::AudioData {
        samples,
        sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_seconds() {
        let result = parse_optional_time(Some("12.5"), "start").unwrap();
        assert_eq!(result, Some(12.5));
    }

    #[test]
    fn parse_time_hms() {
        let result = parse_optional_time(Some("01:02:03.5"), "end").unwrap();
        let expected = 3600.0 + 120.0 + 3.5;
        assert!((result.unwrap() - expected).abs() < 1e-6);
    }

    #[test]
    fn parse_recipe_inline_json() {
        let json = r#"{
            "name": "test",
            "steps": [
                {"repeat_count": 2, "speed_factor": 1.0, "add_silence_after": true}
            ]
        }"#;
        let recipe = parse_runtime_recipe(json).unwrap();
        assert_eq!(recipe.name.as_deref(), Some("test"));
        assert_eq!(recipe.steps.len(), 1);
    }

    #[test]
    fn test_verify_cli_args_compile() {
        // This test just ensures Args can be constructed
        let args = Args {
            input_file: PathBuf::from("test.wav"),
            output_dir: PathBuf::from("output"),
            target_duration: 2.0,
            recipe_json: Some("{}".to_string()),
            recipe_file: None,
            start: None,
            end: None,
            whisper_model: None,
            whisper_language: None,
        };

        assert_eq!(args.target_duration, 2.0);
    }

    #[test]
    fn transcription_settings_defaults_enable_detection() {
        let args = Args {
            input_file: PathBuf::from("test.wav"),
            output_dir: PathBuf::from("output"),
            target_duration: 2.0,
            recipe_json: Some("{}".to_string()),
            recipe_file: None,
            start: None,
            end: None,
            whisper_model: None,
            whisper_language: None,
        };

        let settings = args.transcription_settings().unwrap();
        assert!(settings.detect_language);
        assert!(settings.language.is_none());
    }

    #[test]
    fn transcription_settings_with_language_disables_detection() {
        let args = Args {
            input_file: PathBuf::from("test.wav"),
            output_dir: PathBuf::from("output"),
            target_duration: 2.0,
            recipe_json: Some("{}".to_string()),
            recipe_file: None,
            start: None,
            end: None,
            whisper_model: Some(PathBuf::from("/tmp/whisper.bin")),
            whisper_language: Some("es".to_string()),
        };

        let settings = args.transcription_settings().unwrap();
        assert_eq!(settings.model_path, "/tmp/whisper.bin");
        assert_eq!(settings.language.as_deref(), Some("es"));
        assert!(!settings.detect_language);
    }

    #[test]
    fn transcription_settings_force_english_for_english_only_models() {
        let args = Args {
            input_file: PathBuf::from("test.wav"),
            output_dir: PathBuf::from("output"),
            target_duration: 2.0,
            recipe_json: Some("{}".to_string()),
            recipe_file: None,
            start: None,
            end: None,
            whisper_model: Some(PathBuf::from("/tmp/ggml-base.en.bin")),
            whisper_language: None,
        };

        let settings = args.transcription_settings().unwrap();
        assert_eq!(settings.model_path, "/tmp/ggml-base.en.bin");
        assert_eq!(settings.language.as_deref(), Some("en"));
        assert!(!settings.detect_language);
    }

    #[test]
    fn preview_text_leaves_short_strings() {
        let preview = format_preview_text("Hello world");
        assert_eq!(preview, "Hello world");
    }

    #[test]
    fn preview_text_handles_newlines() {
        let preview = format_preview_text("Hello\nWorld");
        assert_eq!(preview, "Hello World");
    }

    #[test]
    fn preview_text_truncates_utf8_safely() {
        let input = "مرحبا".repeat(12); // 12 * 5 = 60 chars
        let preview = format_preview_text(&input);
        assert!(preview.ends_with('…'));
        assert!(preview.chars().count() <= 41);
    }
}
