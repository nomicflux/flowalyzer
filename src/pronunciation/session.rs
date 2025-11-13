use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::RangeInclusive;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::audio::capture::CaptureConfig;
use crate::audio::playback::duplicate_to_stereo;
use tracing::{debug, error, info};

use super::alignment::AudioAligner;
use super::features::FeatureExtractor;
use super::metrics::MetricCalculator;
use super::validate_config;
use super::{
    load_clip, AlignmentReport, ClipVariant, PronunciationError, PronunciationFeatures,
    PronunciationScores, RecordedClip, Result, SessionConfig, TARGET_SAMPLE_RATE,
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
    auto_shutdown: bool,
}

pub struct SessionHandle {
    config: SessionConfig,
    controller: SessionController,
    updates: Receiver<SessionSnapshot>,
    initial: SessionSnapshot,
    join: Option<JoinHandle<()>>,
    pending: RefCell<VecDeque<SessionSnapshot>>,
}

#[derive(Clone, Debug)]
pub struct SessionSnapshot {
    pub alignment: AlignmentReport,
    pub scores: PronunciationScores,
    pub recording: bool,
    pub reference_playing: bool,
    pub latency_ms: f32,
    pub error: Option<String>,
    pub initializing: bool,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            alignment: AlignmentReport::default(),
            scores: PronunciationScores::default(),
            recording: false,
            reference_playing: false,
            latency_ms: 0.0,
            error: None,
            initializing: true,
        }
    }
}

