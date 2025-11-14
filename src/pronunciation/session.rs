use std::cell::RefCell;
use std::collections::VecDeque;
use std::ops::RangeInclusive;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rodio::buffer::SamplesBuffer;
use rodio::{OutputStream, Sink};

use crate::audio::capture::CaptureConfig;
use crate::audio::playback::duplicate_to_stereo;
use crate::types::Recipe;
use tracing::{debug, error, info};

use super::alignment::AudioAligner;
use super::features::{FeatureExtractionEvent, FeatureExtractionPhase, FeatureExtractor};
use super::metrics::MetricCalculator;
use super::validate_config;
use super::{
    apply_recipe_to_range, load_clip, AlignmentReport, ClipVariant, PronunciationError,
    PronunciationFeatures, PronunciationScores, RecordedClip, Result, SessionConfig,
    TARGET_SAMPLE_RATE,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitializationStage {
    LoadingReference,
    ExtractingReferenceFeatures,
    ComputingAlignment,
    Finalizing,
}

impl InitializationStage {
    pub fn label(&self) -> &'static str {
        match self {
            InitializationStage::LoadingReference => "Loading reference audio",
            InitializationStage::ExtractingReferenceFeatures => {
                "Extracting reference features (mel + pitch)"
            }
            InitializationStage::ComputingAlignment => "Computing initial alignment",
            InitializationStage::Finalizing => "Finalizing session state",
        }
    }

    pub fn order(&self) -> usize {
        match self {
            InitializationStage::LoadingReference => 0,
            InitializationStage::ExtractingReferenceFeatures => 1,
            InitializationStage::ComputingAlignment => 2,
            InitializationStage::Finalizing => 3,
        }
    }

    pub fn ordered() -> &'static [InitializationStage; 4] {
        &[
            InitializationStage::LoadingReference,
            InitializationStage::ExtractingReferenceFeatures,
            InitializationStage::ComputingAlignment,
            InitializationStage::Finalizing,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeApplicationStage {
    ExtractingRange,
    ApplyingRecipe,
    AssemblingChunks,
    ExtractingFeatures,
    Caching,
}

impl RecipeApplicationStage {
    pub fn label(&self) -> &'static str {
        match self {
            RecipeApplicationStage::ExtractingRange => "Extracting audio range",
            RecipeApplicationStage::ApplyingRecipe => "Applying recipe operations",
            RecipeApplicationStage::AssemblingChunks => "Assembling audio chunks",
            RecipeApplicationStage::ExtractingFeatures => "Extracting features (mel + pitch)",
            RecipeApplicationStage::Caching => "Caching features and alignment",
        }
    }

    pub fn order(&self) -> usize {
        match self {
            RecipeApplicationStage::ExtractingRange => 0,
            RecipeApplicationStage::ApplyingRecipe => 1,
            RecipeApplicationStage::AssemblingChunks => 2,
            RecipeApplicationStage::ExtractingFeatures => 3,
            RecipeApplicationStage::Caching => 4,
        }
    }

    pub fn ordered() -> &'static [RecipeApplicationStage; 5] {
        &[
            RecipeApplicationStage::ExtractingRange,
            RecipeApplicationStage::ApplyingRecipe,
            RecipeApplicationStage::AssemblingChunks,
            RecipeApplicationStage::ExtractingFeatures,
            RecipeApplicationStage::Caching,
        ]
    }
}

#[derive(Clone, Debug)]
pub struct InitializationProgress {
    pub stage: InitializationStage,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub sub_stage_index: u32,
    pub sub_stage_total: u32,
    pub sub_stage_label: Option<String>,
    pub metric_label: Option<String>,
    pub current_value: Option<u32>,
    pub total_value: Option<u32>,
    pub elapsed_secs: Option<u32>,
}

impl InitializationProgress {
    fn new(stage: InitializationStage) -> Self {
        Self {
            stage,
            completed_steps: stage.order() as u32,
            total_steps: InitializationStage::ordered().len() as u32,
            sub_stage_index: 0,
            sub_stage_total: 0,
            sub_stage_label: None,
            metric_label: None,
            current_value: None,
            total_value: None,
            elapsed_secs: None,
        }
    }

