use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use tracing::info;

use crate::audio::resample;
use crate::types::AudioData;

const DEFAULT_SAMPLE_RATE: u32 = 16_000;

#[derive(Clone, Debug)]
pub struct CaptureConfig {
    pub device_name: Option<String>,
    pub sample_rate: u32,
    pub latency_ms: RangeInclusive<u32>,
    pub duration: Duration,
}

impl CaptureConfig {
    pub fn new(duration: Duration) -> Self {
        Self {
            device_name: None,
            sample_rate: DEFAULT_SAMPLE_RATE,
            latency_ms: default_latency_range(),
            duration,
        }
    }
}

fn default_latency_range() -> RangeInclusive<u32> {
    100..=200
}

struct StreamSetup {
    stream: Stream,
    receiver: Receiver<Vec<f32>>,
    finished: Arc<AtomicBool>,
    sample_rate: u32,
}

pub struct LiveCapture {
    stream: Stream,
    receiver: Receiver<Vec<f32>>,
    finished: Arc<AtomicBool>,
    sample_rate: u32,
}

pub fn record_audio(config: &CaptureConfig) -> Result<AudioData> {
    let device = select_device(config)?;
    let setup = build_stream(&device, config)?;
    let frames_needed = frames_for_duration(config.duration, setup.sample_rate);
    let raw = collect_samples(
        setup.stream,
        setup.receiver,
        setup.finished.clone(),
        frames_needed,
    )?;
    setup.finished.store(true, Ordering::SeqCst);
    let mono = if setup.sample_rate == config.sample_rate {
        raw
    } else {
        resample::linear_resample(&raw, setup.sample_rate, config.sample_rate)?
    };
    Ok(AudioData {
        samples: mono,
        sample_rate: config.sample_rate,
    })
}

fn select_device(config: &CaptureConfig) -> Result<Device> {
    let host = cpal::default_host();
    let devices: Vec<_> = host
        .input_devices()
        .context("listing input devices failed")?
        .collect();
    let device_names: Vec<String> = devices.iter().filter_map(|d| d.name().ok()).collect();
    info!(
        available_devices = ?device_names,
        requested_device = ?config.device_name,
        "listing available input devices"
    );
    if let Some(name) = config.device_name.as_deref() {
        for device in devices {
            if device.name().map(|n| n == name).unwrap_or(false) {
                let supported = device.default_input_config().ok();
                info!(
                    device_name = ?device.name().ok(),
                    sample_rate = ?supported.as_ref().map(|s| s.sample_rate().0),
                    channels = supported.as_ref().map(|s| s.channels()),
                    format = ?supported.as_ref().map(|s| s.sample_format()),
                    "selected capture device"
                );
                return Ok(device);
            }
        }
        return bail_device(name);
    }
    let selected = host
        .default_input_device()
        .context("no default input device available")?;
    let supported = selected.default_input_config().ok();
    info!(
        device_name = ?selected.name().ok(),
        sample_rate = ?supported.as_ref().map(|s| s.sample_rate().0),
        channels = supported.as_ref().map(|s| s.channels()),
        format = ?supported.as_ref().map(|s| s.sample_format()),
        "selected capture device (default)"
    );
    Ok(selected)
}

fn bail_device(name: &str) -> Result<Device> {
    Err(anyhow!("input device '{}' not found", name))
}

fn build_stream(device: &Device, config: &CaptureConfig) -> Result<StreamSetup> {
    let supported = device
        .default_input_config()
        .context("failed to query default input config")?;
    let stream_config = supported.config();
    info!(
        device_name = ?device.name().ok(),
        channels = stream_config.channels,
        sample_rate = stream_config.sample_rate.0,
        format = ?supported.sample_format(),
        buffer_size = ?stream_config.buffer_size,
        is_input = true,
        "configured capture device"
    );
    let capacity = channel_capacity(stream_config.sample_rate.0, &config.latency_ms);
    let (sender, receiver) = mpsc::sync_channel::<Vec<f32>>(capacity);
    let finished = Arc::new(AtomicBool::new(false));
    let stream = build_input_stream(
        device,
        &stream_config,
        supported.sample_format(),
        Arc::new(sender),
        finished.clone(),
    )?;
    Ok(StreamSetup {
        stream,
        receiver,
        finished,
        sample_rate: stream_config.sample_rate.0,
    })
}

