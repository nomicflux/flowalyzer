use std::time::Duration;

use flowalyzer::pronunciation::session::{ClipVariant, SessionConfig, SessionRuntime};
use flowalyzer::pronunciation::RecordedClip;
use flowalyzer::types::{Recipe, RecipeStep};

fn simple_recipe() -> Recipe {
    Recipe::new("test").add_step(RecipeStep {
        repeat_count: 1,
        speed_factor: 1.0,
        silent: false,
    })
}

#[test]
fn applying_recipe_flags_flowalyzed_clip() {
    let clip = RecordedClip::from_samples(vec![0.5; 32_000], 16_000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    controller
        .apply_recipe(0.0, 2.0, simple_recipe())
        .expect("apply recipe");
    std::thread::sleep(Duration::from_millis(50));

    let snapshots = handle.drain_snapshots();
    assert!(
        snapshots.iter().any(|snap| snap.has_flowalyzed_clip),
        "expected a snapshot indicating flowalyzed clip exists"
    );

    controller.shutdown().ok();
}

#[test]
fn toggle_clip_variant_switches_active_state() {
    let clip = RecordedClip::from_samples(vec![0.25; 32_000], 16_000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    controller
        .apply_recipe(0.0, 2.0, simple_recipe())
        .expect("apply recipe");
    std::thread::sleep(Duration::from_millis(50));
    controller
        .toggle_clip_variant(ClipVariant::Flowalyzed)
        .expect("toggle to flowalyzed");
    let mut snapshots = Vec::new();
    for _ in 0..20 {
        snapshots.extend(handle.drain_snapshots());
        if snapshots
            .iter()
            .any(|snap| snap.active_clip_variant == ClipVariant::Flowalyzed)
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        snapshots
            .iter()
            .any(|snap| snap.active_clip_variant == ClipVariant::Flowalyzed),
        "expected snapshot showing flowalyzed variant active"
    );

    controller.shutdown().ok();
}
