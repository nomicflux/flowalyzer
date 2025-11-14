use anyhow::Result;
use flowalyzer::pronunciation::{
    AlignmentWeights, ClipVariant, SessionConfig, SessionHandle, SessionRuntime,
};
use flowalyzer::types::{Recipe, RecipeStep};
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const SAMPLE_RATE: u32 = 16_000;

fn wait_for_snapshot(
    handle: &SessionHandle,
    timeout: Duration,
) -> Option<flowalyzer::pronunciation::SessionSnapshot> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(snapshot) = handle.try_recv() {
            return Some(snapshot);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    handle.try_recv()
}

fn sine_wave(frequency: f32, duration_secs: f32) -> Vec<f32> {
    let total_samples = (SAMPLE_RATE as f32 * duration_secs) as usize;
    (0..total_samples)
        .map(|index| {
            let t = index as f32 / SAMPLE_RATE as f32;
            (2.0 * std::f32::consts::PI * frequency * t).sin()
        })
        .collect()
}

fn create_test_recipe() -> Recipe {
    Recipe::new("test recipe").add_step(RecipeStep {
        repeat_count: 2,
        speed_factor: 1.0,
        silent: false,
    })
}

fn create_session_runtime() -> Result<(SessionRuntime, TempDir)> {
    let samples = sine_wave(440.0, 1.0);
    let temp_dir = TempDir::new()?;
    let wav_path = temp_dir.path().join("test_reference.wav");
    write_test_wav(&wav_path, &samples)?;
    let config = create_test_config(wav_path);
    let runtime = SessionRuntime::new(config)?;
    Ok((runtime, temp_dir))
}

fn write_test_wav(wav_path: &std::path::Path, samples: &[f32]) -> Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(wav_path, spec)?;
    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_sample = (clamped * 32767.0) as i16;
        writer.write_sample(i16_sample)?;
    }
    writer.finalize()?;
    Ok(())
}

fn create_test_config(wav_path: PathBuf) -> SessionConfig {
    SessionConfig::new(
        wav_path,
        PathBuf::from("assets"),
        flowalyzer::pronunciation::CaptureSettings {
            device_name: None,
            sample_rate: SAMPLE_RATE,
            latency_ms: RangeInclusive::new(50, 200),
        },
        AlignmentWeights::default(),
    )
}

#[test]
fn test_apply_recipe_command_idle() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    let recipe = create_test_recipe();
    controller.apply_recipe(0.0, 0.5, recipe)?;

    let snapshot = wait_for_snapshot(&handle, Duration::from_secs(2));
    assert!(snapshot.is_some(), "should receive snapshot after command");
    let snapshot = snapshot.unwrap();
    assert!(snapshot.error.is_none(), "should have no error");

    Ok(())
}

#[test]
fn test_toggle_variant_command_idle() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    controller.toggle_clip_variant(ClipVariant::Flowalyzed)?;

    let snapshot = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(snapshot.is_some(), "should receive snapshot after command");
    let snapshot = snapshot.unwrap();
    assert!(
        snapshot.error.is_some(),
        "should have error when toggling to Flowalyzed without applying recipe"
    );

    Ok(())
}

#[test]
fn test_apply_recipe_command_during_recording() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    controller.start()?;
    std::thread::sleep(Duration::from_millis(100));

    let recipe = create_test_recipe();
    let result = controller.apply_recipe(0.0, 0.5, recipe);
    assert!(
        result.is_ok(),
        "command should be sent without error during recording"
    );

    controller.stop()?;
    Ok(())
}

#[test]
fn test_toggle_variant_command_during_recording() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    controller.start()?;
    std::thread::sleep(Duration::from_millis(100));

    let result = controller.toggle_clip_variant(ClipVariant::Flowalyzed);
    assert!(
        result.is_ok(),
        "command should be sent without error during recording"
    );

    controller.stop()?;
    Ok(())
}
