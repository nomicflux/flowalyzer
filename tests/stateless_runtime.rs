use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use flowalyzer::audio::capture::{CaptureBuilder, CaptureConfig, CaptureSource};
use flowalyzer::pronunciation::features::{FeatureConfig, FeatureExtractor};
use flowalyzer::pronunciation::session::{SessionConfig, SessionEngine, SessionRuntime};
use flowalyzer::pronunciation::RecordedClip;
use flowalyzer::test_support::sine_wave;

#[test]
fn runtime_spawns_and_shuts_down_without_snapshots_when_never_started() {
    let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let snapshots = handle.drain_snapshots();
    assert!(
        snapshots.is_empty(),
        "no snapshots expected without alignment"
    );
}

#[test]
fn start_stop_cycle_exits_cleanly() {
    let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
    let config = SessionConfig::default();
    let builder = Arc::new(BufferCaptureBuilder::new(Vec::new(), config.sample_rate));
    let (_handle, controller) = SessionRuntime::spawn_with_capture_builder(clip, config, builder);

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(20));

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(20));
}

struct BufferCapture {
    chunks: Vec<Vec<f32>>,
    index: usize,
    sample_rate: u32,
    finished: bool,
}

impl BufferCapture {
    fn new(chunks: Vec<Vec<f32>>, sample_rate: u32) -> Self {
        Self {
            chunks,
            index: 0,
            sample_rate,
            finished: false,
        }
    }
}

impl CaptureSource for BufferCapture {
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn recv_chunk(&mut self, _timeout: Duration) -> Option<Vec<f32>> {
        if self.finished {
            return None;
        }
        if self.index >= self.chunks.len() {
            self.finished = true;
            return None;
        }
        let chunk = self.chunks[self.index].clone();
        self.index += 1;
        Some(chunk)
    }

    fn stop(&mut self) {
        self.finished = true;
    }
}

struct BufferCaptureBuilder {
    chunks: Vec<Vec<f32>>,
    sample_rate: u32,
    fail: bool,
}

impl BufferCaptureBuilder {
    fn new(chunks: Vec<Vec<f32>>, sample_rate: u32) -> Self {
        Self {
            chunks,
            sample_rate,
            fail: false,
        }
    }

    fn failing(sample_rate: u32) -> Self {
        Self {
            chunks: Vec::new(),
            sample_rate,
            fail: true,
        }
    }
}

impl CaptureBuilder for BufferCaptureBuilder {
    fn start_capture(&self, _config: &CaptureConfig) -> Result<Box<dyn CaptureSource>> {
        if self.fail {
            anyhow::bail!("capture failed");
        }
        Ok(Box::new(BufferCapture::new(
            self.chunks.clone(),
            self.sample_rate,
        )))
    }
}