    fn with_sub_stage<S: Into<String>>(mut self, index: u32, total: u32, label: S) -> Self {
        self.sub_stage_index = index;
        self.sub_stage_total = total;
        self.sub_stage_label = Some(label.into());
        self
    }

    fn with_metric<S: Into<String>>(mut self, label: S, current: u32, total: u32) -> Self {
        self.metric_label = Some(label.into());
        self.current_value = Some(current);
        self.total_value = Some(total);
        self
    }

    fn with_elapsed(mut self, elapsed_secs: u32) -> Self {
        self.elapsed_secs = Some(elapsed_secs);
        self
    }
}

#[derive(Clone, Debug)]
pub struct RecipeApplicationProgress {
    pub stage: RecipeApplicationStage,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub sub_stage_index: u32,
    pub sub_stage_total: u32,
    pub sub_stage_label: Option<String>,
    pub metric_label: Option<String>,
    pub current_value: Option<u32>,
    pub total_value: Option<u32>,
    pub elapsed_secs: Option<u32>,
}

impl RecipeApplicationProgress {
    fn new(stage: RecipeApplicationStage) -> Self {
        Self {
            stage,
            completed_steps: stage.order() as u32,
            total_steps: RecipeApplicationStage::ordered().len() as u32,
            sub_stage_index: 0,
            sub_stage_total: 0,
            sub_stage_label: None,
            metric_label: None,
            current_value: None,
            total_value: None,
            elapsed_secs: None,
        }
    }

    fn with_sub_stage<S: Into<String>>(mut self, index: u32, total: u32, label: S) -> Self {
        self.sub_stage_index = index;
        self.sub_stage_total = total;
        self.sub_stage_label = Some(label.into());
        self
    }

    fn with_metric<S: Into<String>>(mut self, label: S, current: u32, total: u32) -> Self {
        self.metric_label = Some(label.into());
        self.current_value = Some(current);
        self.total_value = Some(total);
        self
    }

