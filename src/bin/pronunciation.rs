use std::ops::Range;
use std::path::PathBuf;

use anyhow::{anyhow, bail, Result};
use clap::Parser;
use flowalyzer::pronunciation::{run_session, SessionConfig};

#[derive(Parser, Debug)]
#[command(
    name = "pronunciation",
    about = "Prototype pronunciation analysis binary (Phase 1 scaffolding)"
)]
struct Cli {
    /// Optional reference audio to load during analysis
    #[arg(long, value_name = "PATH")]
    reference: Option<PathBuf>,
    /// Optional transcript text used for future phoneme alignment
    #[arg(long, value_name = "TEXT")]
    transcript: Option<String>,
    /// Start frame (inclusive) for analysis window in milliseconds
    #[arg(long)]
    analysis_start_ms: Option<u32>,
    /// End frame (exclusive) for analysis window in milliseconds
    #[arg(long)]
    analysis_end_ms: Option<u32>,
    /// Disable interactive UI even when available
    #[arg(long)]
    no_ui: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = build_config(&cli)?;
    run_session(config).map_err(|err| anyhow!(err))?;
    Ok(())
}

fn build_config(cli: &Cli) -> Result<SessionConfig> {
    let analysis_window = parse_window(cli.analysis_start_ms, cli.analysis_end_ms)?;
    Ok(SessionConfig {
        reference_wav: cli.reference.clone(),
        transcript: cli.transcript.clone(),
        analysis_window,
        ui_enabled: !cli.no_ui,
    })
}

fn parse_window(start: Option<u32>, end: Option<u32>) -> Result<Option<Range<u32>>> {
    match (start, end) {
        (Some(s), Some(e)) if s < e => Ok(Some(s..e)),
        (Some(_), Some(_)) => bail!("analysis_end_ms must be greater than analysis_start_ms"),
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            bail!("provide both analysis_start_ms and analysis_end_ms or neither")
        }
    }
}
