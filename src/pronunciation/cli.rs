use std::ops::RangeInclusive;
use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "pronunciation", about = "Audio capture and playback utility")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Record microphone audio to a WAV file.
    Record(RecordArgs),
    /// Play an existing audio file through the default output device.
    Play(PlayArgs),
    /// Record audio and immediately play it back.
    RecordAndPlay(RecordArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct RecordArgs {
    /// Recording duration in seconds.
    #[arg(long, default_value_t = 5.0)]
    pub duration: f32,
    /// Optional input device name.
    #[arg(long)]
    pub device: Option<String>,
    /// Minimum latency in milliseconds for capture buffering.
    #[arg(long = "latency-min")]
    pub latency_min: Option<u32>,
    /// Maximum latency in milliseconds for capture buffering.
    #[arg(long = "latency-max")]
    pub latency_max: Option<u32>,
    /// Target sample rate for the captured audio.
    #[arg(long, default_value_t = 16_000)]
    pub sample_rate: u32,
    /// Output WAV path for the recorded audio.
    #[arg(long, default_value = "recording.wav")]
    pub output: PathBuf,
}

#[derive(Parser, Debug)]
pub struct PlayArgs {
    /// Path to the audio file to play.
    #[arg(long)]
    pub file: PathBuf,
}

pub fn latency_range(args: &RecordArgs) -> Result<RangeInclusive<u32>> {
    match (args.latency_min, args.latency_max) {
        (Some(min), Some(max)) => {
            ensure!(min > 0, "latency_min must be positive");
            ensure!(max >= min, "latency_max must be >= latency_min");
            Ok(min..=max)
        }
        (None, None) => Ok(100..=200),
        _ => anyhow::bail!("provide both latency-min and latency-max or neither"),
    }
}

#[cfg(test)]
mod tests {
    use super::{latency_range, Cli, Command};
    use clap::Parser;

    #[test]
    fn parses_and_validates_latency_range() {
        let cli = Cli::try_parse_from([
            "pronunciation",
            "record",
            "--duration",
            "2.5",
            "--latency-min",
            "120",
            "--latency-max",
            "180",
        ])
        .unwrap();
        match cli.command {
            Command::Record(args) => {
                let range = latency_range(&args).unwrap();
                assert_eq!((*range.start(), *range.end()), (120, 180));
            }
            other => panic!("unexpected command parsed: {:?}", other),
        }
    }

    #[test]
    fn rejects_partial_latency_override() {
        let cli = Cli::try_parse_from(["pronunciation", "record", "--latency-min", "150"]).unwrap();
        match cli.command {
            Command::Record(args) => {
                assert!(latency_range(&args).is_err());
            }
            other => panic!("unexpected command parsed: {:?}", other),
        }
    }
}
