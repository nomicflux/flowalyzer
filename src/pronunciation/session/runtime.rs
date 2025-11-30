use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio::capture::{CaptureBuilder, CaptureConfig, CaptureSource, LiveCaptureBuilder};
use crate::audio::playback::duplicate_to_stereo;
use crate::audio::resample::linear_resample;
use crate::pronunciation::features::{FeatureConfig, FeatureExtractor};
use crate::pronunciation::session::{
    AlignmentReport, SessionConfig, SessionEngine, SessionSnapshot,
};
use crate::pronunciation::{PronunciationError, RecordedClip, Result};
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

thread_local! {
    static PLAYBACK_STREAM: RefCell<Option<OutputStream>> = const { RefCell::new(None) };
}

pub enum SessionCommand {
    Start,
    Stop,
    ReplayReference,
    StopReplay,
    Shutdown,
}

pub struct SessionRuntime {
    reference_clip: Arc<RecordedClip>,
    reference_playing: Arc<AtomicBool>,
    playback_state: Arc<Mutex<Option<PlaybackState>>>,
    engine: Arc<Mutex<SessionEngine>>,
    last_alignment: Arc<Mutex<Option<AlignmentReport>>>,
    config: SessionConfig,
    snapshot_sender: Sender<SessionSnapshot>,
    command_receiver: Receiver<SessionCommand>,
    capture_buffer: Arc<Mutex<Vec<f32>>>,
    resampled_buffer: Arc<Mutex<Vec<f32>>>,
    tail_seeded: Arc<AtomicBool>,
    capture_builder: Arc<dyn CaptureBuilder>,
}

impl SessionRuntime {
    fn create_engine(reference_samples: &[f32], sample_rate: u32) -> SessionEngine {
        let feature_cfg = FeatureConfig::from_sample_rate(sample_rate);
        let extractor = FeatureExtractor::new();
        let reference_features =
            extractor.extract_reference(reference_samples, sample_rate, feature_cfg);
        SessionEngine::new(reference_features, sample_rate)
    }

    pub fn spawn(
        reference_clip: RecordedClip,
        config: SessionConfig,
    ) -> (SessionHandle, SessionController) {
        Self::spawn_with_capture_builder(reference_clip, config, Arc::new(LiveCaptureBuilder))
    }

    pub fn spawn_with_capture_builder(
        reference_clip: RecordedClip,
        config: SessionConfig,
        capture_builder: Arc<dyn CaptureBuilder>,
    ) -> (SessionHandle, SessionController) {
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let reference_clip = Arc::new(reference_clip);
        let engine = Self::create_engine(&reference_clip.samples, config.sample_rate);
        let runtime = Self {
            reference_clip: reference_clip.clone(),
            reference_playing: Arc::new(AtomicBool::new(false)),
            playback_state: Arc::new(Mutex::new(None)),
            engine: Arc::new(Mutex::new(engine)),
            last_alignment: Arc::new(Mutex::new(None)),
            config: config.clone(),
            snapshot_sender: snapshot_tx,
            command_receiver: command_rx,
            capture_buffer: Arc::new(Mutex::new(Vec::new())),
            resampled_buffer: Arc::new(Mutex::new(Vec::new())),
            tail_seeded: Arc::new(AtomicBool::new(false)),
            capture_builder,
        };
        let controller = SessionController {
            command_sender: command_tx,
        };
        let thread_handle = thread::spawn(move || runtime.run());
        let initial_snapshot = snapshot_rx.recv().unwrap();
        let handle = SessionHandle {
            initial_snapshot,
            snapshot_receiver: snapshot_rx,
            config: config.clone(),
            join_handle: Arc::new(Mutex::new(Some(thread_handle))),
        };
        (handle, controller)
    }