    fn with_elapsed(mut self, elapsed_secs: u32) -> Self {
        self.elapsed_secs = Some(elapsed_secs);
        self
    }
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
    pub init_progress: Option<InitializationProgress>,
    pub recipe_applying: bool,
    pub recipe_progress: Option<RecipeApplicationProgress>,
    pub active_clip_variant: ClipVariant,
    pub has_flowalyzed_clip: bool,
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
            init_progress: None,
            recipe_applying: false,
            recipe_progress: None,
            active_clip_variant: ClipVariant::Original,
            has_flowalyzed_clip: false,
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
                let emitter = InitializationEmitter::new(&update_tx);
                match EngineRunner::build(thread_config, &emitter) {
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
        info!("session runtime thread spawned; awaiting first snapshot");
        let timeout = Duration::from_secs(60);
        let initial = match update_rx.recv_timeout(timeout) {
            Ok(snapshot) => {
                if snapshot.initializing {
                    info!("received initializing snapshot; returning control to UI");
                } else {
                    info!("received complete snapshot from runtime thread");
                }
                snapshot
            }
            Err(_) => {
                error!("timeout waiting for first snapshot from runtime thread");
                SessionSnapshot::default()
                    .with_error_message("timeout waiting for engine initialization".to_string())
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
        if !initializing {
            self.init_progress = None;
        }
        self
    }

    pub fn with_progress(mut self, progress: InitializationProgress) -> Self {
        self.initializing = true;
        self.init_progress = Some(progress);
        self
    }

    pub fn with_recipe_applying(mut self, recipe_applying: bool) -> Self {
        self.recipe_applying = recipe_applying;
        if !recipe_applying {
            self.recipe_progress = None;
        }
        self
    }

    pub fn with_recipe_progress(mut self, progress: RecipeApplicationProgress) -> Self {
        self.recipe_applying = true;
        self.recipe_progress = Some(progress);
        self
    }
}

struct InitializationEmitter {
    tx: Sender<SessionSnapshot>,
}

impl InitializationEmitter {
    fn new(tx: &Sender<SessionSnapshot>) -> Self {
        Self { tx: tx.clone() }
    }

    fn stage(&self, stage: InitializationStage) {
        let progress = InitializationProgress::new(stage);
        self.send(progress);
    }

    fn stage_detail<S: Into<String>>(
        &self,
        stage: InitializationStage,
        index: u32,
        total: u32,
        label: S,
    ) {
        let progress = InitializationProgress::new(stage).with_sub_stage(index, total, label);
        self.send(progress);
    }

    fn handle_feature_event(&self, event: FeatureExtractionEvent) {
        match event {
            FeatureExtractionEvent::PhaseStart(phase) => {
                self.stage_detail(
                    InitializationStage::ExtractingReferenceFeatures,
                    phase.order() as u32,
                    FeatureExtractionPhase::total() as u32,
                    phase.label(),
                );
            }
            FeatureExtractionEvent::PhaseProgress {
                phase,
                label,
                current,
                total,
            } => {
                let progress =
                    InitializationProgress::new(InitializationStage::ExtractingReferenceFeatures)
                        .with_sub_stage(
                            phase.order() as u32,
                            FeatureExtractionPhase::total() as u32,
                            phase.label(),
                        )
                        .with_metric(label, current, total);
                self.send(progress);
            }
            FeatureExtractionEvent::PhaseElapsed {
                phase,
                elapsed_secs,
            } => {
                let progress =
                    InitializationProgress::new(InitializationStage::ExtractingReferenceFeatures)
                        .with_sub_stage(
                            phase.order() as u32,
                            FeatureExtractionPhase::total() as u32,
                            phase.label(),
                        )
                        .with_elapsed(elapsed_secs);
                self.send(progress);
            }
        }
    }

    fn send(&self, progress: InitializationProgress) {
        let snapshot = SessionSnapshot::default().with_progress(progress);
        let _ = self.tx.send(snapshot);
    }
}

struct RecipeApplicationEmitter {
    tx: Sender<SessionSnapshot>,
    latest_metrics: Option<(String, u32, u32)>,
}

impl RecipeApplicationEmitter {
    fn new(tx: &Sender<SessionSnapshot>) -> Self {
        Self {
            tx: tx.clone(),
            latest_metrics: None,
        }
    }

    fn stage(&self, stage: RecipeApplicationStage) {
        let progress = RecipeApplicationProgress::new(stage);
        self.send(progress);
    }

    fn stage_detail<S: Into<String>>(
        &self,
        stage: RecipeApplicationStage,
        index: u32,
        total: u32,
        label: S,
    ) {
        let progress = RecipeApplicationProgress::new(stage).with_sub_stage(index, total, label);
        self.send(progress);
    }

    fn handle_feature_event(&mut self, event: FeatureExtractionEvent) {
        match event {
            FeatureExtractionEvent::PhaseStart(phase) => {
                self.stage_detail(
                    RecipeApplicationStage::ExtractingFeatures,
                    phase.order() as u32,
                    FeatureExtractionPhase::total() as u32,
                    phase.label(),
                );
            }
            FeatureExtractionEvent::PhaseProgress {
                phase,
                label,
                current,
                total,
            } => {
                self.latest_metrics = Some((label.to_string(), current, total));
                let progress =
                    RecipeApplicationProgress::new(RecipeApplicationStage::ExtractingFeatures)
                        .with_sub_stage(
                            phase.order() as u32,
                            FeatureExtractionPhase::total() as u32,
                            phase.label(),
                        )
                        .with_metric(label, current, total);
                self.send(progress);
            }
            FeatureExtractionEvent::PhaseElapsed {
                phase,
                elapsed_secs,
            } => {
                let mut progress =
                    RecipeApplicationProgress::new(RecipeApplicationStage::ExtractingFeatures)
                        .with_sub_stage(
                            phase.order() as u32,
                            FeatureExtractionPhase::total() as u32,
                            phase.label(),
                        )
                        .with_elapsed(elapsed_secs);
                if let Some((ref label, current, total)) = self.latest_metrics {
                    progress = progress.with_metric(label.clone(), current, total);
                }
                self.send(progress);
            }
        }
    }

    fn send(&self, progress: RecipeApplicationProgress) {
        let snapshot = SessionSnapshot::default().with_recipe_progress(progress);
        let _ = self.tx.send(snapshot);
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

    pub fn apply_recipe(&self, range_start: f64, range_end: f64, recipe: Recipe) -> Result<()> {
        self.send(
            SessionCommand::ApplyFlowalyzerRecipe {
                range_start,
                range_end,
                recipe,
            },
            "apply flowalyzer recipe",
        )
    }

    pub fn toggle_clip_variant(&self, variant: ClipVariant) -> Result<()> {
        self.send(
            SessionCommand::ToggleClipVariant { variant },
            "toggle clip variant",
        )
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
    use crate::pronunciation::features::FeatureExtractionEvent;
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
        reference: RecordedClip,
        reference_samples: usize,
        latency_budget_ms: u32,
        capture_sample_rate: Option<u32>,
        chunk_count: usize,
        defer_feature_extraction: bool,
    }

    impl<C: CaptureSource> SessionEngine<C> {
        pub fn new(
            reference: RecordedClip,
            alignment: AlignmentWeights,
            latency_budget_ms: u32,
            capture: C,
            defer_feature_extraction: bool,
        ) -> Result<Self> {
            Self::new_with_progress(
                reference,
                alignment,
                latency_budget_ms,
                capture,
                defer_feature_extraction,
                None,
            )
        }

        pub fn new_with_progress(
            reference: RecordedClip,
            alignment: AlignmentWeights,
            latency_budget_ms: u32,
            capture: C,
            defer_feature_extraction: bool,
            feature_progress: Option<&dyn Fn(FeatureExtractionEvent)>,
        ) -> Result<Self> {
            let extractor = FeatureExtractor::new();
            let aligner = AudioAligner::new(alignment);
            let mut reference_features_cache = HashMap::new();
            let mut reference_alignment_cache = HashMap::new();

            if !defer_feature_extraction {
                info!(
                    samples = reference.samples.len(),
                    duration_secs = reference.duration.as_secs_f64(),
                    "starting feature extraction for reference audio"
                );
                let start = Instant::now();
                let reference_features = match feature_progress {
                    Some(reporter) => {
                        let mut event_reporter = |event: FeatureExtractionEvent| {
                            reporter(event);
                        };
                        extractor
                            .extract_with_progress(&reference, &mut event_reporter)
                            .map_err(|err| {
                                let elapsed = start.elapsed();
                                error!(
                                    elapsed_secs = elapsed.as_secs_f64(),
                                    error = %err,
                                    "feature extraction failed"
                                );
                                err
                            })?
                    }
                    None => extractor.extract(&reference).map_err(|err| {
                        let elapsed = start.elapsed();
                        error!(
                            elapsed_secs = elapsed.as_secs_f64(),
                            error = %err,
                            "feature extraction failed"
                        );
                        err
                    })?,
                };
                let elapsed = start.elapsed();
                info!(
                    elapsed_secs = elapsed.as_secs_f64(),
                    energy_frames = reference_features.energy.len(),
                    pitch_frames = reference_features.pitch_contour.len(),
                    "feature extraction completed"
                );
                reference_features_cache.insert(ClipVariant::Original, reference_features.clone());
                let reference_alignment =
                    aligner.align(&reference_features, &reference_features)?;
                reference_alignment_cache.insert(ClipVariant::Original, reference_alignment);
            }
            let reference_samples = reference.samples.len();
            Ok(Self {
                capture,
                extractor,
                aligner,
                metrics: MetricCalculator::new(),
                reference_features_cache,
                reference_alignment_cache,
                active_clip: ClipVariant::Original,
                learner_buffer: Vec::new(),
                reference,
                reference_samples,
                latency_budget_ms,
                capture_sample_rate: None,
                chunk_count: 0,
                defer_feature_extraction,
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

        pub fn reference_alignment(&mut self, variant: ClipVariant) -> Result<AlignmentReport> {
            self.ensure_features(variant)?;
            match self.get_reference_alignment(variant) {
                Ok(alignment) => Ok(alignment.clone()),
                Err(_) => {
                    warn!("cache miss for reference alignment, returning default");
                    Ok(AlignmentReport::default())
                }
            }
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

        fn ensure_features(&mut self, variant: ClipVariant) -> Result<()> {
            if self.reference_features_cache.contains_key(&variant) {
                return Ok(());
            }
            self.validate_defer_mode(variant)?;
            let reference_clip = self.get_clip_for_variant(variant)?;
            let reference_features = self.extract_features_lazy(reference_clip, variant)?;
            self.cache_features_and_alignment(variant, reference_features)
        }

        fn validate_defer_mode(&self, variant: ClipVariant) -> Result<()> {
            if !self.defer_feature_extraction {
                return Err(PronunciationError::new(format!(
                    "cache miss for variant: {:?} (feature extraction was not deferred)",
                    variant
                )));
            }
            Ok(())
        }

        fn get_clip_for_variant(&self, variant: ClipVariant) -> Result<&RecordedClip> {
            match variant {
                ClipVariant::Original => Ok(&self.reference),
                ClipVariant::Flowalyzed => Err(PronunciationError::new(
                    "Flowalyzed variant requires explicit caching via cache_flowalyzed_features",
                )),
            }
        }

        fn extract_features_lazy(
            &self,
            clip: &RecordedClip,
            variant: ClipVariant,
        ) -> Result<PronunciationFeatures> {
            info!(
                variant = ?variant,
                samples = clip.samples.len(),
                duration_secs = clip.duration.as_secs_f64(),
                "lazy extracting features for reference audio"
            );
            let start = Instant::now();
            let features = self.extractor.extract(clip).map_err(|err| {
                let elapsed = start.elapsed();
                error!(
                    elapsed_secs = elapsed.as_secs_f64(),
                    error = %err,
                    variant = ?variant,
                    "lazy feature extraction failed"
                );
                err
            })?;
            let elapsed = start.elapsed();
            info!(
                elapsed_secs = elapsed.as_secs_f64(),
                energy_frames = features.energy.len(),
                pitch_frames = features.pitch_contour.len(),
                variant = ?variant,
                "lazy feature extraction completed"
            );
            Ok(features)
        }

        fn cache_features_and_alignment(
            &mut self,
            variant: ClipVariant,
            features: PronunciationFeatures,
        ) -> Result<()> {
            let alignment = self.aligner.align(&features, &features)?;
            self.reference_features_cache.insert(variant, features);
            self.reference_alignment_cache.insert(variant, alignment);
            Ok(())
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

        pub fn cache_flowalyzed_features(
            &mut self,
            clip: &RecordedClip,
            progress: Option<Box<dyn FnMut(FeatureExtractionEvent)>>,
        ) -> Result<()> {
            let features = if let Some(mut progress_cb) = progress {
                self.extractor
                    .extract_with_progress(clip, &mut progress_cb)?
            } else {
                self.extractor.extract(clip)?
            };
            self.cache_features_and_alignment(ClipVariant::Flowalyzed, features)
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
            self.ensure_features(self.active_clip)?;
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
}

struct EngineRunner {
    engine: engine::SessionEngine<engine::LiveCaptureSource>,
    original_reference: RecordedClip,
    flowalyzed_clip: Option<RecordedClip>,
    initial_snapshot: SessionSnapshot,
    playback_sink: Option<Sink>,
    _playback_stream: Option<OutputStream>, // Keep stream alive during playback
}

impl EngineRunner {
    fn build(config: SessionConfig, progress: &InitializationEmitter) -> Result<Self> {
        info!(
            path = %config.reference_wav.display(),
            "loading reference WAV file"
        );
        progress.stage(InitializationStage::LoadingReference);
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
        progress.stage(InitializationStage::ExtractingReferenceFeatures);
        let feature_progress = |event: FeatureExtractionEvent| {
            progress.handle_feature_event(event);
        };
        let build_start = Instant::now();
        let mut engine = engine::SessionEngine::new_with_progress(
            original_reference.clone(),
            config.alignment,
            config.latency_budget_ms,
            capture,
            false, // Don't defer in production
            Some(&feature_progress),
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
        progress.stage(InitializationStage::ComputingAlignment);
        let alignment_start = Instant::now();
        let initial_alignment = engine.reference_alignment(ClipVariant::Original)?;
        let alignment_elapsed = alignment_start.elapsed();
        info!(
            elapsed_secs = alignment_elapsed.as_secs_f64(),
            phonemes = initial_alignment.phonemes.len(),
            "initial reference alignment computed"
        );
        progress.stage(InitializationStage::Finalizing);
        let mut initial_snapshot = SessionSnapshot::default()
            .with_alignment(initial_alignment, PronunciationScores::default())
            .with_initializing(false);
        initial_snapshot.active_clip_variant = ClipVariant::Original;
        let total_elapsed = build_start.elapsed();
        info!(
            total_elapsed_secs = total_elapsed.as_secs_f64(),
            "engine runner built; ready to send initial snapshot"
        );
        Ok(Self {
            engine,
            original_reference,
            flowalyzed_clip: None,
            initial_snapshot,
            playback_sink: None,
            _playback_stream: None,
        })
    }

    fn active_clip(&self, variant: ClipVariant) -> Result<&RecordedClip> {
        match variant {
            ClipVariant::Original => Ok(&self.original_reference),
            ClipVariant::Flowalyzed => self
                .flowalyzed_clip
                .as_ref()
                .ok_or_else(|| PronunciationError::new("flowalyzed clip not available")),
        }
    }

    fn generate_flowalyzed_clip(
        &self,
        range_start: f64,
        range_end: f64,
        recipe: &Recipe,
    ) -> Result<RecordedClip> {
        apply_recipe_to_range(&self.original_reference, range_start, range_end, recipe)
    }

    fn cache_and_activate_flowalyzed(
        &mut self,
        clip: &RecordedClip,
        progress: Option<Box<dyn FnMut(FeatureExtractionEvent)>>,
    ) -> Result<()> {
        self.engine.cache_flowalyzed_features(clip, progress)?;
        self.engine.set_active_clip(ClipVariant::Flowalyzed);
        Ok(())
    }

    fn run(mut self, commands: Receiver<SessionCommand>, updates: Sender<SessionSnapshot>) {
        let mut snapshot = self.initial_snapshot.clone();
        // Snapshot already has correct variant from initial_snapshot, don't overwrite
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
                    let mut update = self.engine.stop(&mut snapshot);
                    // Preserve snapshot's variant and flowalyzed state, don't overwrite
                    update.active_clip_variant = snapshot.active_clip_variant;
                    update.has_flowalyzed_clip = snapshot.has_flowalyzed_clip;
                    let _ = updates.send(update);
                }
                SessionCommand::Shutdown => {
                    info!("received shutdown command");
                    break;
                }
                SessionCommand::ApplyFlowalyzerRecipe {
                    range_start,
                    range_end,
                    recipe,
                } => {
                    self.handle_apply_recipe(
                        &updates,
                        &mut snapshot,
                        range_start,
                        range_end,
                        recipe,
                    );
                }
                SessionCommand::ToggleClipVariant { variant } => {
                    self.handle_toggle_variant(&updates, &mut snapshot, variant);
                }
            }
        }
        info!("session runtime thread exiting");
    }

    fn handle_replay_reference(
        &mut self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) {
        info!("replay reference command received");
        // Stop any existing playback
        self.stop_playback();
        // Get clip from snapshot variant (source of truth) and prepare samples
        let (stereo_samples, sample_rate) = match self.active_clip(snapshot.active_clip_variant) {
            Ok(clip) => (duplicate_to_stereo(&clip.samples), clip.sample_rate),
            Err(err) => {
                error!(error = %err, "failed to get clip for replay");
                snapshot.error = Some(err.to_string());
                let _ = updates.send(snapshot.clone());
                return;
            }
        };
        // Create stream and sink, play clip
        match self.start_playback_with_samples(stereo_samples, sample_rate) {
            Ok(()) => {
                snapshot.reference_playing = true;
                let _ = updates.send(snapshot.clone());
            }
            Err(err) => {
                error!(error = %err, "failed to start playback");
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
        self.stop_playback();
        snapshot.reference_playing = false;
        let _ = updates.send(snapshot.clone());
    }

    fn start_playback_with_samples(
        &mut self,
        stereo_samples: Vec<f32>,
        sample_rate: u32,
    ) -> Result<()> {
        let (stream, handle) =
            OutputStream::try_default().map_err(|err| PronunciationError::new(err.to_string()))?;
        let sink =
            Sink::try_new(&handle).map_err(|err| PronunciationError::new(err.to_string()))?;
        let buffer = SamplesBuffer::new(2, sample_rate, stereo_samples);
        sink.append(buffer);
        sink.play();
        sink.set_volume(1.0);
        self.playback_sink = Some(sink);
        self._playback_stream = Some(stream);
        Ok(())
    }

    fn stop_playback(&mut self) {
        if let Some(sink) = self.playback_sink.take() {
            sink.stop();
        }
        self._playback_stream = None;
    }

    fn handle_apply_recipe(
        &mut self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
        range_start: f64,
        range_end: f64,
        recipe: Recipe,
    ) {
        let emitter = RecipeApplicationEmitter::new(updates);
        emitter.stage(RecipeApplicationStage::ExtractingRange);
        self.prepare_recipe_application(range_start, range_end, &recipe);
        emitter.stage(RecipeApplicationStage::ApplyingRecipe);
        let new_clip = match self.generate_flowalyzed_clip(range_start, range_end, &recipe) {
            Ok(clip) => clip,
            Err(err) => {
                self.emit_generation_error(updates, snapshot, err);
                return;
            }
        };
        emitter.stage(RecipeApplicationStage::AssemblingChunks);
        emitter.stage(RecipeApplicationStage::ExtractingFeatures);
        let tx = updates.clone();
        use std::cell::RefCell;
        use std::rc::Rc;
        let metrics_state = Rc::new(RefCell::new(None::<(String, u32, u32)>));
        let metrics_state_clone = metrics_state.clone();
        let progress_callback = Box::new(move |event: FeatureExtractionEvent| {
            let mut temp_emitter = RecipeApplicationEmitter::new(&tx);
            temp_emitter.latest_metrics = metrics_state_clone.borrow().clone();
            temp_emitter.handle_feature_event(event);
            if let Some(metrics) = &temp_emitter.latest_metrics {
                *metrics_state_clone.borrow_mut() = Some(metrics.clone());
            }
        });
        if let Err(err) =
            self.activate_flowalyzed_clip(&new_clip, snapshot, Some(progress_callback))
        {
            self.emit_cache_error(updates, snapshot, err);
            return;
        }
        emitter.stage(RecipeApplicationStage::Caching);
        self.complete_recipe_application(new_clip, updates, snapshot);
    }

    fn prepare_recipe_application(&mut self, range_start: f64, range_end: f64, recipe: &Recipe) {
        info!(
            range_start, range_end, recipe = %recipe.name,
            "apply recipe command received"
        );
        self.engine.invalidate_flowalyzed_cache();
    }

    fn complete_recipe_application(
        &mut self,
        new_clip: RecordedClip,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) {
        self.flowalyzed_clip = Some(new_clip);
        snapshot.active_clip_variant = ClipVariant::Flowalyzed;
        snapshot.has_flowalyzed_clip = true;
        snapshot.recipe_applying = false;
        info!("flowalyzed clip generated and activated successfully");
        let _ = updates.send(snapshot.clone());
    }

    fn emit_generation_error(
        &self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
        err: PronunciationError,
    ) {
        error!(error = %err, "failed to generate flowalyzed clip");
        snapshot.error = Some(err.to_string());
        // Don't overwrite snapshot's variant - preserve current state
        snapshot.recipe_applying = false;
        let _ = updates.send(snapshot.clone());
    }

    fn activate_flowalyzed_clip(
        &mut self,
        new_clip: &RecordedClip,
        _snapshot: &mut SessionSnapshot,
        progress: Option<Box<dyn FnMut(FeatureExtractionEvent)>>,
    ) -> Result<()> {
        self.cache_and_activate_flowalyzed(new_clip, progress)
    }

    fn emit_cache_error(
        &self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
        err: PronunciationError,
    ) {
        error!(error = %err, "failed to cache flowalyzed features");
        snapshot.error = Some(err.to_string());
        // Don't overwrite snapshot's variant - preserve current state
        snapshot.recipe_applying = false;
        let _ = updates.send(snapshot.clone());
    }

    fn handle_toggle_variant(
        &mut self,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
        variant: ClipVariant,
    ) {
        let current = snapshot.active_clip_variant; // Read from snapshot, not engine
        if current == variant {
            info!(?variant, "already on requested variant, no change needed");
            let _ = updates.send(snapshot.clone());
            return;
        }
        if variant == ClipVariant::Flowalyzed && !self.can_toggle_to_flowalyzed() {
            error!("cannot toggle to flowalyzed: no flowalyzed clip exists");
            snapshot.error =
                Some("No flowalyzed clip available. Apply a recipe first.".to_string());
            let _ = updates.send(snapshot.clone());
            return;
        }
        // Update engine's active_clip to match snapshot
        if let Err(err) = self.apply_variant_change(variant) {
            error!(error = %err, "failed to toggle variant");
            snapshot.error = Some(err.to_string());
            let _ = updates.send(snapshot.clone());
            return;
        }
        snapshot.active_clip_variant = variant;
        info!(?variant, "variant toggled successfully");
        let _ = updates.send(snapshot.clone());
    }

    fn can_toggle_to_flowalyzed(&self) -> bool {
        self.flowalyzed_clip.is_some()
    }

    fn apply_variant_change(&mut self, variant: ClipVariant) -> Result<()> {
        self.engine.set_active_clip(variant);
        Ok(())
    }

    fn handle_start(
        &mut self,
        commands: &Receiver<SessionCommand>,
        updates: &Sender<SessionSnapshot>,
        snapshot: &mut SessionSnapshot,
    ) -> LoopExit {
        info!("recording session starting");
        let mut start_update = match self.engine.start(snapshot) {
            Ok(update) => update,
            Err(err) => {
                error!(error = %err, "failed to start capture engine");
                emit_error(updates, snapshot, err.to_string());
                return LoopExit::Finished;
            }
        };
        // Preserve snapshot's variant and flowalyzed state, don't overwrite
        start_update.active_clip_variant = snapshot.active_clip_variant;
        start_update.has_flowalyzed_clip = snapshot.has_flowalyzed_clip;
        let _ = updates.send(start_update);
        info!("starting reference playback");
        // Get clip from snapshot variant (source of truth) and prepare samples
        let (stereo_samples, sample_rate) = match self.active_clip(snapshot.active_clip_variant) {
            Ok(clip) => (duplicate_to_stereo(&clip.samples), clip.sample_rate),
            Err(err) => {
                error!(error = %err, "failed to get clip for playback");
                self.engine.stop(snapshot);
                emit_error(updates, snapshot, err.to_string());
                return LoopExit::Finished;
            }
        };
        if let Err(err) = self.start_playback_with_samples(stereo_samples, sample_rate) {
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
                        let mut update = self.engine.stop(snapshot);
                        // Preserve snapshot's variant and flowalyzed state, don't overwrite
                        update.active_clip_variant = snapshot.active_clip_variant;
                        update.has_flowalyzed_clip = snapshot.has_flowalyzed_clip;
                        self.stop_playback();
                        let _ = updates.send(update);
                        return LoopExit::Shutdown;
                    }
                    SessionCommand::Stop => {
                        info!("stop command received");
                        let mut update = self.engine.stop(snapshot);
                        // Preserve snapshot's variant and flowalyzed state, don't overwrite
                        update.active_clip_variant = snapshot.active_clip_variant;
                        update.has_flowalyzed_clip = snapshot.has_flowalyzed_clip;
                        self.stop_playback();
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
                    SessionCommand::ApplyFlowalyzerRecipe {
                        range_start,
                        range_end,
                        recipe,
                    } => {
                        self.handle_apply_recipe(updates, snapshot, range_start, range_end, recipe);
                    }
                    SessionCommand::ToggleClipVariant { variant } => {
                        self.handle_toggle_variant(updates, snapshot, variant);
                    }
                }
            }
            match self.engine.poll(snapshot) {
                Ok(Some(mut update)) => {
                    // Preserve snapshot's variant and flowalyzed state, don't overwrite
                    update.active_clip_variant = snapshot.active_clip_variant;
                    update.has_flowalyzed_clip = snapshot.has_flowalyzed_clip;
                    let _ = updates.send(update);
                }
                Ok(None) => {}
                Err(err) => {
                    error!(error = %err, "capture engine error during poll");
                    self.engine.stop(snapshot);
                    self.stop_playback();
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

#[derive(Clone)]
enum SessionCommand {
    Start,
    Stop,
    ReplayReference,
    StopReplay,
    Shutdown,
    ApplyFlowalyzerRecipe {
        range_start: f64,
        range_end: f64,
        recipe: Recipe,
    },
    ToggleClipVariant {
        variant: ClipVariant,
    },
}