fn build_input_stream(
    device: &Device,
    config: &StreamConfig,
    format: SampleFormat,
    sender: Arc<SyncSender<Vec<f32>>>,
    finished: Arc<AtomicBool>,
) -> Result<Stream> {
    let err_fn = |err| eprintln!("audio input stream error: {}", err);
    let channels = config.channels as usize;
    match format {
        SampleFormat::F32 => device.build_input_stream(
            config,
            {
                let sender = sender.clone();
                let finished = finished.clone();
                move |data: &[f32], _| emit_chunk_f32(data, channels, &sender, &finished)
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            {
                let sender = sender.clone();
                let finished = finished.clone();
                move |data: &[i16], _| emit_chunk_i16(data, channels, &sender, &finished)
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            {
                let sender = sender.clone();
                let finished = finished.clone();
                move |data: &[u16], _| emit_chunk_u16(data, channels, &sender, &finished)
            },
            err_fn,
            None,
        ),
        other => return Err(anyhow!("unsupported input sample format {:?}", other)),
    }
    .map_err(|err| anyhow!(err))
    .context("failed to build input stream")
}

impl LiveCapture {
    pub fn start(config: &CaptureConfig) -> Result<Self> {
        let setup = start_streaming_capture(config)?;
        Ok(Self {
            stream: setup.stream,
            receiver: setup.receiver,
            finished: setup.finished,
            sample_rate: setup.sample_rate,
        })
    }

    pub fn recv_chunk(&self, timeout: Duration) -> Option<Vec<f32>> {
        match self.receiver.recv_timeout(timeout) {
            Ok(chunk) => Some(chunk),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => None,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn stop(&self) {
        self.finished.store(true, Ordering::SeqCst);
        let _ = self.stream.pause();
    }
}

impl Drop for LiveCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

fn start_streaming_capture(config: &CaptureConfig) -> Result<StreamSetup> {
    let device = select_device(config)?;
    let setup = build_stream(&device, config)?;
    setup
        .stream
        .play()
        .context("failed to start live capture stream")?;
    Ok(setup)
}

fn emit_chunk_f32(
    data: &[f32],
    channels: usize,
    sender: &Arc<SyncSender<Vec<f32>>>,
    finished: &Arc<AtomicBool>,
) {
    // Diagnostic logging for raw stream data - log actual sample values to verify what we're receiving
    if !data.is_empty() {
        let raw_min = data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let raw_max = data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let _raw_mean = data.iter().sum::<f32>() / data.len() as f32;
        let _raw_rms = (data.iter().map(|&x| x * x).sum::<f32>() / data.len() as f32).sqrt();
        // Log first few actual sample values to see what we're getting
        // Also check if values are in expected [-1.0, 1.0] range or if they're in a different range
        let _sample_preview: Vec<f32> = data.iter().take(10).copied().collect();
        let abs_max = raw_max.abs().max(raw_min.abs());
        let _expected_range = if abs_max > 1.0 {
            "OUTSIDE expected [-1.0, 1.0] range - may need normalization"
        } else if abs_max < 0.01 {
            "VERY LOW - microphone may not be capturing properly"
        } else {
            "within expected range"
        };
    }
    emit_from_slice(data, channels, sender, finished);
}

fn emit_chunk_i16(
    data: &[i16],
    channels: usize,
    sender: &Arc<SyncSender<Vec<f32>>>,
    finished: &Arc<AtomicBool>,
) {
    let mut converted = Vec::with_capacity(data.len());
    for &sample in data {
        converted.push(sample as f32 / i16::MAX as f32);
    }
    emit_from_slice(&converted, channels, sender, finished);
}

fn emit_chunk_u16(
    data: &[u16],
    channels: usize,
    sender: &Arc<SyncSender<Vec<f32>>>,
    finished: &Arc<AtomicBool>,
) {
    let mut converted = Vec::with_capacity(data.len());
    for &sample in data {
        let normalized = (sample as f32 / u16::MAX as f32) * 2.0 - 1.0;
        converted.push(normalized);
    }
    emit_from_slice(&converted, channels, sender, finished);
}

fn emit_from_slice(
    data: &[f32],
    channels: usize,
    sender: &Arc<SyncSender<Vec<f32>>>,
    finished: &Arc<AtomicBool>,
) {
    if finished.load(Ordering::Relaxed) || channels == 0 {
        return;
    }
    let mut mono = Vec::with_capacity(data.len() / channels);
    for frame in data.chunks(channels) {
        mono.push(mix_to_mono(frame));
    }
    // Use try_send to avoid blocking in the audio callback
    // If the channel is full or receiver is dropped, we skip this chunk
    // This prevents blocking the audio thread and allows clean shutdown
    if sender.try_send(mono).is_err() {
        // CRITICAL: Chunks are being dropped! This will cause audio to not match what user is saying
        // Channel full or receiver dropped - this is a serious problem
        static DROP_WARNED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !DROP_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            eprintln!("ERROR: Audio chunks are being dropped! Channel is full or receiver disconnected. Audio will not match user's speech.");
        }
    }
}

fn collect_samples(
    stream: Stream,
    receiver: Receiver<Vec<f32>>,
    finished: Arc<AtomicBool>,
    frames_needed: usize,
) -> Result<Vec<f32>> {
    stream.play().context("failed to start capture stream")?;
    let mut collected = Vec::with_capacity(frames_needed);
    while collected.len() < frames_needed {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(chunk) => {
                append_chunk(&mut collected, chunk, frames_needed);
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    finished.store(true, Ordering::SeqCst);
    stream.pause().ok();
    Ok(collected)
}

fn append_chunk(buffer: &mut Vec<f32>, mut chunk: Vec<f32>, frames_needed: usize) {
    if buffer.len() + chunk.len() <= frames_needed {
        buffer.extend_from_slice(&chunk);
    } else {
        let remaining = frames_needed.saturating_sub(buffer.len());
        chunk.truncate(remaining);
        buffer.extend_from_slice(&chunk);
    }
}

fn frames_for_duration(duration: Duration, sample_rate: u32) -> usize {
    let frames = duration.as_secs_f64() * sample_rate as f64;
    frames.ceil() as usize
}

fn channel_capacity(sample_rate: u32, latency_ms: &RangeInclusive<u32>) -> usize {
    let max_latency = (*latency_ms.end()).max(*latency_ms.start());
    // Estimate chunks per second: sample_rate / typical_chunk_size (512)
    let chunks_per_second = sample_rate as f64 / 512.0;
    // Capacity should hold at least latency_ms worth of chunks, plus some headroom
    let capacity = (chunks_per_second * max_latency as f64 / 1000.0 * 2.0).ceil() as usize;
    capacity.max(10) // Minimum 10 chunks to prevent drops
}

pub fn mix_to_mono(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    frame.iter().sum::<f32>() / frame.len() as f32
}

#[cfg(test)]
mod tests {
    use super::mix_to_mono;

    #[test]
    fn averages_samples_in_frame() {
        let frame = [0.8, 0.2];
        let mono = mix_to_mono(&frame);
        assert!((mono - 0.5).abs() < 1e-6);
    }
}
