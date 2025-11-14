use anyhow::Result;
use flowalyzer::pronunciation::{ClipVariant, SessionConfig, SessionHandle, SessionRuntime};
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
        flowalyzer::pronunciation::AlignmentWeights::default(),
    )
}

#[test]
fn test_toggle_original_to_flowalyzed() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    let recipe = create_test_recipe();
    controller.apply_recipe(0.0, 0.5, recipe)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_secs(2));
    assert!(
        snapshot.is_some(),
        "should receive snapshot after recipe application"
    );
    let snapshot = snapshot.unwrap();
    assert!(
        snapshot.error.is_none(),
        "recipe application should succeed"
    );
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Flowalyzed,
        "active clip should be Flowalyzed after recipe application"
    );

    controller.toggle_clip_variant(ClipVariant::Original)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(snapshot.is_some(), "should receive snapshot after toggle");
    let snapshot = snapshot.unwrap();
    assert!(snapshot.error.is_none(), "toggle should succeed");
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Original,
        "active clip should be Original after toggle"
    );

    Ok(())
}

#[test]
fn test_toggle_flowalyzed_to_original() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    let recipe = create_test_recipe();
    controller.apply_recipe(0.0, 0.5, recipe)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_secs(2));
    assert!(
        snapshot.is_some(),
        "should receive snapshot after recipe application"
    );
    let snapshot = snapshot.unwrap();
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Flowalyzed,
        "should start with Flowalyzed after recipe"
    );

    controller.toggle_clip_variant(ClipVariant::Original)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(
        snapshot.is_some(),
        "should receive snapshot after toggle to Original"
    );
    let snapshot = snapshot.unwrap();
    assert!(
        snapshot.error.is_none(),
        "toggle to Original should succeed"
    );
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Original,
        "active clip should be Original"
    );

    controller.toggle_clip_variant(ClipVariant::Flowalyzed)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(
        snapshot.is_some(),
        "should receive snapshot after toggle back to Flowalyzed"
    );
    let snapshot = snapshot.unwrap();
    assert!(
        snapshot.error.is_none(),
        "toggle back to Flowalyzed should succeed"
    );
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Flowalyzed,
        "active clip should be Flowalyzed again"
    );

    Ok(())
}

#[test]
fn test_toggle_to_flowalyzed_when_none_exists() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    controller.toggle_clip_variant(ClipVariant::Flowalyzed)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(
        snapshot.is_some(),
        "should receive snapshot after toggle attempt"
    );
    let snapshot = snapshot.unwrap();
    assert!(
        snapshot.error.is_some(),
        "should have error when toggling to Flowalyzed without applying recipe"
    );
    let error_msg = snapshot.error.unwrap();
    assert!(
        error_msg.contains("flowalyzed") || error_msg.contains("recipe"),
        "error message should mention flowalyzed clip or recipe"
    );
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Original,
        "active clip should remain Original when toggle fails"
    );

    Ok(())
}

#[test]
fn test_cache_reuse_on_toggle() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    let recipe1 = Recipe::new("first recipe").add_step(RecipeStep {
        repeat_count: 2,
        speed_factor: 1.0,
        silent: false,
    });
    controller.apply_recipe(0.0, 0.3, recipe1)?;
    let snapshot1 = wait_for_snapshot(&handle, Duration::from_secs(5));
    assert!(
        snapshot1.is_some(),
        "should receive snapshot after first recipe"
    );
    assert!(
        snapshot1.unwrap().error.is_none(),
        "first recipe should succeed"
    );

    controller.toggle_clip_variant(ClipVariant::Original)?;
    let _ = wait_for_snapshot(&handle, Duration::from_millis(500));

    controller.toggle_clip_variant(ClipVariant::Flowalyzed)?;
    let snapshot2 = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(
        snapshot2.is_some(),
        "should receive snapshot after toggle back"
    );
    assert!(
        snapshot2.unwrap().error.is_none(),
        "toggle should succeed and reuse cached features"
    );

    Ok(())
}

#[test]
fn test_snapshot_active_variant_field() -> Result<()> {
    let (runtime, _temp_dir) = create_session_runtime()?;
    let handle = runtime.into_handle();
    let controller = handle.controller();

    let initial_snapshot = handle.initial_snapshot();
    assert_eq!(
        initial_snapshot.active_clip_variant,
        ClipVariant::Original,
        "initial snapshot should have Original variant"
    );

    let recipe = create_test_recipe();
    controller.apply_recipe(0.0, 0.5, recipe)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_secs(2));
    assert!(snapshot.is_some(), "should receive snapshot after recipe");
    let snapshot = snapshot.unwrap();
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Flowalyzed,
        "snapshot should reflect Flowalyzed variant after recipe"
    );

    controller.toggle_clip_variant(ClipVariant::Original)?;
    let snapshot = wait_for_snapshot(&handle, Duration::from_millis(500));
    assert!(snapshot.is_some(), "should receive snapshot after toggle");
    let snapshot = snapshot.unwrap();
    assert_eq!(
        snapshot.active_clip_variant,
        ClipVariant::Original,
        "snapshot should reflect Original variant after toggle"
    );

    Ok(())
}