impl SessionRuntime {
    pub fn new(config: SessionConfig) -> Result<Self> {
        validate_config(&config)?;
        info!(
            reference = %config.reference_wav.display(),
            latency_budget_ms = config.latency_budget_ms,
            "session config validated; launching runtime thread"
        );
        let thread_config = config.clone();
        let (command_tx, command_rx) = channel();
        let (update_tx, update_rx) = channel();
        let join = thread::Builder::new()
            .name("session-runtime".to_string())
            .spawn(move || {
                info!("runtime thread started; sending initializing snapshot");
                let initializing_snapshot = SessionSnapshot::default();
                let _ = update_tx.send(initializing_snapshot);
                match EngineRunner::build(thread_config) {
                    Ok(runner) => runner.run(command_rx, update_tx),
                    Err(err) => {
                        error!(error = %err, "failed to construct session engine");
                        let error_snapshot =
                            SessionSnapshot::default().with_error_message(err.to_string());
                        let _ = update_tx.send(error_snapshot);
                    }
                }
            })
            .map_err(|err| {
                error!(error = %err, "failed to spawn session runtime thread");
                PronunciationError::new(err.to_string())
            })?;
        info!("session runtime thread spawned; waiting for complete snapshot");
        let timeout = Duration::from_secs(60);
        let start = Instant::now();
        let initial = loop {
            if start.elapsed() > timeout {
                error!("timeout waiting for complete snapshot from runtime thread");
                break SessionSnapshot::default()
                    .with_error_message("timeout waiting for engine initialization".to_string());
            }
            match update_rx.recv_timeout(Duration::from_secs(1)) {
                Ok(snapshot) => {
                    if !snapshot.initializing {
                        info!("received complete snapshot from runtime thread");
                        break snapshot;
                    } else {
                        info!("received initializing snapshot; waiting for complete snapshot");
                    }
                }
                Err(_) => {
                    continue;
                }
            }
        };
        Ok(Self {
            config,
            controller: SessionController { tx: command_tx },
            updates: Some(update_rx),
            initial,
            join: Some(join),
            auto_shutdown: true,
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
        self.auto_shutdown = false;
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
            pending: RefCell::new(VecDeque::new()),
        }
    }

    pub fn launch(self) -> Result<()> {
        if !self.config.ui_enabled {
            return Err(PronunciationError::new(
                "interactive session must enable UI; headless mode is not supported",
            ));
        }
        crate::ui::launch_ui(self)
    }
}

impl Drop for SessionRuntime {
    fn drop(&mut self) {
        if self.auto_shutdown {
            let _ = self.controller.shutdown();
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
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
        self.extend_pending();
        self.pending.borrow_mut().pop_front()
    }

    pub fn drain_snapshots(&self) -> Vec<SessionSnapshot> {
        self.extend_pending();
        self.pending.borrow_mut().drain(..).collect()
    }

    fn extend_pending(&self) {
        let mut pending = self.pending.borrow_mut();
        for snapshot in self.pull_updates() {
            pending.push_back(snapshot);
        }
    }

    fn pull_updates(&self) -> Vec<SessionSnapshot> {
        let mut fresh = Vec::new();
        while let Ok(snapshot) = self.updates.try_recv() {
            fresh.push(snapshot);
        }
        fresh
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

    pub fn with_initializing(mut self, initializing: bool) -> Self {
        self.initializing = initializing;
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

    pub fn replay_reference(&self) -> Result<()> {
        self.send(SessionCommand::ReplayReference, "replay reference")
    }

    pub fn stop_replay(&self) -> Result<()> {
        self.send(SessionCommand::StopReplay, "stop replay")
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

type CaptureSettings = super::CaptureSettings;

pub mod engine {
    use super::{
        append_limited, min_required_samples, AlignmentReport, AudioAligner, CaptureSettings,
        ClipVariant, FeatureExtractor, MetricCalculator, PronunciationError, PronunciationFeatures,
        PronunciationScores, RecordedClip, Result, SessionSnapshot, CAPTURE_POLL_MS,
        TARGET_SAMPLE_RATE,
    };
    use crate::audio::capture::{CaptureConfig, LiveCapture};
    use crate::audio::resample;
    use crate::pronunciation::AlignmentWeights;
    use std::collections::{HashMap, VecDeque};
    use std::time::{Duration, Instant};
    use tracing::{debug, error, info, warn};

    pub trait CaptureSource {
        fn start(&mut self) -> Result<u32>;
        fn recv_chunk(&mut self, timeout: Duration) -> Option<Vec<f32>>;
        fn stop(&mut self);
    }

    pub struct SessionEngine<C: CaptureSource> {
        capture: C,
        extractor: FeatureExtractor,
        aligner: AudioAligner,
        metrics: MetricCalculator,
        reference_features_cache: HashMap<ClipVariant, PronunciationFeatures>,
        reference_alignment_cache: HashMap<ClipVariant, AlignmentReport>,
        active_clip: ClipVariant,
        learner_buffer: Vec<f32>,
        reference_samples: usize,
        latency_budget_ms: u32,
        capture_sample_rate: Option<u32>,
        chunk_count: usize,
    }

    impl<C: CaptureSource> SessionEngine<C> {
        pub fn new(
            reference: RecordedClip,
            alignment: AlignmentWeights,
            latency_budget_ms: u32,
            capture: C,
        ) -> Result<Self> {
            let extractor = FeatureExtractor::new();
            info!(
                samples = reference.samples.len(),
                duration_secs = reference.duration.as_secs_f64(),
                "starting feature extraction for reference audio"
            );
            let start = Instant::now();
            let reference_features = extractor.extract(&reference).map_err(|err| {
                let elapsed = start.elapsed();
                error!(
                    elapsed_secs = elapsed.as_secs_f64(),
                    error = %err,
                    "feature extraction failed"
                );
                err
            })?;
            let elapsed = start.elapsed();
            info!(
                elapsed_secs = elapsed.as_secs_f64(),
                energy_frames = reference_features.energy.len(),
                pitch_frames = reference_features.pitch_contour.len(),
                "feature extraction completed"
            );
            let mut reference_features_cache = HashMap::new();
            reference_features_cache.insert(ClipVariant::Original, reference_features.clone());
            let reference_alignment =
                Self::create_reference_alignment(&reference_features, reference.samples.len());
            let mut reference_alignment_cache = HashMap::new();
            reference_alignment_cache.insert(ClipVariant::Original, reference_alignment);
            Ok(Self {
                capture,
                extractor,
                aligner: AudioAligner::new(alignment),
                metrics: MetricCalculator::new(),
                reference_features_cache,
                reference_alignment_cache,
                active_clip: ClipVariant::Original,
                learner_buffer: Vec::new(),
                reference_samples: reference.samples.len(),
                latency_budget_ms,
                capture_sample_rate: None,
                chunk_count: 0,
            })
        }

        pub fn start(&mut self, snapshot: &mut SessionSnapshot) -> Result<SessionSnapshot> {
            info!("starting capture stream");
            let sample_rate = self.capture.start()?;
            self.capture_sample_rate = Some(sample_rate);
            self.learner_buffer.clear();
            self.chunk_count = 0;
            info!(sample_rate, "capture stream started successfully");
            *snapshot = snapshot.clone().with_recording(true, true);
            Ok(snapshot.clone())
        }

        pub fn poll(&mut self, snapshot: &mut SessionSnapshot) -> Result<Option<SessionSnapshot>> {
            let timeout = Duration::from_millis(CAPTURE_POLL_MS);
            if let Some(chunk) = self.capture.recv_chunk(timeout) {
                self.chunk_count += 1;
                if let Some(update) = self.process_chunk(chunk)? {
                    if update.latency_ms > self.latency_budget_ms as f32 {
                        warn!(
                            latency_ms = update.latency_ms,
                            budget_ms = self.latency_budget_ms,
                            "latency exceeds budget"
                        );
                    }
                    if self.chunk_count.is_multiple_of(50) {
                        debug!(
                            chunk = self.chunk_count,
                            latency_ms = update.latency_ms,
                            learner_samples = self.learner_buffer.len(),
                            "processed chunk"
                        );
                    }
                    *snapshot = snapshot
                        .clone()
                        .with_alignment(update.alignment, update.scores)
                        .with_recording(true, true)
                        .with_latency(update.latency_ms, self.latency_budget_ms);
                    return Ok(Some(snapshot.clone()));
                }
            }
            Ok(None)
        }

        pub fn stop(&mut self, snapshot: &mut SessionSnapshot) -> SessionSnapshot {
            info!(
                chunks_processed = self.chunk_count,
                "stopping capture stream"
            );
            self.capture.stop();
            self.capture_sample_rate = None;
            self.learner_buffer.clear();
            *snapshot = snapshot.clone().with_recording(false, false);
            snapshot.clone()
        }

        pub fn latency_budget_ms(&self) -> u32 {
            self.latency_budget_ms
        }

        pub fn reference_alignment(&self, variant: ClipVariant) -> AlignmentReport {
            self.get_reference_alignment(variant)
                .cloned()
                .unwrap_or_else(|_| {
                    warn!("cache miss for reference alignment, returning default");
                    AlignmentReport::default()
                })
        }

        fn get_reference_features(&self, variant: ClipVariant) -> Result<&PronunciationFeatures> {
            self.reference_features_cache.get(&variant).ok_or_else(|| {
                PronunciationError::new(format!("cache miss for variant: {:?}", variant))
            })
        }

        fn get_reference_alignment(&self, variant: ClipVariant) -> Result<&AlignmentReport> {
            self.reference_alignment_cache.get(&variant).ok_or_else(|| {
                PronunciationError::new(format!("cache miss for variant: {:?}", variant))
            })
        }

        fn create_reference_alignment(
            features: &PronunciationFeatures,
            sample_count: usize,
        ) -> AlignmentReport {
            let mut alignment = AlignmentReport::default();
            alignment.total_duration =
                Duration::from_secs_f32(sample_count as f32 / TARGET_SAMPLE_RATE as f32);
            alignment.reference_energy = features.energy.to_vec();
            alignment.reference_pitch = features.pitch_contour.to_vec();
            alignment.similarity_band = normalize_band(&alignment.reference_energy);
            alignment.contour_band = alignment.reference_pitch.clone();
            alignment
        }

        pub fn set_active_clip(&mut self, variant: ClipVariant) {
            self.active_clip = variant;
        }

        pub fn invalidate_flowalyzed_cache(&mut self) {
            self.reference_features_cache
                .remove(&ClipVariant::Flowalyzed);
            self.reference_alignment_cache
                .remove(&ClipVariant::Flowalyzed);
        }

        pub fn cache_flowalyzed_features(&mut self, clip: &RecordedClip) -> Result<()> {
            let features = self.extractor.extract(clip)?;
            let alignment = Self::create_reference_alignment(&features, clip.samples.len());
            self.reference_features_cache
                .insert(ClipVariant::Flowalyzed, features);
            self.reference_alignment_cache
                .insert(ClipVariant::Flowalyzed, alignment);
            Ok(())
        }

        fn process_chunk(&mut self, chunk: Vec<f32>) -> Result<Option<SnapshotUpdate>> {
            let capture_rate = self
                .capture_sample_rate
                .ok_or_else(|| PronunciationError::new("capture stream not started"))?;
            let resampled = resample::linear_resample(&chunk, capture_rate, TARGET_SAMPLE_RATE)
                .map_err(|err| PronunciationError::new(err.to_string()))?;
            let max_samples = self.max_samples();
            append_limited(&mut self.learner_buffer, &resampled, max_samples);
            if self.learner_buffer.len() < min_required_samples() {
                return Ok(None);
            }
            let start = Instant::now();
            let clip = RecordedClip::from_samples(self.learner_buffer.clone(), TARGET_SAMPLE_RATE);
            let features = self.extractor.extract(&clip)?;
            let ref_features = self.get_reference_features(self.active_clip)?;
            let alignment = self.aligner.align(ref_features, &features)?;
            let scores = self.metrics.score(&alignment)?;
            let latency_ms = start.elapsed().as_secs_f32() * 1000.0;
            Ok(Some(SnapshotUpdate {
                alignment,
                scores,
                latency_ms,
            }))
        }

        fn max_samples(&self) -> usize {
            self.reference_samples + TARGET_SAMPLE_RATE as usize / 2
        }
    }

    struct SnapshotUpdate {
        alignment: AlignmentReport,
        scores: PronunciationScores,
        latency_ms: f32,
    }

    pub struct LiveCaptureSource {
        config: CaptureConfig,
        live: Option<LiveCapture>,
    }

    impl LiveCaptureSource {
        pub fn new(settings: &CaptureSettings) -> Self {
            Self {
                config: super::build_capture_config(settings),
                live: None,
            }
        }
    }

    impl CaptureSource for LiveCaptureSource {
        fn start(&mut self) -> Result<u32> {
            info!(
                device = ?self.config.device_name,
                sample_rate = self.config.sample_rate,
                latency_ms = ?self.config.latency_ms,
                "starting live capture stream"
            );
            let live = LiveCapture::start(&self.config).map_err(|err| {
                let err_msg = err.to_string();
                error!(
                    device = ?self.config.device_name,
                    error = %err_msg,
                    "failed to start live capture stream"
                );
                PronunciationError::new(err_msg)
            })?;
            let sample_rate = live.sample_rate();
            self.live = Some(live);
            Ok(sample_rate)
        }

        fn recv_chunk(&mut self, timeout: Duration) -> Option<Vec<f32>> {
            self.live
                .as_ref()
                .and_then(|capture| capture.recv_chunk(timeout))
        }

        fn stop(&mut self) {
            if let Some(capture) = self.live.take() {
                capture.stop();
            }
        }
    }

    pub struct MockCapture {
        sample_rate: u32,
        chunks: VecDeque<Vec<f32>>,
        started: bool,
    }

    impl MockCapture {
        pub fn from_samples(sample_rate: u32, samples: Vec<f32>, chunk_len: usize) -> Self {
            let mut chunks = VecDeque::new();
            if chunk_len == 0 {
                chunks.push_back(samples);
            } else {
                for chunk in samples.chunks(chunk_len) {
                    chunks.push_back(chunk.to_vec());
                }
            }
            Self {
                sample_rate,
                chunks,
                started: false,
            }
        }
    }

    impl CaptureSource for MockCapture {
        fn start(&mut self) -> Result<u32> {
            self.started = true;
            Ok(self.sample_rate)
        }

        fn recv_chunk(&mut self, _timeout: Duration) -> Option<Vec<f32>> {
            if !self.started {
                return None;
            }
            self.chunks.pop_front()
        }

        fn stop(&mut self) {
            self.started = false;
        }
    }

    fn normalize_band(values: &[f32]) -> Vec<f32> {
        if values.is_empty() {
            return Vec::new();
        }
        let max = values
            .iter()
            .cloned()
            .fold(0.0_f32, |acc, v| acc.max(v.abs()))
            .max(1e-6);
        values
            .iter()
            .map(|v| (v.abs() / max).clamp(0.0, 1.0))
            .collect()
    }
}

struct EngineRunner {
    engine: engine::SessionEngine<engine::LiveCaptureSource>,
    original_reference: RecordedClip,
    active_clip: ClipVariant,
    initial_snapshot: SessionSnapshot,
    player: Option<ReferencePlayer>,
}

impl EngineRunner {
    fn build(config: SessionConfig) -> Result<Self> {
        info!(
            path = %config.reference_wav.display(),
            "loading reference WAV file"
        );
        let original_reference = load_clip(&config.reference_wav)?;
        info!(
            duration_secs = original_reference.duration.as_secs_f64(),
            sample_rate = original_reference.sample_rate,
            samples = original_reference.samples.len(),
            "reference WAV loaded successfully"
        );
        info!(
            device = ?config.capture.device_name,
            sample_rate = config.capture.sample_rate,
            latency_ms = ?config.capture.latency_ms,
            "creating live capture source"
        );
        let capture = engine::LiveCaptureSource::new(&config.capture);
        info!("creating session engine (this will extract features from reference)");
        let build_start = Instant::now();
        let engine = engine::SessionEngine::new(
            original_reference.clone(),
            config.alignment,
            config.latency_budget_ms,
            capture,
        )
        .map_err(|err| {
            let elapsed = build_start.elapsed();
            error!(
                elapsed_secs = elapsed.as_secs_f64(),
                error = %err,
                "failed to create session engine"
            );
            err
        })?;
        let engine_elapsed = build_start.elapsed();
        info!(
            elapsed_secs = engine_elapsed.as_secs_f64(),
            "session engine created successfully"
        );
        info!("computing initial reference alignment");
        let alignment_start = Instant::now();
        let initial_alignment = engine.reference_alignment(ClipVariant::Original);
        let alignment_elapsed = alignment_start.elapsed();
        info!(
            elapsed_secs = alignment_elapsed.as_secs_f64(),
            phonemes = initial_alignment.phonemes.len(),
            "initial reference alignment computed"
        );
        let initial_snapshot = SessionSnapshot::default()
            .with_alignment(initial_alignment, PronunciationScores::default())
            .with_initializing(false);
        let total_elapsed = build_start.elapsed();
        info!(
            total_elapsed_secs = total_elapsed.as_secs_f64(),
            "engine runner built; ready to send initial snapshot"
        );
        Ok(Self {
            engine,
            original_reference,
            active_clip: ClipVariant::Original,
            initial_snapshot,
            player: None,
        })
    }

    fn active_clip(&self) -> &RecordedClip {
        match self.active_clip {
            ClipVariant::Original => &self.original_reference,
            ClipVariant::Flowalyzed => {
                panic!("active_clip is Flowalyzed but flowalyzed_reference not implemented until Phase 4.2")
            }
        }
    }

    fn run(mut self, commands: Receiver<SessionCommand>, updates: Sender<SessionSnapshot>) {
        let mut snapshot = self.initial_snapshot.clone();
        info!("session runtime thread running; emitting initial snapshot");
        let _ = updates.send(snapshot.clone());
        while let Ok(command) = commands.recv() {
            match command {
                SessionCommand::ReplayReference => {
                    self.handle_replay_reference(&updates, &mut snapshot);
                }
                SessionCommand::StopReplay => {
                    self.handle_stop_replay(&updates, &mut snapshot);
                }
                SessionCommand::Start => {
                    info!("received start command");
                    match self.handle_start(&commands, &updates, &mut snapshot) {
                        LoopExit::Finished => {}
                        LoopExit::Shutdown => break,
                    }
                }
                SessionCommand::Stop => {
                    info!("received stop command");
                    let update = self.engine.stop(&mut snapshot);
                    let _ = updates.send(update);
                }
                SessionCommand::Shutdown => {
                    info!("received shutdown command");
                    break;
                }
            }
        }
        info!("session runtime thread exiting");
    }

    fn get_or_create_player(&mut self) -> Result<&mut ReferencePlayer> {
        if self.player.is_none() {
            self.player = Some(ReferencePlayer::new(self.active_clip())?);
        }
        Ok(self.player.as_mut().unwrap())
    }

    fn handle_replay_reference(
        &mut self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) {
        info!("replay reference command received");
        match self.get_or_create_player() {
            Ok(player) => {
                player.stop();
                if let Err(err) = player.play() {
                    error!(error = %err, "failed to replay reference");
                    snapshot.error = Some(err.to_string());
                } else {
                    snapshot.reference_playing = true;
                }
                let _ = updates.send(snapshot.clone());
            }
            Err(err) => {
                error!(error = %err, "failed to get player for replay");
                snapshot.error = Some(err.to_string());
                let _ = updates.send(snapshot.clone());
            }
        }
    }

    fn handle_stop_replay(
        &mut self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) {
        info!("stop replay command received");
        if let Some(player) = self.player.as_mut() {
            player.stop();
            snapshot.reference_playing = false;
            let _ = updates.send(snapshot.clone());
        }
    }

    fn handle_start(
        &mut self,
        commands: &Receiver<SessionCommand>,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) -> LoopExit {
        info!("recording session starting");
        let start_update = match self.engine.start(snapshot) {
            Ok(update) => update,
            Err(err) => {
                error!(error = %err, "failed to start capture engine");
                emit_error(updates, snapshot, err.to_string());
                return LoopExit::Finished;
            }
        };
        let _ = updates.send(start_update);
        info!("starting reference playback");
        let player = match self.get_or_create_player() {
            Ok(player) => player,
            Err(err) => {
                error!(error = %err, "failed to create reference player");
                self.engine.stop(snapshot);
                emit_error(updates, snapshot, err.to_string());
                return LoopExit::Finished;
            }
        };
        if let Err(err) = player.play() {
            error!(error = %err, "failed to start reference playback");
            self.engine.stop(snapshot);
            emit_error(updates, snapshot, err.to_string());
            return LoopExit::Finished;
        }
        info!("recording session active; entering drive loop");
        self.drive(commands, updates, snapshot)
    }

    fn drive(
        &mut self,
        commands: &Receiver<SessionCommand>,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) -> LoopExit {
        loop {
            if let Some(command) = poll_command(commands) {
                match command {
                    SessionCommand::Shutdown => {
                        info!("shutdown command received");
                        let update = self.engine.stop(snapshot);
                        if let Some(player) = self.player.as_mut() {
                            player.stop();
                        }
                        let _ = updates.send(update);
                        return LoopExit::Shutdown;
                    }
                    SessionCommand::Stop => {
                        info!("stop command received");
                        let update = self.engine.stop(snapshot);
                        if let Some(player) = self.player.as_mut() {
                            player.stop();
                        }
                        let _ = updates.send(update);
                        return LoopExit::Finished;
                    }
                    SessionCommand::Start => {
                        debug!("start command received while already recording");
                    }
                    SessionCommand::ReplayReference => {
                        self.handle_replay_reference(updates, snapshot);
                    }
                    SessionCommand::StopReplay => {
                        self.handle_stop_replay(updates, snapshot);
                    }
                }
            }
            match self.engine.poll(snapshot) {
                Ok(Some(update)) => {
                    let _ = updates.send(update);
                }
                Ok(None) => {}
                Err(err) => {
                    error!(error = %err, "capture engine error during poll");
                    self.engine.stop(snapshot);
                    if let Some(player) = self.player.as_mut() {
                        player.stop();
                    }
                    emit_error(updates, snapshot, err.to_string());
                    return LoopExit::Finished;
                }
            }
        }
    }
}

enum LoopExit {
    Finished,
    Shutdown,
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
    ReplayReference,
    StopReplay,
    Shutdown,
}
