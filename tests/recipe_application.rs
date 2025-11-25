use flowalyzer::pronunciation::session::{SessionConfig, SessionRuntime};
use flowalyzer::pronunciation::RecordedClip;

#[test]
fn minimal_commands_start_and_stop() {
    let clip = RecordedClip::from_samples(vec![0.5; 32_000], 16_000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    controller.start().ok();
    controller.stop().ok();
    controller.shutdown().ok();

    let snapshots = handle.drain_snapshots();
    assert!(
        snapshots.is_empty(),
        "no alignment snapshots expected without capture chunks"
    );
}
