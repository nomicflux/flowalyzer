use std::ops::RangeInclusive;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::audio::capture::{CaptureConfig, LiveCapture};
use crate::audio::playback::duplicate_to_stereo;
use crate::audio::resample;

use super::alignment::AudioAligner;
use super::features::FeatureExtractor;
use super::metrics::MetricCalculator;
use super::{
    load_clip, AlignmentReport, PronunciationError, PronunciationFeatures, PronunciationScores,
    RecordedClip, Result, SessionConfig, TARGET_SAMPLE_RATE,
};

const CAPTURE_POLL_MS: u64 = 20;
const MIN_SIGNAL_FRACTION: usize = 10;

#[derive(Clone)]
pub struct SessionController {
    tx: Sender<SessionCommand>,
}

pub struct SessionRuntime {
    config: SessionConfig,
    controller: SessionController,
    updates: Option<Receiver<SessionSnapshot>>,
    initial: SessionSnapshot,
    join: Option<JoinHandle<()>>,
}

pub struct SessionHandle {
    config: SessionConfig,
    controller: SessionController,
    updates: Receiver<SessionSnapshot>,
    initial: SessionSnapshot,
    join: Option<JoinHandle<()>>,
}

#[derive(Clone, Debug, Default)]
pub struct SessionSnapshot {
    pub alignment: AlignmentReport,
    pub scores: PronunciationScores,
    pub recording: bool,
    pub reference_playing: bool,
    pub latency_ms: f32,
    pub error: Option<String>,
}

impl SessionRuntime {
    pub fn new(config: SessionConfig) -> Result<Self> {
        let worker = Worker::prepare(&config)?;
        let (command_tx, command_rx) = channel();
        let (update_tx, update_rx) = channel();
        let join = worker.spawn(command_rx, update_tx)?;
        Ok(Self {
            config,
            controller: SessionController { tx: command_tx },
            updates: Some(update_rx),
            initial: SessionSnapshot::default(),
            join: Some(join),
        })
    }

    pub fn controller(&self) -> SessionController {
        self.controller.clone()
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    pub fn initial_snapshot(&self) -> SessionSnapshot {
        self.initial.clone()
    }

    pub fn try_recv(&self) -> Option<SessionSnapshot> {
        self.updates
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok())
    }

    pub fn into_handle(mut self) -> SessionHandle {
        let updates = self
            .updates
            .take()
            .expect("session updates channel already taken");
        SessionHandle {
            config: self.config.clone(),
            controller: self.controller.clone(),
            updates,
            initial: self.initial.clone(),
            join: self.join.take(),
        }
    }

    pub fn launch(self) -> Result<()> {
        if self.config.ui_enabled {
            crate::ui::launch_ui(self)
        } else {
            run_headless(self)
        }
    }
}

