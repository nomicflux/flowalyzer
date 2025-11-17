use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio::capture::{CaptureConfig, LiveCapture};
use crate::audio::playback::duplicate_to_stereo;
use crate::pronunciation::session::{
    AlignmentReport, ClipVariant, PronunciationScores, SessionConfig, SessionEngine,
    SessionSnapshot,
};
use crate::pronunciation::{apply_recipe_to_range, PronunciationError, RecordedClip, Result};
use crate::types::Recipe;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

thread_local! {
    static PLAYBACK_STREAM: RefCell<Option<OutputStream>> = const { RefCell::new(None) };
}

pub enum SessionCommand {
    Start,
    Stop,
    ReplayReference,
    StopReplay,
    ApplyRecipe {
        start_sec: f64,
        end_sec: f64,
        recipe: Recipe,
    },
    ToggleClipVariant(ClipVariant),
    Shutdown,
}

pub struct SessionRuntime {
    reference_clip: Arc<RecordedClip>,
    flowalyzed_clip: Arc<Mutex<Option<RecordedClip>>>,
    active_variant: Arc<Mutex<ClipVariant>>,
    reference_playing: Arc<AtomicBool>,
    playback_state: Arc<Mutex<Option<PlaybackState>>>,
    engine: Arc<Mutex<SessionEngine>>,
    config: SessionConfig,
    snapshot_sender: Sender<SessionSnapshot>,
    command_receiver: Receiver<SessionCommand>,
}

impl SessionRuntime {
    pub fn spawn(
        reference_clip: RecordedClip,
        config: SessionConfig,
    ) -> (SessionHandle, SessionController) {
        assert!(
            !reference_clip.samples.is_empty(),
            "reference clip cannot be empty"
        );
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let reference_clip = Arc::new(reference_clip);
        let engine = SessionEngine::new(&reference_clip.samples, &config);
        let runtime = Self {
            reference_clip: reference_clip.clone(),
            flowalyzed_clip: Arc::new(Mutex::new(None)),
            active_variant: Arc::new(Mutex::new(ClipVariant::Original)),
            reference_playing: Arc::new(AtomicBool::new(false)),
            playback_state: Arc::new(Mutex::new(None)),
            engine: Arc::new(Mutex::new(engine)),
            config: config.clone(),
            snapshot_sender: snapshot_tx,
            command_receiver: command_rx,
        };
        let handle = SessionHandle {
            snapshot_receiver: snapshot_rx,
            config: config.clone(),
        };
        let controller = SessionController {
            command_sender: command_tx,
        };
        runtime
            .snapshot_sender
            .send(runtime.build_snapshot(PronunciationScores::default()))
            .ok();
        thread::spawn(move || runtime.run());
        (handle, controller)
    }

    fn run(self) {
        let mut recording = false;
        let mut capture: Option<LiveCapture> = None;
        loop {
            match self.command_receiver.try_recv() {
                Ok(SessionCommand::Start) => {
                    recording = true;
                    capture = self.start_capture().ok();
                    if let Ok(mut engine) = self.engine.lock() {
                        engine.reset();
                    }
                }
                Ok(SessionCommand::Stop) => {
                    recording = false;
                    capture = None;
                }
                Ok(SessionCommand::Shutdown) => break,
                Ok(SessionCommand::ReplayReference) => {
                    self.start_reference_playback();
                }
                Ok(SessionCommand::StopReplay) => {
                    self.stop_reference_playback();
                }
                Ok(SessionCommand::ApplyRecipe {
                    start_sec,
                    end_sec,
                    recipe,
                }) => self.handle_apply_recipe(start_sec, end_sec, recipe),
                Ok(SessionCommand::ToggleClipVariant(variant)) => {
                    self.handle_toggle_variant(variant)
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => break,
            }

            if recording {
                if let Some(ref cap) = capture {
                    self.process_capture_chunk(cap);
                }
            }

            self.poll_playback_completion();
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn start_capture(&self) -> Result<LiveCapture> {
        let mut capture_config = CaptureConfig::new(Duration::from_secs(3600));
        capture_config.sample_rate = self.config.sample_rate;
        capture_config.latency_ms = self.config.latency_range.clone();
        LiveCapture::start(&capture_config).map_err(|err| PronunciationError::new(err.to_string()))
    }

    fn process_capture_chunk(&self, capture: &LiveCapture) {
        let timeout = Duration::from_millis(self.config.chunk_duration_ms as u64);
        if let Some(samples) = capture.recv_chunk(timeout) {
            if let Ok(mut engine) = self.engine.lock() {
                let report = engine.process_chunk(&samples);
                let snapshot = self.snapshot_for(report, true);
                let _ = self.snapshot_sender.send(snapshot);
            }
        }
    }

    fn build_snapshot(&self, scores: PronunciationScores) -> SessionSnapshot {
        SessionSnapshot {
            alignment: AlignmentReport::default(),
            scores,
            recording: false,
            reference_playing: self.reference_playing.load(Ordering::SeqCst),
            active_clip_variant: self.current_variant(),
            has_flowalyzed_clip: self.has_flowalyzed_clip(),
            recipe_state: None,
            error: None,
        }
    }

    fn snapshot_for(&self, alignment: AlignmentReport, recording: bool) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            scores: PronunciationScores::default(),
            recording,
            reference_playing: self.reference_playing.load(Ordering::SeqCst),
            active_clip_variant: self.current_variant(),
            has_flowalyzed_clip: self.has_flowalyzed_clip(),
            recipe_state: None,
            error: None,
        }
    }

    fn start_reference_playback(&self) {
        self.stop_reference_playback();
        let samples: Vec<f32> = self.reference_clip.samples.iter().copied().collect();
        if samples.is_empty() {
            return;
        }
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
                let _ = self
                    .snapshot_sender
                    .send(self.build_snapshot(PronunciationScores::default()));
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
            let _ = self
                .snapshot_sender
                .send(self.build_snapshot(PronunciationScores::default()));
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
            let _ = self
                .snapshot_sender
                .send(self.build_snapshot(PronunciationScores::default()));
        }
    }

