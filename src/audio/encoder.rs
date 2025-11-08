use crate::types::AudioData;
use anyhow::{Context, Result};
use std::path::Path;

/// Encode AudioData to WAV format and write to file
pub fn encode_audio<P: AsRef<Path>>(audio: &AudioData, path: P) -> Result<()> {
    let path = path.as_ref();

    // Create WAV writer specification
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    // Create the WAV writer
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("Failed to create WAV file: {}", path.display()))?;

    // Write samples as i16
    for &sample in &audio.samples {
        // Clamp to [-1.0, 1.0] and scale to i16 range
        let clamped = sample.clamp(-1.0, 1.0);
        let i16_sample = (clamped * 32767.0) as i16;

        writer
            .write_sample(i16_sample)
            .context("Failed to write audio sample")?;
    }

    // Finalize the WAV file
    writer.finalize().context("Failed to finalize WAV file")?;

    Ok(())
}
