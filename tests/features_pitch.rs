use std::sync::Arc;
use std::time::Duration;

use flowalyzer::pronunciation::features::FeatureExtractor;
use flowalyzer::pronunciation::RecordedClip;
use ndarray::Array1;

const SAMPLE_RATE: u32 = 16_000;
const DURATION_SECONDS: f32 = 0.8;

#[test]
fn contour_remains_invariant_under_octave_shift() {
    let rising = glide_clip(220.0, 440.0);
    let shifted = glide_clip(440.0, 880.0);

    let extractor = FeatureExtractor::new();
    let base = extractor.extract(&rising).expect("extract base");
    let octave = extractor.extract(&shifted).expect("extract shifted");

    assert_eq!(base.pitch_contour.len(), octave.pitch_contour.len());
    let max_diff = max_abs_difference(&base.pitch_contour, &octave.pitch_contour);
    assert!(
        max_diff < 0.15,
        "expected contour invariance under octave shift, max diff={max_diff}"
    );
}

#[test]
fn contour_detects_shape_difference_between_flat_and_rising() {
    let rising = glide_clip(220.0, 440.0);
    let flat = glide_clip(220.0, 220.0);

    let extractor = FeatureExtractor::new();
    let rising_features = extractor.extract(&rising).expect("extract rising");
    let flat_features = extractor.extract(&flat).expect("extract flat");

    let mad = mean_abs_difference(&rising_features.pitch_contour, &flat_features.pitch_contour);
    assert!(
        mad > 0.35,
        "expected distinct contours between rising and flat, mad={mad}"
    );
}

fn glide_clip(f_start: f32, f_end: f32) -> RecordedClip {
    let total_samples = (SAMPLE_RATE as f32 * DURATION_SECONDS) as usize;
    let mut samples = Vec::with_capacity(total_samples);
    let dt = 1.0 / SAMPLE_RATE as f32;
    let mut phase = 0.0;

    for index in 0..total_samples {
        let progress = index as f32 / (total_samples - 1).max(1) as f32;
        let freq = f_start + (f_end - f_start) * progress;
        phase += 2.0 * std::f32::consts::PI * freq * dt;
        samples.push((phase).sin() * 0.4);
    }

    RecordedClip {
        samples: Arc::from(samples),
        sample_rate: SAMPLE_RATE,
        channels: 1,
        duration: Duration::from_secs_f32(DURATION_SECONDS),
    }
}

fn max_abs_difference(lhs: &Array1<f32>, rhs: &Array1<f32>) -> f32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max)
}

fn mean_abs_difference(lhs: &Array1<f32>, rhs: &Array1<f32>) -> f32 {
    let total = lhs
        .iter()
        .zip(rhs.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>();
    if lhs.is_empty() {
        0.0
    } else {
        total / lhs.len() as f32
    }
}