impl Drop for SessionRuntime {
    fn drop(&mut self) {
        let _ = self.controller.shutdown();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl SessionHandle {
    pub fn controller(&self) -> SessionController {
        self.controller.clone()
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    pub fn initial_snapshot(&self) -> SessionSnapshot {
        self.initial.clone()
    }

    pub fn try_recv(&self) -> Option<SessionSnapshot> {
        self.updates.try_recv().ok()
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        let _ = self.controller.shutdown();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl SessionSnapshot {
    pub fn with_recording(mut self, recording: bool, playing: bool) -> Self {
        self.recording = recording;
        self.reference_playing = playing;
        self
    }

    pub fn with_latency(mut self, latency_ms: f32, budget_ms: u32) -> Self {
        self.latency_ms = latency_ms;
        if latency_ms > budget_ms as f32 {
            self.error = Some(format!(
                "latency {:.1} ms exceeds budget {} ms",
                latency_ms, budget_ms
            ));
        } else if self.error.is_some() {
            self.error = None;
        }
        self
    }

    pub fn with_alignment(
        mut self,
        alignment: AlignmentReport,
        scores: PronunciationScores,
    ) -> Self {
        self.alignment = alignment;
        self.scores = scores;
        self
    }

    pub fn with_error_message(mut self, message: String) -> Self {
        self.error = Some(message);
        self
    }
}

impl SessionController {
    pub fn start(&self) -> Result<()> {
        self.send(SessionCommand::Start, "start session")
    }

    pub fn stop(&self) -> Result<()> {
        self.send(SessionCommand::Stop, "stop session")
    }

    pub fn shutdown(&self) -> Result<()> {
        self.send(SessionCommand::Shutdown, "shutdown session")
    }

    fn send(&self, command: SessionCommand, label: &str) -> Result<()> {
        self.tx
            .send(command)
            .map_err(|_| PronunciationError::new(format!("failed to {}", label)))
    }
}

fn run_headless(runtime: SessionRuntime) -> Result<()> {
    let controller = runtime.controller();
    controller.start()?;
    std::thread::sleep(Duration::from_secs(2));
    controller.stop()?;
    Ok(())
}

struct Worker {
    reference: RecordedClip,
    reference_features: PronunciationFeatures,
    aligner: AudioAligner,
    metrics: MetricCalculator,
    extractor: FeatureExtractor,
    capture: CaptureSettings,
    latency_budget_ms: u32,
    reference_samples: usize,
}

type CaptureSettings = super::CaptureSettings;

impl Worker {
    fn prepare(config: &SessionConfig) -> Result<Self> {
        let reference = load_clip(&config.reference_wav)?;
        let extractor = FeatureExtractor::new();
        let reference_features = extractor.extract(&reference)?;
        Ok(Self {
            reference_samples: reference.samples.len(),
            reference,
            reference_features,
            aligner: AudioAligner::new(config.alignment.clone()),
            metrics: MetricCalculator::new(),
            extractor,
            capture: config.capture.clone(),
            latency_budget_ms: config.latency_budget_ms,
        })
    }

    fn spawn(
        self,
        commands: Receiver<SessionCommand>,
        updates: Sender<SessionSnapshot>,
    ) -> Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("session-runtime".to_string())
            .spawn(move || self.run(commands, updates))
            .map_err(|err| PronunciationError::new(err.to_string()))
    }

    fn run(self, commands: Receiver<SessionCommand>, updates: Sender<SessionSnapshot>) {
        let mut snapshot = SessionSnapshot::default();
        let _ = updates.send(snapshot.clone());
        while let Ok(command) = commands.recv() {
            match command {
                SessionCommand::Start => {
                    match self.capture_loop(&commands, &updates, &mut snapshot) {
                        LoopExit::Finished => {}
                        LoopExit::Shutdown => break,
                    }
                }
                SessionCommand::Stop => {
                    snapshot = snapshot.with_recording(false, false);
                    let _ = updates.send(snapshot.clone());
                }
                SessionCommand::Shutdown => break,
            }
        }
    }

    fn capture_loop(
        &self,
        commands: &Receiver<SessionCommand>,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) -> LoopExit {
        match self.start_capture(snapshot, updates) {
            Ok(mut context) => self.drive_capture(commands, &mut context),
            Err(exit) => exit,
        }
    }

    fn start_capture<'a>(
        &'a self,
        snapshot: &'a mut SessionSnapshot,
        updates: &'a Sender<SessionSnapshot>,
    ) -> std::result::Result<CaptureContext<'a>, LoopExit> {
        let config = build_capture_config(&self.capture);
        let capture = match LiveCapture::start(&config) {
            Ok(stream) => stream,
            Err(err) => {
                emit_error(updates, snapshot, err.to_string());
                return Err(LoopExit::Finished);
            }
        };
        let mut player = match ReferencePlayer::new(&self.reference) {
            Ok(player) => player,
            Err(err) => {
                emit_error(updates, snapshot, err.to_string());
                return Err(LoopExit::Finished);
            }
        };
        if let Err(err) = player.play() {
            emit_error(updates, snapshot, err.to_string());
            return Err(LoopExit::Finished);
        }
        let mut context = CaptureContext::new(
            capture,
            player,
            snapshot,
            updates,
            self.reference_samples + TARGET_SAMPLE_RATE as usize / 2,
        );
        context.set_state(true, true);
        Ok(context)
    }

    fn drive_capture(
        &self,
        commands: &Receiver<SessionCommand>,
        context: &mut CaptureContext<'_>,
    ) -> LoopExit {
        loop {
            if let Some(exit) = self.handle_commands(commands, context) {
                return exit;
            }
            if let Some(exit) = self.consume_chunk(context) {
                return exit;
            }
        }
    }

    fn consume_chunk(&self, context: &mut CaptureContext<'_>) -> Option<LoopExit> {
        match context
            .capture
            .recv_chunk(Duration::from_millis(CAPTURE_POLL_MS))
        {
            Some(chunk) => match self.process_chunk(
                chunk,
                context.capture_rate,
                &mut context.buffer,
                context.max_samples,
            ) {
                Ok(Some(result)) => {
                    context.apply_alignment(result, self.latency_budget_ms);
                    None
                }
                Ok(None) => None,
                Err(err) => {
                    emit_error(context.updates, context.snapshot, err.to_string());
                    context.stop();
                    Some(LoopExit::Finished)
                }
            },
            None => None,
        }
    }

    fn handle_commands(
        &self,
        commands: &Receiver<SessionCommand>,
        context: &mut CaptureContext<'_>,
    ) -> Option<LoopExit> {
        match poll_command(commands) {
            Some(SessionCommand::Shutdown) => {
                context.stop();
                context.set_state(false, false);
                Some(LoopExit::Shutdown)
            }
            Some(SessionCommand::Stop) => {
                context.stop();
                context.set_state(false, false);
                Some(LoopExit::Finished)
            }
            Some(SessionCommand::Start) | None => None,
        }
    }

    fn process_chunk(
        &self,
        chunk: Vec<f32>,
        capture_rate: u32,
        buffer: &mut Vec<f32>,
        max_samples: usize,
    ) -> Result<Option<ChunkResult>> {
        let resampled = resample::linear_resample(&chunk, capture_rate, TARGET_SAMPLE_RATE)
            .map_err(|err| PronunciationError::new(err.to_string()))?;
        append_limited(buffer, &resampled, max_samples);
        if buffer.len() < min_required_samples() {
            return Ok(None);
        }
        let start = Instant::now();
        let clip = RecordedClip::from_samples(buffer.clone(), TARGET_SAMPLE_RATE);
        let features = self.extractor.extract(&clip)?;
        let alignment = self.aligner.align(&self.reference_features, &features)?;
        let scores = self.metrics.score(&alignment)?;
        let latency = start.elapsed().as_secs_f32() * 1000.0;
        Ok(Some(ChunkResult {
            alignment,
            scores,
            latency_ms: latency,
        }))
    }
}

enum LoopExit {
    Finished,
    Shutdown,
}

struct ChunkResult {
    alignment: AlignmentReport,
    scores: PronunciationScores,
    latency_ms: f32,
}

struct CaptureContext<'a> {
    capture: LiveCapture,
    player: ReferencePlayer,
    buffer: Vec<f32>,
    snapshot: &'a mut SessionSnapshot,
    updates: &'a Sender<SessionSnapshot>,
    capture_rate: u32,
    max_samples: usize,
}

impl<'a> CaptureContext<'a> {
    fn new(
        capture: LiveCapture,
        player: ReferencePlayer,
        snapshot: &'a mut SessionSnapshot,
        updates: &'a Sender<SessionSnapshot>,
        max_samples: usize,
    ) -> Self {
        Self {
            capture_rate: capture.sample_rate(),
            capture,
            player,
            buffer: Vec::new(),
            snapshot,
            updates,
            max_samples,
        }
    }

