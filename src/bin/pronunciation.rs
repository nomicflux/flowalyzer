use std::path::PathBuf;

use clap::Parser;
use eframe::NativeOptions;
use flowalyzer::pronunciation::load_clip;
use flowalyzer::pronunciation::session::{SessionConfig, SessionRuntime};
use flowalyzer::ui::screens::session::SessionApp;

#[derive(Parser, Debug)]
#[command(name = "flowalyzer")]
#[command(about = "Run the Flowalyzer pronunciation UI")]
struct Args {
    /// Reference clip to shadow (wav)
    #[arg(long)]
    reference: PathBuf,

    /// Engine sample rate (defaults to reference clip rate)
    #[arg(long)]
    engine_sample_rate: Option<u32>,

    /// Capture device sample rate (defaults to engine rate)
    #[arg(long)]
    capture_sample_rate: Option<u32>,
}

fn main() {
    let args = Args::parse();
    let clip = load_clip(&args.reference).unwrap();
    let config = SessionConfig {
        sample_rate: args.engine_sample_rate.unwrap_or(clip.sample_rate),
        capture_sample_rate: args.capture_sample_rate,
        chunk_duration_ms: 100,
        latency_budget_ms: 200,
        latency_range: 100..=200,
    };
    let (handle, controller) = SessionRuntime::spawn(clip, config);
    let app = SessionApp::new(handle, controller);
    let options = NativeOptions::default();
    eframe::run_native(
        "Flowalyzer Pronunciation",
        options,
        Box::new(|_cc| Box::new(app)),
    )
    .unwrap();
}
