use aus::analysis;
use ndarray::Array1;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

use crate::audio::resample;
use crate::pronunciation::{PronunciationError, RecordedClip, Result};

use super::mel::{TARGET_SAMPLE_RATE, WINDOW_MS};
use super::FeatureExtractionEvent;
use super::FeatureExtractionPhase;

const FREQ_MIN: f64 = 55.0;
const FREQ_MAX: f64 = 1200.0;
const SMOOTH_WINDOW: usize = 5;
const HALF_SECOND_SAMPLES: usize = (TARGET_SAMPLE_RATE as usize) / 2;

pub(super) fn extract_pitch_contour_with_reporting<F>(
    clip: &RecordedClip,
    frame_count: usize,
    reporter: &mut F,
    start_time: Instant,
) -> Result<Array1<f32>>
where
    F: FnMut(FeatureExtractionEvent),
{
    info!("ensuring sample rate for pitch extraction");
    let samples = ensure_sample_rate(clip)?;
    info!(
        elapsed_secs = start_time.elapsed().as_secs_f64(),
        samples = samples.len(),
        "sample rate ensured for pitch"
    );

    info!("converting to f64 for pitch");
    let audio: Vec<f64> = samples.into_iter().map(|s| s as f64).collect();
    let audio = Arc::new(audio);
    info!(
        elapsed_secs = start_time.elapsed().as_secs_f64(),
        "conversion to f64 complete for pitch"
    );

    let frame_len = frame_length_samples();

    info!(
        frame_len,
        audio_samples = audio.len(),
        freq_min = FREQ_MIN,
        freq_max = FREQ_MAX,
        "extracting pitch contour"
    );

    let pyin_start = Instant::now();
    let hop_samples = frame_len / 4;
    let expected_pitch_frames = if audio.len() >= frame_len {
        ((audio.len() - frame_len) / hop_samples) + 1
    } else {
        0
    };

    let chunk_size_samples = HALF_SECOND_SAMPLES.max(frame_len + hop_samples);
    let overlap_samples = frame_len;
    let mut all_pitches = Vec::new();
    let mut all_voiced = Vec::new();
    let mut total_processed_frames = 0u32;

    let step = chunk_size_samples
        .saturating_sub(overlap_samples)
        .max(hop_samples);
    let mut chunk_start = 0;
    while chunk_start < audio.len() {
        let chunk_end = (chunk_start + chunk_size_samples).min(audio.len());
        let chunk = &audio[chunk_start..chunk_end];
        if chunk.len() < frame_len {
            break;
        }
        // CRITICAL: Verify audio values being passed to pitch estimator
        let chunk_min = chunk.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let chunk_max = chunk.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let chunk_rms = (chunk.iter().map(|&s| s * s).sum::<f64>() / chunk.len() as f64).sqrt();
        let chunk_preview: Vec<f64> = chunk.iter().take(10).copied().collect();
        info!(
            chunk_start = chunk_start,
            chunk_len = chunk.len(),
            chunk_min = chunk_min,
            chunk_max = chunk_max,
            chunk_rms = chunk_rms,
            chunk_preview = ?chunk_preview,
            "VERIFY: Audio chunk being passed to pitch estimator"
        );

        let (_timestamps, pitches, voiced_flags, _confidence) = analysis::pyin_pitch_estimator(
            chunk,
            TARGET_SAMPLE_RATE,
            FREQ_MIN,
            FREQ_MAX,
            frame_len,
        );

        let chunk_frames = pitches.len();
        if chunk_start == 0 {
            all_pitches.extend_from_slice(&pitches);
            all_voiced.extend_from_slice(&voiced_flags);
            total_processed_frames = chunk_frames as u32;
        } else {
            let overlap_frames = (overlap_samples / hop_samples).min(chunk_frames);
            let skip = overlap_frames.min(chunk_frames);
            all_pitches.extend_from_slice(&pitches[skip..]);
            all_voiced.extend_from_slice(&voiced_flags[skip..]);
            total_processed_frames += (chunk_frames - skip) as u32;
        }

        reporter(FeatureExtractionEvent::PhaseProgress {
            phase: FeatureExtractionPhase::PitchContour,
            label: "Pitch frames",
            current: total_processed_frames,
            total: expected_pitch_frames as u32,
        });

        chunk_start = chunk_start.saturating_add(step);
    }

    let pitches = all_pitches;
    let voiced_flags = all_voiced;
    let pyin_elapsed = pyin_start.elapsed();
    let total_elapsed = start_time.elapsed();
    let elapsed_secs = total_elapsed.as_secs() as u32;
    reporter(FeatureExtractionEvent::PhaseElapsed {
        phase: FeatureExtractionPhase::PitchContour,
        elapsed_secs,
    });
    let voiced_count = voiced_flags.iter().filter(|&&v| v).count();
    let valid_pitches: Vec<f64> = pitches
        .iter()
        .zip(voiced_flags.iter())
        .filter_map(|(&p, &v)| (v && p.is_finite() && p > 0.0).then_some(p))
        .collect();
    let pitch_min = valid_pitches.iter().fold(f64::MAX, |a, &b| a.min(b));
    let pitch_max = valid_pitches.iter().fold(0.0f64, |a, &b| a.max(b));
    let pitch_mean = if !valid_pitches.is_empty() {
        valid_pitches.iter().sum::<f64>() / valid_pitches.len() as f64
    } else {
        0.0
    };
    info!(
        elapsed_secs = pyin_elapsed.as_secs_f64(),
        pitch_frames = pitches.len(),
        voiced_frames = voiced_count,
        valid_pitch_frames = valid_pitches.len(),
        raw_pitch_min = if pitch_min == f64::MAX {
            0.0
        } else {
            pitch_min
        },
        raw_pitch_max = pitch_max,
        raw_pitch_mean = pitch_mean,
        "pitch extraction completed"
    );

    info!("normalizing pitch contour");
    let contour = normalise_contour(&pitches, &voiced_flags);
    let normalized_valid: Vec<f32> = contour
        .iter()
        .filter_map(|&v| v)
        .filter(|&x| x != 0.0)
        .collect();
    let normalized_count = normalized_valid.len();
    let (normalized_min, normalized_max, normalized_mean) = if normalized_count > 0 {
        let min = normalized_valid.iter().fold(f32::MAX, |a, &b| a.min(b));
        let max = normalized_valid.iter().fold(0.0f32, |a, &b| a.max(b));
        let mean = normalized_valid.iter().sum::<f32>() / normalized_count as f32;
        (min, max, mean)
    } else {
        (0.0, 0.0, 0.0)
    };
    info!(
        normalized_valid_frames = normalized_count,
        normalized_min = normalized_min,
        normalized_max = normalized_max,
        normalized_mean = normalized_mean,
        "pitch contour normalized"
    );
    let elapsed_secs = start_time.elapsed().as_secs() as u32;
    reporter(FeatureExtractionEvent::PhaseElapsed {
        phase: FeatureExtractionPhase::PitchContour,
        elapsed_secs,
    });
    info!(
        elapsed_secs = start_time.elapsed().as_secs_f64(),
        "pitch contour normalized"
    );

    info!("filling missing pitch values");
    let total_frames = contour.len();
    let filled = fill_missing_with_reporting(&contour, reporter, total_frames, start_time);
    let elapsed_secs = start_time.elapsed().as_secs() as u32;
    reporter(FeatureExtractionEvent::PhaseElapsed {
        phase: FeatureExtractionPhase::PitchContour,
        elapsed_secs,
    });
    info!(
        elapsed_secs = start_time.elapsed().as_secs_f64(),
        "missing pitch values filled"
    );

    info!("smoothing pitch contour");
    let smoothed = smooth(&filled, SMOOTH_WINDOW);
    let elapsed_secs = start_time.elapsed().as_secs() as u32;
    reporter(FeatureExtractionEvent::PhaseElapsed {
        phase: FeatureExtractionPhase::PitchContour,
        elapsed_secs,
    });
    info!(
        elapsed_secs = start_time.elapsed().as_secs_f64(),
        "pitch contour smoothed"
    );

    info!("aligning pitch contour to frames");
    let aligned = align_to_frames(&smoothed, frame_count);
    let elapsed_secs = start_time.elapsed().as_secs() as u32;
    reporter(FeatureExtractionEvent::PhaseElapsed {
        phase: FeatureExtractionPhase::PitchContour,
        elapsed_secs,
    });
    info!(
        elapsed_secs = start_time.elapsed().as_secs_f64(),
        aligned_frames = aligned.len(),
        "pitch contour aligned"
    );

    Ok(Array1::from(aligned))
}

