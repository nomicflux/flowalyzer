mod audio;
mod chunking;
mod operations;
mod transcription;
mod types;

use anyhow::{bail, ensure, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::{fs, path::Path};

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate arguments
    args.validate()
        .context("Failed to validate command-line arguments")?;

    println!("Flowalyzer v0.1.0 - Language Learning Audio Processor");
    println!("Input:  {:?}", args.input_file);
    println!("Output dir: {:?}", args.output_dir);
    println!("Target chunk duration: {} seconds", args.target_duration);

    let runtime_recipe = args
        .runtime_recipe()
        .context("Failed to load recipe specification")?;
    runtime_recipe
        .validate()
        .context("Recipe validation failed")?;
    let recipe = runtime_recipe.to_recipe();
    println!("Recipe: {} ({} steps)", recipe.name, recipe.steps.len());

    let (trim_start, trim_end) = args.trim_range()?;
    if let Some(start) = trim_start {
        println!("Trim start: {:.3} seconds", start);
    }
    if let Some(end) = trim_end {
        println!("Trim end: {:.3} seconds", end);
    }

    // Pipeline implementation
    println!("\n1. Decoding input audio...");
    let decoded_audio =
        audio::decoder::decode_audio(&args.input_file).context("Failed to decode input audio")?;
    println!(
        "   Loaded {} samples at {} Hz",
        decoded_audio.samples.len(),
        decoded_audio.sample_rate
    );

    let total_duration = decoded_audio.samples.len() as f64 / decoded_audio.sample_rate as f64;
    let requested_start = trim_start.unwrap_or(0.0);
    let requested_end = trim_end.unwrap_or(total_duration);
    ensure!(
        requested_start >= 0.0,
        "Trim start must be non-negative (got {:.3})",
        requested_start
    );
    ensure!(
        requested_start < total_duration,
        "Trim start ({:.3}) must be less than audio duration ({:.3})",
        requested_start,
        total_duration
    );
    ensure!(
        requested_end > requested_start,
        "Trim end ({:.3}) must be greater than start ({:.3})",
        requested_end,
        requested_start
    );
    let effective_end = requested_end.min(total_duration);
    let whole_file = requested_start <= f64::EPSILON
        && (effective_end - total_duration).abs() <= (1.0 / decoded_audio.sample_rate as f64);
    let audio = if whole_file {
        decoded_audio
    } else {
        println!(
            "   Trimming audio to range {:.3}s - {:.3}s (duration {:.3}s)",
            requested_start,
            effective_end,
            effective_end - requested_start
        );
        trim_audio_segment(&decoded_audio, requested_start, effective_end)
    };

    // 2. Transcribe audio to get linguistic boundaries
    println!("\n2. Transcribing audio with Whisper...");
    let transcript =
        transcription::transcribe_audio(&audio).context("Failed to transcribe audio")?;
    println!("   Found {} segments", transcript.segments.len());

    // 3. Calculate chunk boundaries at linguistic breaks
    println!("\n3. Calculating linguistic chunk boundaries...");
    let config = types::ChunkConfig::new(args.target_duration);
    let chunk_boundaries = chunking::calculate_chunk_boundaries(&transcript, config);
    println!(
        "   Created {} chunks at natural breaks",
        chunk_boundaries.len()
    );

    // 4. Slice audio into chunks
    println!("\n4. Slicing audio into chunks...");
    let chunks = audio::slicer::slice_audio(&audio, &chunk_boundaries);
    println!("   Sliced into {} audio chunks", chunks.len());

    // 5. Apply recipe to each chunk and write outputs
    println!("\n5. Applying recipe to each chunk and writing outputs...");
    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("Failed to create output directory {:?}", args.output_dir))?;
    let mut written = 0usize;
    for (i, chunk) in chunks.iter().enumerate() {
        let processed = operations::recipe::apply_recipe(chunk, &recipe);
        if processed.is_empty() {
            eprintln!(
                "   Chunk {} produced no processed segments; skipping",
                i + 1
            );
            continue;
        }

        let chunk_dir = args.output_dir.join(format!("chunk_{:04}", i + 1));
        fs::create_dir_all(&chunk_dir)
            .with_context(|| format!("Failed to create chunk output directory {:?}", chunk_dir))?;

        let processed_audio = match audio::assembler::assemble_audio(&processed) {
            Some(audio) => audio,
            None => {
                eprintln!(
                    "   Failed to assemble processed audio for chunk {}; skipping",
                    i + 1
                );
                continue;
            }
        };

        let output_path = chunk_dir.join("processed.wav");
        audio::encoder::encode_audio(&processed_audio, &output_path).with_context(|| {
            format!(
                "Failed to encode processed audio for chunk {} at {:?}",
                i + 1,
                output_path
            )
        })?;
        println!(
            "   Wrote chunk {:04} to {:?} ({:.3}s → {:.3}s)",
            i + 1,
            output_path,
            chunk.start_time,
            chunk.end_time
        );
        written += 1;
    }
    println!(
        "   Completed writing {} chunk files under {:?}",
        written, args.output_dir
    );

    println!("\n✓ Processing complete!");

    Ok(())
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
        };

        assert_eq!(args.target_duration, 2.0);
    }
}
