use std::f32::consts::PI;

use flowalyzer::pronunciation::features::{FeatureConfig, FeatureExtractor};

fn lcg_noise(len: usize) -> Vec<f32> {
    // Deterministic pseudo-random noise in [-1, 1]
    let mut state: u32 = 1_234_567;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state = state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let val = ((state >> 8) & 0x00FF_FFFF) as f32 / 0x00FF_FFFF as f32;
        out.push(val * 2.0 - 1.0);
    }
    out
}

fn sine_wave(len: usize, freq: f32, sample_rate: u32) -> Vec<f32> {
    (0..len)
        .map(|i| (2.0 * PI * freq * i as f32 / sample_rate as f32).sin())
        .collect()
}

fn ramp_sine(len: usize, freq: f32, sample_rate: u32) -> Vec<f32> {
    (0..len)
        .map(|i| {
            let amp = 0.1 + 0.9 * (i as f32 / len as f32);
            amp * (2.0 * PI * freq * i as f32 / sample_rate as f32).sin()
        })
        .collect()
}

#[test]
fn given_white_noise_frames_then_pitch_reports_unvoiced_zero() {
    // Given deterministic white noise across multiple frames
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let frame_count = 4;
    let total_len = cfg.frame_len_samples + cfg.hop_samples * (frame_count - 1);
    let samples = lcg_noise(total_len);

    // When we extract reference features
    let extractor = FeatureExtractor::new();
    let features = extractor.extract_reference(&samples, sample_rate, cfg);

    // Then all pitch estimates should be zero (unvoiced)
    assert_eq!(features.pitch.len(), frame_count);
    assert!(
        features.pitch.iter().all(|p| *p == 0.0),
        "noise should be treated as unvoiced; got {:?}", features.pitch
    );
}

#[test]
fn given_silence_then_pitch_is_zero() {
    // Given pure silence covering multiple frames
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let frame_count = 3;
    let total_len = cfg.frame_len_samples + cfg.hop_samples * (frame_count - 1);
    let samples = vec![0.0; total_len];

    // When we extract reference features
    let extractor = FeatureExtractor::new();
    let features = extractor.extract_reference(&samples, sample_rate, cfg);

    // Then pitch should be zero everywhere
    assert_eq!(features.pitch.len(), frame_count);
    assert!(features.pitch.iter().all(|p| *p == 0.0));
}

#[test]
fn given_voiced_sine_then_pitch_matches_frequency() {
    // Given a clean 220 Hz sine across multiple frames
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let frame_count = 4;
    let freq = 220.0;
    let total_len = cfg.frame_len_samples + cfg.hop_samples * (frame_count - 1);
    let samples = sine_wave(total_len, freq, sample_rate);

    // When we extract reference features
    let extractor = FeatureExtractor::new();
    let features = extractor.extract_reference(&samples, sample_rate, cfg);

    // Then each frame's pitch is near 220 Hz
    assert_eq!(features.pitch.len(), frame_count);
    for p in features.pitch {
        assert!(
            (p - freq).abs() < 5.0,
            "expected ~{freq}Hz, got {p}"
        );
    }
}

#[test]
fn given_soft_to_loud_ramp_then_pitch_stays_stable() {
    // Given a sine at fixed frequency with amplitude ramping up
    let sample_rate = 16_000;
    let cfg = FeatureConfig::from_sample_rate(sample_rate);
    let frame_count = 5;
    let freq = 180.0;
    let total_len = cfg.frame_len_samples + cfg.hop_samples * (frame_count - 1);
    let samples = ramp_sine(total_len, freq, sample_rate);

    // When we extract reference features
    let extractor = FeatureExtractor::new();
    let features = extractor.extract_reference(&samples, sample_rate, cfg);

    // Then pitch remains near the target despite amplitude changes
    assert_eq!(features.pitch.len(), frame_count);
    for p in features.pitch {
        assert!(
            (p - freq).abs() < 6.0,
            "expected ~{freq}Hz stable across ramp, got {p}"
        );
    }
}
