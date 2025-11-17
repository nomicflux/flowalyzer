use std::f32::consts::PI;

use flowalyzer::pronunciation::features::{compute_energy_frames, compute_pitch_frames};

#[test]
fn energy_frames_from_silence_are_zero() {
    let samples = vec![0.0; 320];
    let energy = compute_energy_frames(&samples);
    assert!(!energy.is_empty());
    assert!(energy.iter().all(|value| value.abs() < 1e-6));
}

#[test]
fn energy_frames_detect_signal() {
    let samples: Vec<f32> = (0..320)
        .map(|i| (2.0 * PI * 100.0 * i as f32 / 16_000.0).sin())
        .collect();
    let energy = compute_energy_frames(&samples);
    assert!(!energy.is_empty());
    assert!(energy.iter().all(|value| *value > 0.0));
}

#[test]
fn pitch_frames_zero_for_silence() {
    let samples = vec![0.0; 1600];
    let pitches = compute_pitch_frames(&samples);
    assert!(pitches.iter().all(|value| value.abs() < f32::EPSILON));
}

#[test]
fn pitch_frames_detect_tone() {
    let samples: Vec<f32> = (0..16_000)
        .map(|i| (2.0 * PI * 220.0 * i as f32 / 16_000.0).sin())
        .collect();
    let pitches = compute_pitch_frames(&samples);
    let voiced: Vec<f32> = pitches.into_iter().filter(|value| *value > 0.0).collect();
    assert!(
        !voiced.is_empty(),
        "expected voiced frames for sustained tone"
    );
    let avg_pitch = voiced.iter().sum::<f32>() / voiced.len() as f32;
    assert!(avg_pitch > 150.0 && avg_pitch < 300.0);
}
