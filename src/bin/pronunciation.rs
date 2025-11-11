use std::time::Duration;

use anyhow::{ensure, Context, Result};
use clap::Parser;
use flowalyzer::audio::capture::{record_audio, CaptureConfig};
use flowalyzer::audio::encoder;
use flowalyzer::audio::playback;
use flowalyzer::pronunciation::cli::{latency_range, Cli, Command, PlayArgs, RecordArgs};
use flowalyzer::types::AudioData;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Record(args) => handle_record(&args),
        Command::Play(args) => handle_play(&args),
        Command::RecordAndPlay(args) => handle_record_and_play(&args),
    }
}

fn handle_record(args: &RecordArgs) -> Result<()> {
    let audio = capture_audio(args)?;
    persist_audio(&audio, &args.output)?;
    println!(
        "Recorded {} samples to {:?}",
        audio.samples.len(),
        args.output
    );
    Ok(())
}

fn handle_play(args: &PlayArgs) -> Result<()> {
    playback::play_file(&args.file)
}

fn handle_record_and_play(args: &RecordArgs) -> Result<()> {
    let audio = capture_audio(args)?;
    persist_audio(&audio, &args.output)?;
    playback::play_audio(&audio)?;
    println!("Recorded and played back {:?}", args.output);
    Ok(())
}

fn capture_audio(args: &RecordArgs) -> Result<AudioData> {
    ensure!(args.duration > 0.0, "duration must be positive");
    ensure!(args.sample_rate > 0, "sample rate must be positive");
    let mut config = CaptureConfig::new(Duration::from_secs_f32(args.duration));
    config.device_name = args.device.clone();
    config.sample_rate = args.sample_rate;
    config.latency_ms = latency_range(args)?;
    record_audio(&config)
}

fn persist_audio(audio: &AudioData, output: &std::path::PathBuf) -> Result<()> {
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {:?}", parent))?;
        }
    }
    encoder::encode_audio(audio, output)
}