fn fill_missing_with_reporting<F>(
    values: &[Option<f32>],
    reporter: &mut F,
    total_frames: usize,
    start_time: Instant,
) -> Vec<f32>
where
    F: FnMut(FeatureExtractionEvent),
{
    const REPORT_INTERVAL: usize = 256;
    let mut filled =
        forward_fill_with_reporting(values, reporter, total_frames, REPORT_INTERVAL, start_time);
    backward_fill(&mut filled);
    filled.iter_mut().for_each(|v| {
        if v.is_nan() {
            *v = 0.0;
        }
    });
    filled
}

fn forward_fill_with_reporting<F>(
    values: &[Option<f32>],
    reporter: &mut F,
    total: usize,
    interval: usize,
    start_time: Instant,
) -> Vec<f32>
where
    F: FnMut(FeatureExtractionEvent),
{
    let mut filled = vec![f32::NAN; values.len()];
    let mut last = None;
    let mut last_elapsed_secs = 0u32;
    for (idx, value) in values.iter().enumerate() {
        if let Some(v) = value {
            filled[idx] = *v;
            last = Some(*v);
        } else if let Some(prev) = last {
            filled[idx] = prev;
        }
        if idx > 0 && idx % interval == 0 {
            reporter(FeatureExtractionEvent::PhaseProgress {
                phase: FeatureExtractionPhase::PitchContour,
                label: "Processing frames",
                current: idx as u32,
                total: total as u32,
            });
        }
        let current_elapsed_secs = start_time.elapsed().as_secs() as u32;
        if current_elapsed_secs > last_elapsed_secs {
            reporter(FeatureExtractionEvent::PhaseElapsed {
                phase: FeatureExtractionPhase::PitchContour,
                elapsed_secs: current_elapsed_secs,
            });
            last_elapsed_secs = current_elapsed_secs;
        }
    }
    filled
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
        let upper = upper.min(len - 1);
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
