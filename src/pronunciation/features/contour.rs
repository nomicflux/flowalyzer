use aus::analysis;
use ndarray::Array1;

use crate::audio::resample;
use crate::pronunciation::{PronunciationError, RecordedClip, Result};

use super::mel::{TARGET_SAMPLE_RATE, WINDOW_MS};

const FREQ_MIN: f64 = 55.0;
const FREQ_MAX: f64 = 1200.0;
const SMOOTH_WINDOW: usize = 5;

pub(super) fn extract_pitch_contour(
    clip: &RecordedClip,
    frame_count: usize,
) -> Result<Array1<f32>> {
    let samples = ensure_sample_rate(clip)?;
    let audio: Vec<f64> = samples.into_iter().map(|s| s as f64).collect();
    let frame_len = frame_length_samples();
    let (_timestamps, pitches, voiced_flags, _confidence) =
        analysis::pyin_pitch_estimator(&audio, TARGET_SAMPLE_RATE, FREQ_MIN, FREQ_MAX, frame_len);
    let contour = normalise_contour(&pitches, &voiced_flags);
    let filled = fill_missing(&contour);
    let smoothed = smooth(&filled, SMOOTH_WINDOW);
    let aligned = align_to_frames(&smoothed, frame_count);
    Ok(Array1::from(aligned))
}

fn ensure_sample_rate(clip: &RecordedClip) -> Result<Vec<f32>> {
    if clip.sample_rate == TARGET_SAMPLE_RATE {
        Ok(clip.samples.to_vec())
    } else {
        resample::linear_resample(clip.samples.as_ref(), clip.sample_rate, TARGET_SAMPLE_RATE)
            .map_err(|err| PronunciationError::new(err.to_string()))
    }
}

fn frame_length_samples() -> usize {
    ((TARGET_SAMPLE_RATE as usize * WINDOW_MS) / 1000).max(1)
}

fn normalise_contour(pitches: &[f64], voiced: &[bool]) -> Vec<Option<f32>> {
    let reference = match median_pitch(pitches, voiced) {
        Some(value) => value,
        None => return vec![Some(0.0); pitches.len()],
    };
    pitches
        .iter()
        .zip(voiced.iter())
        .map(|(&pitch, &flag)| {
            if flag && pitch.is_finite() && pitch > 0.0 {
                let ratio = (pitch / reference).max(f64::MIN_POSITIVE);
                Some((12.0 * ratio.log2()) as f32)
            } else {
                None
            }
        })
        .collect()
}

fn median_pitch(pitches: &[f64], voiced: &[bool]) -> Option<f64> {
    let mut values: Vec<f64> = pitches
        .iter()
        .zip(voiced.iter())
        .filter_map(|(&pitch, &flag)| (flag && pitch.is_finite() && pitch > 0.0).then_some(pitch))
        .collect();
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = values.len() / 2;
    Some(if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

fn fill_missing(values: &[Option<f32>]) -> Vec<f32> {
    let mut filled = forward_fill(values);
    backward_fill(&mut filled);
    filled.iter_mut().for_each(|v| {
        if v.is_nan() {
            *v = 0.0;
        }
    });
    filled
}

fn forward_fill(values: &[Option<f32>]) -> Vec<f32> {
    let mut filled = vec![f32::NAN; values.len()];
    let mut last = None;
    for (idx, value) in values.iter().enumerate() {
        if let Some(v) = value {
            filled[idx] = *v;
            last = Some(*v);
        } else if let Some(prev) = last {
            filled[idx] = prev;
        }
    }
    filled
}

fn backward_fill(values: &mut [f32]) {
    let mut next = None;
    for value in values.iter_mut().rev() {
        if value.is_nan() {
            if let Some(v) = next {
                *value = v;
            }
        } else {
            next = Some(*value);
        }
    }
}

fn smooth(values: &[f32], window: usize) -> Vec<f32> {
    if values.is_empty() || window < 2 {
        return values.to_vec();
    }
    let radius = window / 2;
    let mut smoothed = Vec::with_capacity(values.len());
    for idx in 0..values.len() {
        let start = idx.saturating_sub(radius);
        let end = (idx + radius + 1).min(values.len());
        let count = (end - start) as f32;
        let sum: f32 = values[start..end].iter().sum();
        smoothed.push(sum / count);
    }
    smoothed
}

fn align_to_frames(series: &[f32], frame_count: usize) -> Vec<f32> {
    match (frame_count, series.len()) {
        (0, _) => Vec::new(),
        (_, 0) => vec![0.0; frame_count],
        (count, len) if count == len => series.to_vec(),
        (count, len) => interpolate(series, count, len),
    }
}

fn interpolate(series: &[f32], frame_count: usize, len: usize) -> Vec<f32> {
    let mut aligned = Vec::with_capacity(frame_count);
    for frame in 0..frame_count {
        let denom = (frame_count - 1).max(1) as f32;
        let position = frame as f32 * (len - 1) as f32 / denom;
        let lower = position.floor() as usize;
        let upper = position.ceil() as usize;
        if lower == upper {
            aligned.push(series[lower]);
            continue;
        }
        let weight = position - lower as f32;
        let value = series[lower] * (1.0 - weight) + series[upper] * weight;
        aligned.push(value);
    }
    aligned
}