    fn run(self) {
        let initial_alignment = AlignmentReport {
            reference_energy: Vec::new(),
            learner_energy: Vec::new(),
            energy_error: Vec::new(),
            reference_pitch: Vec::new(),
            learner_pitch: Vec::new(),
            similarity_band: Vec::new(),
            contour_band: Vec::new(),
            start_frame_idx: 0,
            end_frame_idx: 0,
            hop_ms: 0.0,
            global_time_offset_ms: 0.0,
            total_duration: 0.0,
        };
        let initial_snapshot = SessionSnapshot {
            alignment: initial_alignment,
            recording: false,
            reference_playing: false,
        };
        let _ = self.snapshot_sender.send(initial_snapshot);
        let mut recording = false;
        let mut capture: Option<Box<dyn CaptureSource>> = None;
        loop {
            match self.command_receiver.try_recv() {
                Ok(SessionCommand::Start) => {
                    recording = true;
                    if let Some(mut active) = capture.take() {
                        active.stop();
                    }
                    Self::clear_buffer(&self.capture_buffer);
                    Self::clear_buffer(&self.resampled_buffer);
                    self.tail_seeded.store(false, Ordering::SeqCst);
                    capture = Some(self.start_capture());
                    if let Ok(mut engine) = self.engine.lock() {
                        engine.reset();
                    }
                    if let Ok(mut last) = self.last_alignment.lock() {
                        *last = None;
                    }
                }
                Ok(SessionCommand::Stop) => {
                    recording = false;
                    if let Some(mut active) = capture.take() {
                        active.stop();
                    }
                    Self::clear_buffer(&self.capture_buffer);
                    Self::clear_buffer(&self.resampled_buffer);
                    self.tail_seeded.store(false, Ordering::SeqCst);
                    if let Some(snapshot) = self.snapshot_from_last(false) {
                        let _ = self.snapshot_sender.send(snapshot);
                    }
                }
                Ok(SessionCommand::Shutdown) => {
                    if let Some(mut active) = capture.take() {
                        active.stop();
                    }
                    break;
                }
                Ok(SessionCommand::ReplayReference) => {
                    self.start_reference_playback();
                }
                Ok(SessionCommand::StopReplay) => {
                    self.stop_reference_playback();
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => break,
            }

            if recording {
                if let Some(cap) = capture.as_mut() {
                    self.process_capture_chunk(cap.as_mut());
                }
            }

            self.poll_playback_completion();
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn start_capture(&self) -> Box<dyn CaptureSource> {
        let mut capture_config = CaptureConfig::new();
        capture_config.sample_rate = self
            .config
            .capture_sample_rate
            .unwrap_or(self.config.sample_rate);
        capture_config.latency_ms = self.config.latency_range.clone();
        self.capture_builder.start_capture(&capture_config).unwrap()
    }

    fn process_capture_chunk(&self, capture: &mut dyn CaptureSource) {
        let timeout = Duration::from_millis(0);
        let target_len =
            (self.config.sample_rate as usize * self.config.chunk_duration_ms as usize) / 1_000;
        let incoming_rate = capture.sample_rate();
        let mut buffer = self.capture_buffer.lock().unwrap();
        while let Some(chunk) = capture.recv_chunk(timeout) {
            buffer.extend_from_slice(&chunk);
        }

        if let Some((processed, remaining)) = collect_resampled_chunk_from_buffer(
            &buffer,
            target_len,
            incoming_rate,
            self.config.sample_rate,
        )
        .unwrap()
        {
            *buffer = remaining;
            let mut staging = self.resampled_buffer.lock().unwrap();
            staging.extend_from_slice(&processed);
            let required_tail_len = self.required_tail_len();
            while staging.len()
                >= if self.tail_seeded.load(Ordering::SeqCst) {
                    target_len
                } else {
                    required_tail_len + target_len
                }
            {
                if !self.tail_seeded.load(Ordering::SeqCst) {
                    let tail = staging[..required_tail_len].to_vec();
                    let chunk = staging[required_tail_len..required_tail_len + target_len].to_vec();
                    *staging = staging[required_tail_len + target_len..].to_vec();
                    if let Ok(mut engine) = self.engine.lock() {
                        engine.seed_tail(&tail);
                        let report = engine.process_chunk(&chunk);
                        if let Ok(mut last) = self.last_alignment.lock() {
                            *last = Some(report.clone());
                        }
                        let snapshot = self.snapshot_for(report, true);
                        let _ = self.snapshot_sender.send(snapshot);
                    }
                    self.tail_seeded.store(true, Ordering::SeqCst);
                } else {
                    let chunk = staging[..target_len].to_vec();
                    *staging = staging[target_len..].to_vec();
                    if let Ok(mut engine) = self.engine.lock() {
                        let report = engine.process_chunk(&chunk);
                        if let Ok(mut last) = self.last_alignment.lock() {
                            *last = Some(report.clone());
                        }
                        let snapshot = self.snapshot_for(report, true);
                        let _ = self.snapshot_sender.send(snapshot);
                    }
                }
            }
        }
    }

    fn snapshot_for(&self, alignment: AlignmentReport, recording: bool) -> SessionSnapshot {
        self.snapshot_internal(alignment, recording)
    }

    fn snapshot_from_last(&self, recording: bool) -> Option<SessionSnapshot> {
        let alignment = self
            .last_alignment
            .lock()
            .ok()
            .and_then(|guard| guard.clone())?;
        Some(self.snapshot_internal(alignment, recording))
    }

    fn required_tail_len(&self) -> usize {
        let cfg = FeatureConfig::from_sample_rate(self.config.sample_rate);
        cfg.frame_len_samples - cfg.hop_samples
    }

    fn snapshot_internal(&self, alignment: AlignmentReport, recording: bool) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            recording,
            reference_playing: self.reference_playing.load(Ordering::SeqCst),
        }
    }

    fn start_reference_playback(&self) {
        self.stop_reference_playback();
        let samples: Vec<f32> = self.reference_clip.samples.iter().copied().collect();
        let sample_rate = self.reference_clip.sample_rate;
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let stereo = duplicate_to_stereo(&samples);
                sink.append(SamplesBuffer::new(2, sample_rate, stereo));
                let sink_arc = Arc::new(sink);
                PLAYBACK_STREAM.with(|cell| {
                    *cell.borrow_mut() = Some(stream);
                });
                self.reference_playing.store(true, Ordering::SeqCst);
                *self.playback_state.lock().unwrap() = Some(PlaybackState { sink: sink_arc });
            }
        }
    }

    fn stop_reference_playback(&self) {
        if let Some(state) = self.playback_state.lock().unwrap().take() {
            state.sink.stop();
            self.reference_playing.store(false, Ordering::SeqCst);
            PLAYBACK_STREAM.with(|cell| {
                cell.borrow_mut().take();
            });
        }
    }

    fn poll_playback_completion(&self) {
        if !self.reference_playing.load(Ordering::SeqCst) {
            return;
        }
        let finished = {
            let guard = self.playback_state.lock().unwrap();
            guard
                .as_ref()
                .map(|state| state.sink.empty())
                .unwrap_or(false)
        };
        if finished {
            self.reference_playing.store(false, Ordering::SeqCst);
            self.playback_state.lock().unwrap().take();
            PLAYBACK_STREAM.with(|cell| {
                cell.borrow_mut().take();
            });
        }
    }

    fn clear_buffer(buffer: &Arc<Mutex<Vec<f32>>>) {
        if let Ok(mut guard) = buffer.lock() {
            guard.clear();
        }
    }
}

