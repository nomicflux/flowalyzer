use std::time::Duration;

use flowalyzer::pronunciation::session::{SessionConfig, SessionRuntime};
use flowalyzer::pronunciation::RecordedClip;

#[test]
fn runtime_spawns_and_shuts_down_without_snapshots_when_never_started() {
    let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let snapshots = handle.drain_snapshots();
    assert!(
        !snapshots.is_empty(),
        "expected initial snapshot even when runtime never started"
    );
}

#[test]
fn start_stop_cycle_exits_cleanly() {
    let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
    let config = SessionConfig::default();
    let (_handle, controller) = SessionRuntime::spawn(clip, config);

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(20));

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(20));
}
