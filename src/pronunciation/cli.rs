use std::ops::RangeInclusive;
use std::path::PathBuf;

use anyhow::{ensure, Result};
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "pronunciation",
    about = "Real-time pronunciation session utility (audio capture helpers included)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Launch the interactive pronunciation session UI.
    Session(SessionArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CaptureArgs {
    /// Optional input device name.
    #[arg(long)]
    pub device: Option<String>,
    /// Minimum latency in milliseconds for capture buffering.
    #[arg(long = "latency-min")]
    pub latency_min: Option<u32>,
    /// Maximum latency in milliseconds for capture buffering.
    #[arg(long = "latency-max")]
    pub latency_max: Option<u32>,
    /// Target sample rate for capture/playback.
    #[arg(long, default_value_t = 16_000)]
    pub sample_rate: u32,
}

impl CaptureArgs {
    pub fn latency_range(&self) -> Result<RangeInclusive<u32>> {
        match (self.latency_min, self.latency_max) {
            (Some(min), Some(max)) => {
                ensure!(min > 0, "latency_min must be positive");
                ensure!(max >= min, "latency_max must be >= latency_min");
                Ok(min..=max)
            }
            (None, None) => Ok(100..=200),
            _ => anyhow::bail!("provide both latency-min and latency-max or neither"),
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct PipelineArgs {
    /// Path to the reference pronunciation WAV file.
    #[arg(long)]
    pub reference: PathBuf,
    #[command(flatten)]
    pub capture: CaptureArgs,
    /// Optional override for the assets directory.
    #[arg(long = "assets-path")]
    pub assets_path: Option<PathBuf>,
    /// Maximum acceptable capture-to-feedback latency in milliseconds.
    #[arg(long = "latency-budget", default_value_t = 200)]
    pub latency_budget_ms: u32,
}

#[derive(Parser, Debug, Clone)]
pub struct SessionArgs {
    #[command(flatten)]
    pub pipeline: PipelineArgs,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;

    #[test]
    fn parses_and_validates_latency_range() {
        let cli = Cli::try_parse_from([
            "pronunciation",
            "session",
            "--reference",
            "ref.wav",
            "--latency-min",
            "120",
            "--latency-max",
            "180",
        ])
        .unwrap();
        let Command::Session(args) = cli.command;
        let range = args.pipeline.capture.latency_range().unwrap();
        assert_eq!((*range.start(), *range.end()), (120, 180));
    }

    #[test]
    fn rejects_partial_latency_override() {
        let cli = Cli::try_parse_from([
            "pronunciation",
            "session",
            "--reference",
            "ref.wav",
            "--latency-min",
            "150",
        ])
        .unwrap();
        let Command::Session(args) = cli.command;
        assert!(args.pipeline.capture.latency_range().is_err());
    }

    #[test]
    fn session_defaults_to_latency_range() {
        let cli =
            Cli::try_parse_from(["pronunciation", "session", "--reference", "ref.wav"]).unwrap();
        let Command::Session(args) = cli.command;
        let capture = &args.pipeline.capture;
        let range = capture.latency_range().unwrap();
        assert_eq!((*range.start(), *range.end()), (100, 200));
        assert_eq!(capture.sample_rate, 16_000);
        assert_eq!(args.pipeline.latency_budget_ms, 200);
    }
}
