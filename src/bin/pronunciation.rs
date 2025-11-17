use std::path::PathBuf;

use anyhow::Result;
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
}

fn main() -> Result<()> {
    let args = Args::parse();
    let clip = load_clip(&args.reference)?;
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);
    let app = SessionApp::new(handle, controller);
    let options = NativeOptions::default();
    if let Err(err) = eframe::run_native(
        "Flowalyzer Pronunciation",
        options,
        Box::new(|_cc| Box::new(app)),
    ) {
        eprintln!("failed to launch UI: {err}");
    }
    Ok(())
}
