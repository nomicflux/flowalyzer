use crate::types::AudioData;
use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decode an audio file to raw PCM samples (mono, f32)
pub fn decode_audio<P: AsRef<Path>>(path: P) -> Result<AudioData> {
    let path = path.as_ref();

    // Open the file
    let file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open audio file: {}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create hint from file extension
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }

    // Probe the media source
    let probe_result = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Failed to probe audio format")?;

    let mut format = probe_result.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio tracks found in file")?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("Sample rate not specified in audio file")?;

    // Create a decoder for the track
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    // Collect all decoded samples
    let mut all_samples = Vec::new();

    // Decode all packets
    loop {
        // Get the next packet
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err))
                if err.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of stream
                break;
            }
            Err(err) => return Err(err).context("Failed to read packet"),
        };

        // Only process packets for our selected track
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet
        let decoded = decoder
            .decode(&packet)
            .context("Failed to decode audio packet")?;

        // Convert to f32 mono samples
        let mono_samples = convert_to_mono_f32(&decoded);
        all_samples.extend(mono_samples);
    }

    Ok(AudioData {
        samples: all_samples,
        sample_rate,
    })
}

/// Convert any audio buffer format to mono f32 samples in [-1.0, 1.0]
fn convert_to_mono_f32(buffer: &AudioBufferRef) -> Vec<f32> {
    // Get the number of channels and frames
    let spec = buffer.spec();
    let num_channels = spec.channels.count();
    let duration = buffer.frames();

    let mut mono_samples = Vec::with_capacity(duration);

    // Convert based on the actual sample format
    match buffer {
        AudioBufferRef::S8(buf) => {
            // Convert i8 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 128.0;
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale);
                }
            }
        }
        AudioBufferRef::F32(buf) => {
            // Already f32, just mix to mono
            if num_channels == 1 {
                // Already mono
                mono_samples.extend_from_slice(buf.chan(0));
            } else {
                // Mix all channels to mono by averaging
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i];
                    }
                    mono_samples.push(sum / num_channels as f32);
                }
            }
        }
        AudioBufferRef::F64(buf) => {
            // Convert f64 to f32 and mix to mono
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i];
                    }
                    mono_samples.push((sum / num_channels as f64) as f32);
                }
            }
        }
        AudioBufferRef::S16(buf) => {
            // Convert i16 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 32768.0;
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale);
                }
            }
        }
        AudioBufferRef::S24(buf) => {
            // Convert i24 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 8388608.0; // 2^23
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s.inner() as f32 * scale));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i].inner() as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale);
                }
            }
        }
        AudioBufferRef::S32(buf) => {
            // Convert i32 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 2147483648.0; // 2^31
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale);
                }
            }
        }
        AudioBufferRef::U8(buf) => {
            // Convert u8 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 128.0;
            let offset = -1.0;
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale + offset));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale + offset);
                }
            }
        }
        AudioBufferRef::U16(buf) => {
            // Convert u16 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 32768.0;
            let offset = -1.0;
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale + offset));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale + offset);
                }
            }
        }
        AudioBufferRef::U24(buf) => {
            // Convert u24 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 8388608.0;
            let offset = -1.0;
            if num_channels == 1 {
                mono_samples.extend(
                    buf.chan(0)
                        .iter()
                        .map(|&s| s.inner() as f32 * scale + offset),
                );
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i].inner() as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale + offset);
                }
            }
        }
        AudioBufferRef::U32(buf) => {
            // Convert u32 to f32 (normalize to [-1.0, 1.0])
            let scale = 1.0 / 2147483648.0;
            let offset = -1.0;
            if num_channels == 1 {
                mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale + offset));
            } else {
                for i in 0..duration {
                    let mut sum = 0.0;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[i] as f32;
                    }
                    mono_samples.push((sum / num_channels as f32) * scale + offset);
                }
            }
        }
    }

    mono_samples
}
