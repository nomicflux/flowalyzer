use crate::types::AudioData;

/// Detect pause timestamps (in seconds) based on windowed energy analysis.
///
/// # Parameters
/// * `audio` - mono PCM data
/// * `window_ms` - window size in milliseconds (e.g. 20.0)
/// * `min_silence_ms` - minimum consecutive silence needed to declare a pause (e.g. 80.0)
/// * `threshold` - amplitude threshold (linear 0.0-1.0 range) for silence detection
#[cfg_attr(not(test), allow(dead_code))]
pub fn detect_pauses(
    audio: &AudioData,
    window_ms: f64,
    min_silence_ms: f64,
    threshold: f32,
) -> Vec<f64> {
    if audio.samples.is_empty() {
        return Vec::new();
    }

    let sample_rate = audio.sample_rate;
    let window_size = ((window_ms / 1000.0) * sample_rate as f64).max(1.0) as usize;
    let min_silence_samples =
        ((min_silence_ms / 1000.0) * sample_rate as f64).max(window_size as f64) as usize;

    let mut window_energies = Vec::new();
    let mut idx = 0;
    while idx < audio.samples.len() {
        let end = (idx + window_size).min(audio.samples.len());
        let energy = window_energy(&audio.samples[idx..end]);
        window_energies.push((idx, energy));
        idx += window_size;
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
                pauses.push(midpoint as f64 / sample_rate as f64);
            }
            silence_start = None;
        }
    }

    if let Some(start) = silence_start {
        let silence_len = audio.samples.len().saturating_sub(start);
        if silence_len >= min_silence_samples {
            let midpoint = start + silence_len / 2;
            pauses.push(midpoint as f64 / sample_rate as f64);
        }
    }

    pauses
}

#[cfg_attr(not(test), allow(dead_code))]
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

    fn make_audio(samples: Vec<f32>, sample_rate: u32) -> AudioData {
        AudioData {
            samples,
            sample_rate,
        }
    }

    #[test]
    fn detect_no_pauses_in_loud_signal() {
        let audio = make_audio(vec![0.8; 10_000], 10_000); // 1 second loud
        let pauses = detect_pauses(&audio, 20.0, 80.0, 0.2);
        assert!(pauses.is_empty());
    }

    #[test]
    fn detect_single_pause() {
        // 0.5s loud, 0.2s quiet, 0.5s loud
        let mut samples = vec![0.8; 5_000];
        samples.extend(vec![0.01; 2_000]);
        samples.extend(vec![0.8; 5_000]);
        let audio = make_audio(samples, 10_000);

        let pauses = detect_pauses(&audio, 20.0, 80.0, 0.05);
        assert_eq!(pauses.len(), 1);
        let pause_time = pauses[0];
        assert!((pause_time - 0.6).abs() < 0.05); // roughly middle of quiet region
    }

    #[test]
    fn short_silence_ignored() {
        // 0.5s loud, 0.04s quiet, 0.5s loud
        let mut samples = vec![0.8; 5_000];
        samples.extend(vec![0.01; 400]);
        samples.extend(vec![0.8; 5_000]);
        let audio = make_audio(samples, 10_000);

        let pauses = detect_pauses(&audio, 20.0, 80.0, 0.05);
        assert!(pauses.is_empty());
    }
}
