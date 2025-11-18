use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use flowalyzer::pronunciation::session::{SessionConfig, SessionEngine};
use flowalyzer::pronunciation::{load_clip, RecordedClip};

#[derive(Parser, Debug)]
#[command(name = "session-headless")]
#[command(about = "Run the stateless pronunciation engine against recorded clips without the UI.")]
struct Args {
    /// Reference clip path (used for alignment and scoring)
    #[arg(long)]
    reference: PathBuf,

    /// Learner clip path; defaults to the reference clip if omitted
    #[arg(long)]
    learner: Option<PathBuf>,

    /// Chunk duration in milliseconds (default 100ms)
    #[arg(long, default_value_t = 100)]
    chunk_ms: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let reference = load_clip_checked(&args.reference)?;
    let learner = match &args.learner {
        Some(path) => load_clip_checked(path)?,
        None => reference.clone(),
    };

    let config = SessionConfig {
        chunk_duration_ms: args.chunk_ms.max(10),
        ..SessionConfig::default()
    };
    let mut engine = SessionEngine::new(&reference.samples, &config);

    println!(
        "Reference duration: {:.2}s | Learner duration: {:.2}s | chunk={} samples",
        reference.duration.as_secs_f32(),
        learner.duration.as_secs_f32(),
        config.sample_rate * config.chunk_duration_ms / 1_000
    );

    run_headless(&mut engine, &learner);
    Ok(())
}

fn load_clip_checked(path: &PathBuf) -> Result<RecordedClip> {
    load_clip(path).with_context(|| format!("failed to load clip {:?}", path))
}

fn run_headless(engine: &mut SessionEngine, learner: &RecordedClip) {
    let chunk_size =
        (learner.sample_rate * SessionConfig::default().chunk_duration_ms) as usize / 1_000;
    let mut chunk_index = 0usize;
    let samples: Vec<f32> = learner.samples.iter().copied().collect();

    for chunk in samples.chunks(chunk_size) {
        if chunk.is_empty() {
            continue;
        }
        let report = engine.process_chunk(chunk);
        println!(
            "chunk {:03} offset={:7.2}ms confidence={:.3} frames={}",
            chunk_index,
            report.global_time_offset_ms,
            report.confidence,
            report.reference_energy.len()
        );
        chunk_index += 1;
    }

    println!("Processed {} chunks headlessly.", chunk_index);
}
