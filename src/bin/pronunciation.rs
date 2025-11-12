use std::sync::Once;

use anyhow::Result;
use clap::Parser;
use flowalyzer::config::AppConfig;
use flowalyzer::pronunciation::cli::{Cli, Command, PipelineArgs, SessionArgs};
use flowalyzer::pronunciation::{run_session, AlignmentWeights, CaptureSettings, SessionConfig};
use tracing::info;
use tracing_subscriber::EnvFilter;

static INIT_TRACING: Once = Once::new();

fn main() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    info!("pronunciation CLI parsed");
    match cli.command {
        Command::Session(args) => handle_session(&args),
    }
}

fn handle_session(args: &SessionArgs) -> Result<()> {
    info!(?args, "handling session command");
    let config = build_session_config(&args.pipeline, true)?;
    info!(reference = %config.reference_wav.display(), latency_budget_ms = config.latency_budget_ms, "built session config");
    let runtime = run_session(config)?;
    runtime.launch()?;
    Ok(())
}

fn build_session_config(args: &PipelineArgs, ui_enabled: bool) -> Result<SessionConfig> {
    let assets = AppConfig::from_override(args.assets_path.clone())?;
    info!(
        assets_root = %assets.assets_root.display(),
        "resolved assets root"
    );
    let alignment = AlignmentWeights::load_from_assets(&assets.assets_root)?;
    info!(
        mfcc = alignment.mfcc,
        delta = alignment.delta,
        delta_delta = alignment.delta_delta,
        mel = alignment.mel,
        energy = alignment.energy,
        flux = alignment.flux,
        pitch = alignment.pitch,
        "loaded alignment weights"
    );
    let latency_range = args.capture.latency_range()?;
    let capture = CaptureSettings::new(
        args.capture.device.clone(),
        args.capture.sample_rate,
        latency_range.clone(),
    );
    info!(
        device = ?capture.device_name,
        sample_rate = capture.sample_rate,
        latency_ms = ?capture.latency_ms,
        "configured capture settings"
    );
    Ok(SessionConfig::new(
        args.reference.clone(),
        assets.assets_root,
        capture,
        alignment,
    )
    .with_latency_budget(args.latency_budget_ms)
    .with_ui(ui_enabled))
}

fn init_tracing() {
    INIT_TRACING.call_once(|| {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    });
}