    fn set_state(&mut self, recording: bool, playing: bool) {
        let next = self.snapshot.clone().with_recording(recording, playing);
        let _ = self.updates.send(next.clone());
        *self.snapshot = next;
    }

    fn apply_alignment(&mut self, result: ChunkResult, budget_ms: u32) {
        let next = self
            .snapshot
            .clone()
            .with_alignment(result.alignment, result.scores)
            .with_recording(true, true)
            .with_latency(result.latency_ms, budget_ms);
        let _ = self.updates.send(next.clone());
        *self.snapshot = next;
    }

    fn stop(&mut self) {
        self.player.stop();
        self.capture.stop();
    }
}

fn build_capture_config(settings: &CaptureSettings) -> CaptureConfig {
    CaptureConfig {
        device_name: settings.device_name.clone(),
        sample_rate: settings.sample_rate,
        latency_ms: clone_range(&settings.latency_ms),
        duration: Duration::from_secs(0),
    }
}

fn clone_range(range: &RangeInclusive<u32>) -> RangeInclusive<u32> {
    *range.start()..=*range.end()
}

fn append_limited(buffer: &mut Vec<f32>, chunk: &[f32], max_samples: usize) {
    buffer.extend_from_slice(chunk);
    if buffer.len() > max_samples {
        let excess = buffer.len() - max_samples;
        buffer.drain(0..excess);
    }
}

fn min_required_samples() -> usize {
    TARGET_SAMPLE_RATE as usize / MIN_SIGNAL_FRACTION
}

fn poll_command(commands: &Receiver<SessionCommand>) -> Option<SessionCommand> {
    match commands.try_recv() {
        Ok(command) => Some(command),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => Some(SessionCommand::Shutdown),
    }
}

fn emit_error(updates: &Sender<SessionSnapshot>, snapshot: &mut SessionSnapshot, message: String) {
    let next = snapshot
        .clone()
        .with_recording(false, false)
        .with_error_message(message);
    let _ = updates.send(next.clone());
    *snapshot = next;
}

struct ReferencePlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    samples: Vec<f32>,
    sample_rate: u32,
    sink: Option<Sink>,
}

impl ReferencePlayer {
    fn new(clip: &RecordedClip) -> Result<Self> {
        let stereo = duplicate_to_stereo(&clip.samples);
        let (stream, handle) =
            OutputStream::try_default().map_err(|err| PronunciationError::new(err.to_string()))?;
        Ok(Self {
            samples: stereo,
            sample_rate: clip.sample_rate,
            _stream: stream,
            handle,
            sink: None,
        })
    }

    fn play(&mut self) -> Result<()> {
        let sink =
            Sink::try_new(&self.handle).map_err(|err| PronunciationError::new(err.to_string()))?;
        let buffer = SamplesBuffer::new(2, self.sample_rate, self.samples.clone());
        sink.append(buffer);
        sink.play();
        sink.set_volume(1.0);
        self.sink = Some(sink);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
    }
}

#[derive(Clone, Copy)]
enum SessionCommand {
    Start,
    Stop,
    Shutdown,
}
