use anyhow::Result;
use clap::Parser;
use flowalyzer::config::AppConfig;
use flowalyzer::pronunciation::cli::{Cli, Command, PipelineArgs, SessionArgs};
use flowalyzer::pronunciation::{run_session, AlignmentWeights, CaptureSettings, SessionConfig};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Session(args) => handle_session(&args),
    }
}

fn handle_session(args: &SessionArgs) -> Result<()> {
    let config = build_session_config(&args.pipeline, true)?;
    run_session(config)?;
    Ok(())
}

fn build_session_config(args: &PipelineArgs, ui_enabled: bool) -> Result<SessionConfig> {
    let assets = AppConfig::from_override(args.assets_path.clone())?;
    let alignment = AlignmentWeights::load_from_assets(&assets.assets_root)?;
    let capture = CaptureSettings::new(
        args.capture.device.clone(),
        args.capture.sample_rate,
        args.capture.latency_range()?,
    );
    Ok(SessionConfig::new(
        args.reference.clone(),
        args.learner.clone(),
        assets.assets_root,
        capture,
        alignment,
    )
    .with_ui(ui_enabled))
}
