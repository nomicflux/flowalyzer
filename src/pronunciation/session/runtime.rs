use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio::capture::{CaptureConfig, LiveCapture};
use crate::pronunciation::{PronunciationError, RecordedClip, Result};
use crate::pronunciation::session::{
    AlignmentReport, ClipVariant, PronunciationScores, SessionConfig, SessionEngine, SessionSnapshot,
};

pub enum SessionCommand {
    Start,
    Stop,
    ReplayReference,
    StopReplay,
    Shutdown,
}

pub struct SessionRuntime {
    engine: Arc<Mutex<SessionEngine>>,
    config: SessionConfig,
    snapshot_sender: Sender<SessionSnapshot>,
    command_receiver: Receiver<SessionCommand>,
}

impl SessionRuntime {
    pub fn spawn(reference_clip: RecordedClip, config: SessionConfig) -> (SessionHandle, SessionController) {
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let engine = SessionEngine::new(&reference_clip.samples, &config);
        let runtime = Self {
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
                    self.flush_pending();
                }
                Ok(SessionCommand::Shutdown) => break,
                Ok(SessionCommand::ReplayReference) => {
                    let _ = self.snapshot_sender.send(self.build_snapshot(PronunciationScores::default()));
                }
                Ok(SessionCommand::StopReplay) => {}
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => break,
            }

            if recording {
                if let Some(ref cap) = capture {
                    self.process_capture_chunk(cap);
                }
            }

            thread::sleep(Duration::from_millis(10));
        }
        self.flush_pending();
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
                if let Some(report) = engine.ingest_chunk(samples) {
                    let snapshot = self.snapshot_for(report, true);
                    let _ = self.snapshot_sender.send(snapshot);
                }
            }
        }
    }

    fn flush_pending(&self) {
        if let Ok(mut engine) = self.engine.lock() {
            if let Some(report) = engine.flush_pending() {
                let _ = self.snapshot_sender.send(self.snapshot_for(report, true));
            }
        }
    }

    fn build_snapshot(&self, scores: PronunciationScores) -> SessionSnapshot {
        SessionSnapshot {
            alignment: AlignmentReport::default(),
            scores,
            recording: false,
            reference_playing: false,
            active_clip_variant: ClipVariant::Original,
            has_flowalyzed_clip: false,
            recipe_state: None,
            error: None,
        }
    }

    fn snapshot_for(&self, alignment: AlignmentReport, recording: bool) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            scores: PronunciationScores::default(),
            recording,
            reference_playing: false,
            active_clip_variant: ClipVariant::Original,
            has_flowalyzed_clip: false,
            recipe_state: None,
            error: None,
        }
    }
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

    pub fn shutdown(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Shutdown)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }
}