#[test]
fn synthetic_capture_drives_runtime() {
    let config = SessionConfig {
        chunk_duration_ms: 200,
        ..Default::default()
    };
    let reference = sine_wave(config.sample_rate, 220.0, 1.0);
    let clip = RecordedClip::from_samples(reference, config.sample_rate);
    let device_rate = 48_000;
    let capture_signal = sine_wave(device_rate, 220.0, 1.0);
    let device_chunk = 480;
    let chunks: Vec<Vec<f32>> = capture_signal
        .chunks(device_chunk)
        .map(|c| c.to_vec())
        .collect();
    assert!(!chunks.is_empty());
    let builder = Arc::new(BufferCaptureBuilder::new(chunks, device_rate));
    let (handle, controller) =
        SessionRuntime::spawn_with_capture_builder(clip, config.clone(), builder);

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(500));
    controller.stop().unwrap();
    controller.shutdown().unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut processed = None;
    while std::time::Instant::now() < deadline {
        let snapshots = handle.drain_snapshots();
        if let Some(snapshot) = snapshots
            .into_iter()
            .find(|snapshot| !snapshot.alignment.learner_pitch.is_empty())
        {
            processed = Some(snapshot);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let processed = processed.expect("expected processed snapshot");
    let voiced = processed
        .alignment
        .learner_pitch
        .iter()
        .filter(|value| **value > 0.0)
        .count();
    assert!(
        voiced > 0,
        "learner pitch should include voiced frames from synthetic capture"
    );
}

#[test]
#[should_panic]
fn seed_tail_panics_with_insufficient_samples() {
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let required_tail_len = cfg.frame_len_samples - cfg.hop_samples;
    let reference = sine_wave(sample_rate, 220.0, 1.0);
    let extractor = FeatureExtractor::new();
    let reference_features = extractor.extract_reference(&reference, sample_rate, cfg);
    let mut engine = SessionEngine::new(reference_features, sample_rate);
    let insufficient = vec![0.0; required_tail_len - 1];
    engine.seed_tail(&insufficient);
}

struct ErrorCaptureBuilder;

impl CaptureBuilder for ErrorCaptureBuilder {
    fn start_capture(&self, _config: &CaptureConfig) -> Result<Box<dyn CaptureSource>> {
        Err(anyhow::anyhow!("capture device failed"))
    }
}

#[test]
#[should_panic]
fn capture_builder_error_panics() {
    let builder = ErrorCaptureBuilder;
    let config = CaptureConfig::new();
    builder.start_capture(&config).unwrap();
}

#[test]
fn runtime_panics_on_capture_failure() {
    let clip = RecordedClip::from_samples(vec![0.0; 16_000], 16_000);
    let config = SessionConfig::default();
    let builder = Arc::new(BufferCaptureBuilder::failing(config.sample_rate));
    let (handle, controller) = SessionRuntime::spawn_with_capture_builder(clip, config, builder);
    controller.start().unwrap();
    let result = handle.join();
    assert!(
        result.is_err(),
        "runtime thread should panic when capture start fails"
    );
}

#[test]
#[should_panic]
fn resample_with_zero_rate_panics() {
    use flowalyzer::audio::resample::linear_resample;
    let samples = vec![0.5; 100];
    linear_resample(&samples, 0, 16_000).unwrap();
}

#[test]
fn first_snapshot_only_after_tail_seeding_with_real_data() {
    let config = SessionConfig {
        chunk_duration_ms: 200,
        ..Default::default()
    };
    let reference = sine_wave(config.sample_rate, 220.0, 1.0);
    let clip = RecordedClip::from_samples(reference, config.sample_rate);
    let device_rate = 48_000;
    let capture_signal = sine_wave(device_rate, 220.0, 1.0);
    let device_chunk = 480;
    let chunks: Vec<Vec<f32>> = capture_signal
        .chunks(device_chunk)
        .map(|c| c.to_vec())
        .collect();
    let builder = Arc::new(BufferCaptureBuilder::new(chunks, device_rate));
    let (handle, controller) =
        SessionRuntime::spawn_with_capture_builder(clip, config.clone(), builder);
    let snapshots_before = handle.drain_snapshots();
    assert!(
        snapshots_before.is_empty(),
        "no snapshots before start command"
    );
    controller.start().unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut first_snapshot = None;
    while std::time::Instant::now() < deadline {
        let snapshots = handle.drain_snapshots();
        if let Some(snapshot) = snapshots.into_iter().next() {
            first_snapshot = Some(snapshot);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let first_snapshot = first_snapshot.expect("expected first snapshot after tail seeding");
    assert!(
        !first_snapshot.alignment.reference_energy.is_empty(),
        "first snapshot reference_energy must be non-empty"
    );
    assert!(
        !first_snapshot.alignment.learner_energy.is_empty(),
        "first snapshot learner_energy must be non-empty"
    );
    assert!(
        !first_snapshot.alignment.reference_pitch.is_empty(),
        "first snapshot reference_pitch must be non-empty"
    );
    assert!(
        !first_snapshot.alignment.learner_pitch.is_empty(),
        "first snapshot learner_pitch must be non-empty"
    );
    assert!(
        !first_snapshot.alignment.energy_error.is_empty(),
        "first snapshot energy_error must be non-empty"
    );
    assert!(
        !first_snapshot.alignment.similarity_band.is_empty(),
        "first snapshot similarity_band must be non-empty"
    );
    assert!(
        !first_snapshot.alignment.contour_band.is_empty(),
        "first snapshot contour_band must be non-empty"
    );
    controller.stop().unwrap();
    controller.shutdown().unwrap();
}

#[test]
fn processing_stops_naturally_when_reference_exhausted() {
    let config = SessionConfig {
        chunk_duration_ms: 100,
        ..Default::default()
    };
    let reference = sine_wave(config.sample_rate, 220.0, 0.5);
    let clip = RecordedClip::from_samples(reference, config.sample_rate);
    let device_rate = 48_000;
    let capture_signal = sine_wave(device_rate, 220.0, 2.0);
    let device_chunk = 480;
    let chunks: Vec<Vec<f32>> = capture_signal
        .chunks(device_chunk)
        .map(|c| c.to_vec())
        .collect();
    let builder = Arc::new(BufferCaptureBuilder::new(chunks, device_rate));
    let (handle, controller) =
        SessionRuntime::spawn_with_capture_builder(clip, config.clone(), builder);
    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(1000));
    let all_snapshots = handle.drain_snapshots();
    assert!(
        !all_snapshots.is_empty(),
        "should have some snapshots before reference exhausted"
    );
    for snapshot in &all_snapshots {
        assert!(
            !snapshot.alignment.reference_energy.is_empty(),
            "all snapshots must have real alignment data"
        );
        assert!(
            !snapshot.alignment.learner_energy.is_empty(),
            "all snapshots must have real alignment data"
        );
    }
}

#[test]
fn shadowing_aligns_capture_and_reference() {
    let config = SessionConfig {
        chunk_duration_ms: 150,
        ..Default::default()
    };
    let reference = sine_wave(config.sample_rate, 220.0, 1.0);
    let clip = RecordedClip::from_samples(reference, config.sample_rate);
    let device_rate = 48_000;
    let capture_signal = sine_wave(device_rate, 220.0, 1.0);
    let device_chunk = 480;
    let chunks: Vec<Vec<f32>> = capture_signal
        .chunks(device_chunk)
        .map(|c| c.to_vec())
        .collect();
    let builder = Arc::new(BufferCaptureBuilder::new(chunks, device_rate));
    let (handle, controller) =
        SessionRuntime::spawn_with_capture_builder(clip, config.clone(), builder);

    controller.shadow().unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut snapshots = Vec::new();
    while std::time::Instant::now() < deadline {
        let batch = handle.drain_snapshots();
        snapshots.extend(batch.into_iter().filter(|s| s.recording));
        std::thread::sleep(Duration::from_millis(20));
    }
    let best = snapshots
        .into_iter()
        .min_by(|a, b| {
            let a_err = a
                .alignment
                .energy_error
                .iter()
                .map(|v| v.abs())
                .fold(f32::INFINITY, f32::min);
            let b_err = b
                .alignment
                .energy_error
                .iter()
                .map(|v| v.abs())
                .fold(f32::INFINITY, f32::min);
            a_err
                .partial_cmp(&b_err)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("expected snapshots during shadowing");
    let alignment = best.alignment;
    assert!(
        alignment.energy_error.iter().all(|v| v.is_finite()),
        "energy error should be finite"
    );
    assert!(
        alignment.similarity_band.iter().all(|v| v.is_finite()),
        "similarity should be finite"
    );
    assert!(
        alignment.contour_band.iter().all(|v| v.is_finite()),
        "contour should be finite"
    );
    controller.stop().unwrap();
    controller.shutdown().unwrap();
}