    fn handle_apply_recipe(&self, start_sec: f64, end_sec: f64, recipe: Recipe) {
        match apply_recipe_to_range(&self.reference_clip, start_sec, end_sec, &recipe) {
            Ok(flowalyzed) => {
                *self.flowalyzed_clip.lock().unwrap() = Some(flowalyzed);
                let mut snapshot = self.build_snapshot(PronunciationScores::default());
                snapshot.has_flowalyzed_clip = true;
                let _ = self.snapshot_sender.send(snapshot);
            }
            Err(err) => {
                let mut snapshot = self.build_snapshot(PronunciationScores::default());
                snapshot.error = Some(err.to_string());
                let _ = self.snapshot_sender.send(snapshot);
            }
        }
    }

    fn handle_toggle_variant(&self, variant: ClipVariant) {
        if variant == self.current_variant() {
            return;
        }
        self.stop_reference_playback();
        let clip = match variant {
            ClipVariant::Original => Some((*self.reference_clip).clone()),
            ClipVariant::Flowalyzed => self.flowalyzed_clip.lock().unwrap().clone(),
        };
        match clip {
            Some(clip) => {
                let engine = self.engine.clone();
                let config = self.config.clone();
                std::thread::spawn(move || {
                    if let Ok(mut engine) = engine.lock() {
                        *engine = SessionEngine::new(&clip.samples, &config);
                    }
                });
                if let Ok(mut active) = self.active_variant.lock() {
                    *active = variant;
                }
                let _ = self
                    .snapshot_sender
                    .send(self.build_snapshot(PronunciationScores::default()));
            }
            None => {
                let mut snapshot = self.build_snapshot(PronunciationScores::default());
                snapshot.error = Some("flowalyzed clip not available".to_string());
                let _ = self.snapshot_sender.send(snapshot);
            }
        }
    }

    fn current_variant(&self) -> ClipVariant {
        *self.active_variant.lock().unwrap()
    }

    fn has_flowalyzed_clip(&self) -> bool {
        self.flowalyzed_clip.lock().unwrap().is_some()
    }
}

struct PlaybackState {
    sink: Arc<Sink>,
}

pub struct SessionHandle {
    snapshot_receiver: Receiver<SessionSnapshot>,
    config: SessionConfig,
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

    pub fn initial_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot::default()
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

    pub fn apply_recipe(&self, start_sec: f64, end_sec: f64, recipe: Recipe) -> Result<()> {
        self.command_sender
            .send(SessionCommand::ApplyRecipe {
                start_sec,
                end_sec,
                recipe,
            })
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn toggle_clip_variant(&self, variant: ClipVariant) -> Result<()> {
        self.command_sender
            .send(SessionCommand::ToggleClipVariant(variant))
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn shutdown(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Shutdown)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }
}
