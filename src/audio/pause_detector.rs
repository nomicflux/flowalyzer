use crate::types::{AudioData, FrameCount, FrameIndex};

/// Measure a pause threshold from the audio itself using integer frame windows.
/// Returns the minimum window energy observed; callers can adjust if desired.
pub fn calibrate_threshold(audio: &AudioData, window_frames: FrameCount) -> f32 {
    if audio.samples.is_empty() || window_frames.is_zero() {
        return 0.0;
    }
    let window = window_frames.as_usize().max(1);
    let mut idx = 0;
    let mut min_energy = f32::MAX;
    while idx < audio.samples.len() {
        let end = (idx + window).min(audio.samples.len());
        let energy = window_energy(&audio.samples[idx..end]);
        if energy < min_energy {
            min_energy = energy;
        }
        idx += window;
    }
    if min_energy.is_finite() {
        min_energy
    } else {
        0.0
    }
}

/// Detect pause midpoints (in frame indices) using frame-based windows.
///
/// All parameters are integer frame counts to align with the sample clock.
pub fn detect_pauses_frames(
    audio: &AudioData,
    window_frames: FrameCount,
    min_silence_frames: FrameCount,
    threshold: f32,
) -> Vec<FrameIndex> {
    if audio.samples.is_empty() || window_frames.is_zero() {
        return Vec::new();
    }

    let window_size = window_frames.as_usize().max(1);
    let min_silence_samples = min_silence_frames.as_usize().max(window_size);

    let mut window_energies = Vec::new();
    let mut idx = 0;
    while idx < audio.samples.len() {
        let end = (idx + window_size).min(audio.samples.len());
        let energy = window_energy(&audio.samples[idx..end]);
        window_energies.push((idx, energy));
        idx += window_size;
    }

    let (min_energy, max_energy) = window_energies
        .iter()
        .fold((f32::MAX, f32::MIN), |(min_e, max_e), (_, energy)| {
            (min_e.min(*energy), max_e.max(*energy))
        });
    if (max_energy - min_energy).abs() <= f32::EPSILON {
        // Flat energy profile: no pauses.
        return Vec::new();
    }

    let mut pauses = Vec::new();
    let mut silence_start: Option<usize> = None;

    for (start_idx, energy) in window_energies.into_iter() {
        if energy <= threshold {
            silence_start.get_or_insert(start_idx);
        } else if let Some(start) = silence_start {
            let silence_len = start_idx.saturating_sub(start);
            if silence_len >= min_silence_samples {
                let midpoint = start + silence_len / 2;
                pauses.push(FrameIndex::from(midpoint));
            }
            silence_start = None;
        }
    }

    if let Some(start) = silence_start {
        let silence_len = audio.samples.len().saturating_sub(start);
        if silence_len >= min_silence_samples {
            let midpoint = start + silence_len / 2;
            pauses.push(FrameIndex::from(midpoint));
        }
    }

    pauses
}

fn window_energy(window: &[f32]) -> f32 {
    if window.is_empty() {
        return 0.0;
    }
    let sum: f32 = window.iter().map(|sample| sample.abs()).sum();
    sum / window.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FrameRange, SampleRate};

    fn make_audio(samples: Vec<f32>, sample_rate: u32) -> AudioData {
        let length = FrameCount::from(samples.len());
        AudioData {
            frame_range: FrameRange::new(FrameIndex::ZERO, length),
            samples,
            sample_rate,
        }
    }

    #[test]
    fn detect_no_pauses_in_loud_signal() {
        let sample_rate = SampleRate::new(10_000).unwrap();
        let audio = make_audio(vec![0.8; 10_000], sample_rate.hz()); // 1 second loud
        let window = sample_rate.frames_from_seconds_round(0.02);
        let threshold = calibrate_threshold(&audio, window);
        let pauses = detect_pauses_frames(
            &audio,
            window,
            sample_rate.frames_from_seconds_round(0.08),
            threshold,
        );
        assert!(pauses.is_empty());
    }

    #[test]
    fn detect_single_pause() {
        // 0.5s loud, 0.2s quiet, 0.5s loud
        let mut samples = vec![0.8; 5_000];
        samples.extend(vec![0.01; 2_000]);
        samples.extend(vec![0.8; 5_000]);
        let sample_rate = SampleRate::new(10_000).unwrap();
        let audio = make_audio(samples, sample_rate.hz());

        let window = sample_rate.frames_from_seconds_round(0.02);
        let threshold = calibrate_threshold(&audio, window);
        let pauses = detect_pauses_frames(
            &audio,
            window,
            sample_rate.frames_from_seconds_round(0.08),
            threshold,
        );
        assert_eq!(pauses.len(), 1);
        let pause_time = sample_rate.seconds_from_frames(pauses[0].to_count());
        assert!((pause_time - 0.6).abs() < 0.05); // roughly middle of quiet region
    }

    #[test]
    fn short_silence_ignored() {
        // 0.5s loud, 0.04s quiet, 0.5s loud
        let mut samples = vec![0.8; 5_000];
        samples.extend(vec![0.01; 400]);
        samples.extend(vec![0.8; 5_000]);
        let sample_rate = SampleRate::new(10_000).unwrap();
        let audio = make_audio(samples, sample_rate.hz());

        let window = sample_rate.frames_from_seconds_round(0.02);
        let threshold = calibrate_threshold(&audio, window);
        let pauses = detect_pauses_frames(
            &audio,
            window,
            sample_rate.frames_from_seconds_round(0.08),
            threshold,
        );
        assert!(pauses.is_empty());
    }
}