fn required_raw_samples(target_len: usize, input_rate: u32, output_rate: u32) -> usize {
    (target_len as u64 * input_rate as u64).div_ceil(output_rate as u64) as usize
}

#[allow(dead_code)]
fn collect_resampled_chunk<F>(
    next_chunk: &mut F,
    target_len: usize,
    input_rate: u32,
    output_rate: u32,
) -> std::result::Result<Option<Vec<f32>>, PronunciationError>
where
    F: FnMut() -> Option<Vec<f32>>,
{
    let mut raw_samples = Vec::new();
    let mut required = required_raw_samples(target_len, input_rate, output_rate);

    loop {
        while raw_samples.len() < required {
            match next_chunk() {
                Some(chunk) => raw_samples.extend_from_slice(&chunk),
                None => return Ok(None),
            }
        }

        let output = if input_rate == output_rate {
            raw_samples.clone()
        } else {
            linear_resample(&raw_samples, input_rate, output_rate)
                .map_err(|err| PronunciationError::new(err.to_string()))?
        };

        if output.len() >= target_len {
            let mut chunk = output;
            if chunk.len() > target_len {
                chunk.truncate(target_len);
            }
            return Ok(Some(chunk));
        }

        let shortfall = target_len - output.len();
        let extra = required_raw_samples(shortfall, input_rate, output_rate).max(1);
        required = raw_samples.len() + extra;
    }
}

type ChunkResult = std::result::Result<Option<(Vec<f32>, Vec<f32>)>, PronunciationError>;

fn collect_resampled_chunk_from_buffer(
    buffer: &[f32],
    target_len: usize,
    input_rate: u32,
    output_rate: u32,
) -> ChunkResult {
    if buffer.is_empty() {
        return Ok(None);
    }
    let required = required_raw_samples(target_len, input_rate, output_rate);
    if buffer.len() < required {
        return Ok(None);
    }
    let (to_process, remainder) = buffer.split_at(required);
    let resampled = if input_rate == output_rate {
        to_process.to_vec()
    } else {
        linear_resample(to_process, input_rate, output_rate)
            .map_err(|err| PronunciationError::new(err.to_string()))?
    };
    Ok(Some((resampled, remainder.to_vec())))
}

struct PlaybackState {
    sink: Arc<Sink>,
}

pub struct SessionHandle {
    initial_snapshot: SessionSnapshot,
    snapshot_receiver: Receiver<SessionSnapshot>,
    config: SessionConfig,
    join_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl SessionHandle {
    pub fn drain_snapshots(&self) -> Vec<SessionSnapshot> {
        let mut snapshots = Vec::new();
        while let Ok(snapshot) = self.snapshot_receiver.try_recv() {
            snapshots.push(snapshot);
        }
        snapshots
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    pub fn initial_snapshot(&self) -> &SessionSnapshot {
        &self.initial_snapshot
    }

    pub fn join(&self) -> thread::Result<()> {
        self.join_handle
            .lock()
            .ok()
            .and_then(|mut guard| guard.take())
            .unwrap()
            .join()
    }
}

pub struct SessionController {
    command_sender: Sender<SessionCommand>,
}

impl SessionController {
    pub fn start(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Start)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn stop(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Stop)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn replay_reference(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::ReplayReference)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn stop_replay(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::StopReplay)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn shutdown(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Shutdown)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use crate::audio::resample::linear_resample;
    use crate::pronunciation::features::FeatureConfig;
    use crate::pronunciation::session::SessionConfig;

    use super::{
        collect_resampled_chunk, collect_resampled_chunk_from_buffer, required_raw_samples,
        SessionRuntime,
    };

    #[test]
    fn required_samples_scale_with_input_rate() {
        let target_len = 1_600; // 100ms at 16kHz
        let required = required_raw_samples(target_len, 44_100, 16_000);
        assert_eq!(required, 4_410);
    }

    #[test]
    fn resampled_chunk_meets_target_length() {
        let target_len = 1_600;
        let mut remaining_chunks = vec![vec![0.0_f32; 2_205]; 2].into_iter();
        let mut provider = || remaining_chunks.next();
        let resampled = collect_resampled_chunk(&mut provider, target_len, 44_100, 16_000).unwrap();
        let samples = resampled.expect("expected samples");
        assert!(
            samples.len() >= target_len,
            "resampled chunk should meet or exceed target length"
        );
    }

    #[test]
    fn returns_none_when_no_chunks_available() {
        let mut provider = || -> Option<Vec<f32>> { None };
        let resampled = collect_resampled_chunk(&mut provider, 1_000, 16_000, 16_000).unwrap();
        assert!(resampled.is_none());
    }

    #[test]
    fn same_rate_chunks_meet_target_length() {
        let target_len = 1_600;
        let mut remaining_chunks = vec![vec![0.0_f32; target_len / 2]; 2].into_iter();
        let mut provider = || remaining_chunks.next();
        let resampled = collect_resampled_chunk(&mut provider, target_len, 16_000, 16_000).unwrap();
        let samples = resampled.expect("expected samples");
        assert!(
            samples.len() >= target_len,
            "same-rate chunk should meet or exceed target length"
        );
    }

    #[test]
    fn buffered_collection_emits_and_retain_remainder() {
        let target_len = 1_600;
        let input_rate = 48_000;
        let output_rate = 16_000;
        let required = required_raw_samples(target_len, input_rate, output_rate);
        let buffer = vec![0.1_f32; required + 100];
        let result =
            collect_resampled_chunk_from_buffer(&buffer, target_len, input_rate, output_rate)
                .expect("should resample");
        let (processed, remainder) = result.expect("expected a processed chunk");
        assert!(
            processed.len() >= target_len,
            "processed chunk should meet target length"
        );
        assert_eq!(
            remainder.len(),
            100,
            "should retain unconsumed samples as remainder"
        );
    }

    #[test]
    fn capture_pipeline_produces_voiced_pitch() {
        fn sine_wave(sample_rate: u32, frequency: f32, duration_secs: f32) -> Vec<f32> {
            let total_samples = (sample_rate as f32 * duration_secs) as usize;
            (0..total_samples)
                .map(|i| {
                    (2.0 * PI * frequency * i as f32 / sample_rate as f32)
                        .sin()
                        .clamp(-1.0, 1.0)
                })
                .collect()
        }

        let config = SessionConfig {
            chunk_duration_ms: 150,
            ..Default::default()
        };
        let feature_cfg = FeatureConfig::from_sample_rate(config.sample_rate);
        let required_tail_len = feature_cfg.frame_len_samples - feature_cfg.hop_samples;
        let reference = sine_wave(config.sample_rate, 220.0, 2.0);
        let mut engine = SessionRuntime::create_engine(&reference, config.sample_rate);

        let device_rate = 48_000;
        let capture_signal = sine_wave(device_rate, 220.0, 2.0);
        let device_chunk = 480; // ~10ms at 48kHz
        let target_len = (config.sample_rate as usize * config.chunk_duration_ms as usize) / 1_000;

        let raw_tail_len = ((required_tail_len as f32 * device_rate as f32
            / config.sample_rate as f32)
            .ceil()) as usize;
        let raw_tail = &capture_signal[..raw_tail_len];
        let resampled_tail = linear_resample(raw_tail, device_rate, config.sample_rate).unwrap();
        let tail_for_engine = &resampled_tail[..required_tail_len];
        engine.seed_tail(tail_for_engine);

        let mut device_buffer = Vec::new();
        let mut emitted_voiced_frames = 0usize;
        let mut index = raw_tail_len;
        while index < capture_signal.len() {
            let end = (index + device_chunk).min(capture_signal.len());
            device_buffer.extend_from_slice(&capture_signal[index..end]);
            index = end;

            loop {
                match collect_resampled_chunk_from_buffer(
                    &device_buffer,
                    target_len,
                    device_rate,
                    config.sample_rate,
                ) {
                    Ok(Some((chunk, remainder))) => {
                        device_buffer = remainder;
                        let report = engine.process_chunk(&chunk);
                        emitted_voiced_frames += report
                            .learner_pitch
                            .iter()
                            .filter(|value| **value > 0.0)
                            .count();
                    }
                    Ok(None) => break,
                    Err(err) => panic!("resample error: {}", err),
                }
            }
        }

        assert!(
            emitted_voiced_frames > 0,
            "capture pipeline should yield voiced pitch frames"
        );
    }
}
