# Stateless Processing Implementation Plan (PSP-Compliant)

## Overview
Rebuild the pronunciation session stack with stateless processing. The engine processes each chunk once, aligns it against the absolute reference timeline, and discards it. No learner history in the engine. UI owns all visualization history.

## Current State
- **Teardown complete**: All pronunciation subsystems removed except recipe helpers (`RecordedClip`, `apply_recipe_to_range`)
- **Existing infrastructure**:
  - `LiveCapture` in `src/audio/capture.rs` (streaming audio capture)
  - `cpal` for audio input
  - `aus` crate with `pyin` for pitch detection
  - Recipe helpers in `src/pronunciation/mod.rs`
- **UI file has broken imports**: `src/ui/screens/session.rs` references types that no longer exist

## Success Criteria
1. Engine never stores learner history between chunks
2. Snapshots carry only current chunk data
3. UI owns and manages all visualization history
4. Tests pass with 100% success rate
5. No dead code after each phase

---

## Phase 1: Stateless Core Data Model (Foundation)

### Code Style Checklist
- [ ] Functions <20 lines (preferably <10)
- [ ] Pure functions for data transformation
- [ ] Helper functions instead of nested logic
- [ ] No defensive coding
- [ ] No future-proofing
- [ ] Every function gets a test

### Subagent: kiss-code-generator

### Deliverables
1. `AlignmentReport` struct with chunk-only vectors
2. `SessionSnapshot` struct with session state flags
3. `PronunciationScores` struct
4. Supporting enums (`ClipVariant`, `RecipeApplicationProgress`)
5. Unit tests for all new types

### Files to Create
- `src/pronunciation/session/mod.rs` (module glue)
- `src/pronunciation/session/snapshot.rs` (data types)
- `tests/pronunciation_types.rs` (unit tests)

### Files to Modify
- `src/pronunciation/mod.rs` (add `pub mod session;` export)

### Implementation Steps

#### 1A. Create session module structure
```rust
// src/pronunciation/session/mod.rs
mod snapshot;

pub use snapshot::{
    AlignedPhoneme, AlignmentReport, ClipVariant,
    PronunciationScores, RecipeApplicationProgress,
    SessionSnapshot,
};
```

#### 1B. Define AlignmentReport (current chunk only)
```rust
// src/pronunciation/session/snapshot.rs
use std::time::Duration;

pub struct AlignmentReport {
    pub reference_energy: Vec<f32>,      // Current chunk only
    pub learner_energy: Vec<f32>,        // Current chunk only
    pub reference_pitch: Vec<f32>,       // Current chunk only
    pub learner_pitch: Vec<f32>,         // Current chunk only
    pub similarity_band: Vec<f32>,       // Current chunk only
    pub contour_band: Vec<f32>,          // Current chunk only
    pub phonemes: Vec<AlignedPhoneme>,   // Current chunk only
    pub total_duration: Duration,
    pub global_time_offset_ms: f32,      // Absolute offset since session start
    pub confidence: f32,
}
```

Key constraint: **No timeline slices, no history arrays. Each vector describes ONLY the current chunk.**

#### 1C. Define SessionSnapshot (one-frame view)
```rust
pub struct SessionSnapshot {
    pub alignment: AlignmentReport,
    pub scores: PronunciationScores,
    pub recording: bool,
    pub reference_playing: bool,
    pub active_clip_variant: ClipVariant,
    pub has_flowalyzed_clip: bool,
    pub recipe_state: Option<RecipeApplicationProgress>,
    pub error: Option<String>,
}
```

Key constraint: **No history vectors, no warming-up flags, no latency timelines.**

#### 1D. Define supporting types
```rust
pub struct AlignedPhoneme {
    pub symbol: String,
    pub timing_delta_ms: f32,
    pub similarity: f32,
    pub articulation_variance: f32,
    pub contour_similarity: f32,
}

pub struct PronunciationScores {
    pub overall: f32,
    pub timing: f32,
    pub articulation: f32,
    pub intonation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipVariant {
    Original,
    Flowalyzed,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeApplicationStage {
    ExtractingAudio,
    ApplyingRecipe,
    SavingResult,
}

impl RecipeApplicationStage {
    pub fn order(&self) -> usize {
        match self {
            Self::ExtractingAudio => 0,
            Self::ApplyingRecipe => 1,
            Self::SavingResult => 2,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ExtractingAudio => "Extracting audio",
            Self::ApplyingRecipe => "Applying recipe",
            Self::SavingResult => "Saving result",
        }
    }

    pub fn ordered() -> [Self; 3] {
        [Self::ExtractingAudio, Self::ApplyingRecipe, Self::SavingResult]
    }
}
```

#### 1E. Implement Default traits
```rust
impl Default for AlignmentReport {
    fn default() -> Self {
        Self {
            reference_energy: Vec::new(),
            learner_energy: Vec::new(),
            reference_pitch: Vec::new(),
            learner_pitch: Vec::new(),
            similarity_band: Vec::new(),
            contour_band: Vec::new(),
            phonemes: Vec::new(),
            total_duration: Duration::ZERO,
            global_time_offset_ms: 0.0,
            confidence: 0.0,
        }
    }
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            alignment: AlignmentReport::default(),
            scores: PronunciationScores::default(),
            recording: false,
            reference_playing: false,
            active_clip_variant: ClipVariant::Original,
            has_flowalyzed_clip: false,
            recipe_state: None,
            error: None,
        }
    }
}

impl Default for PronunciationScores {
    fn default() -> Self {
        Self {
            overall: 0.0,
            timing: 0.0,
            articulation: 0.0,
            intonation: 0.0,
        }
    }
}
```

#### 1F. Unit tests
```rust
// tests/pronunciation_types.rs
#[test]
fn alignment_report_default_has_empty_vectors() {
    let report = AlignmentReport::default();
    assert!(report.reference_energy.is_empty());
    assert!(report.learner_energy.is_empty());
    assert_eq!(report.global_time_offset_ms, 0.0);
}

#[test]
fn session_snapshot_default_not_recording() {
    let snapshot = SessionSnapshot::default();
    assert!(!snapshot.recording);
    assert!(!snapshot.reference_playing);
    assert!(snapshot.error.is_none());
}

#[test]
fn clip_variant_equality() {
    assert_eq!(ClipVariant::Original, ClipVariant::Original);
    assert_ne!(ClipVariant::Original, ClipVariant::Flowalyzed);
}

#[test]
fn recipe_stage_ordering() {
    assert_eq!(RecipeApplicationStage::ExtractingAudio.order(), 0);
    assert_eq!(RecipeApplicationStage::ApplyingRecipe.order(), 1);
    assert_eq!(RecipeApplicationStage::SavingResult.order(), 2);
}
```

### Phase End Verification
1. Run `cargo test` - expect 100% pass rate
2. Run `cargo clippy --all-targets --all-features` - fix ALL errors
3. Verify NO dead code - all types must be used in tests
4. Update status: "Phase 1 complete - data model established"
5. Commit: `git add -A && git commit -m "Phase 1 (stateless data model) complete"`

---

## Phase 2: Feature Extraction & Alignment (Stateless Engine Core)

### Code Style Checklist
- [ ] Functions <20 lines (preferably <10)
- [ ] Pure functions for feature extraction
- [ ] Helper functions for RMS, pitch computation
- [ ] No defensive coding
- [ ] No future-proofing
- [ ] Every function gets a test

### Subagent: modular-builder

### Deliverables
1. `FeatureExtractor` for energy and pitch extraction
2. `StatelessAligner` for chunk-reference alignment
3. Pure functions: `compute_rms`, `compute_pitch`, `align_features`
4. Unit tests for all feature extraction functions

### Files to Create
- `src/pronunciation/features/mod.rs` (module glue)
- `src/pronunciation/features/energy.rs` (RMS energy)
- `src/pronunciation/features/pitch.rs` (pitch using `aus`)
- `src/pronunciation/alignment/mod.rs` (alignment logic)
- `tests/feature_extraction.rs` (unit tests)
- `tests/alignment.rs` (unit tests)

### Files to Modify
- `src/pronunciation/mod.rs` (add `pub mod features; pub mod alignment;`)

### Implementation Steps

#### 2A. Energy extraction (pure function)
```rust
// src/pronunciation/features/energy.rs
const FRAME_SIZE_SAMPLES: usize = 160;  // 10ms at 16kHz
const HOP_SIZE_SAMPLES: usize = 160;    // 10ms hop

pub fn compute_energy_frames(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks(HOP_SIZE_SAMPLES)
        .map(compute_frame_rms)
        .collect()
}

fn compute_frame_rms(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = frame.iter().map(|s| s * s).sum();
    (sum_squares / frame.len() as f32).sqrt()
}
```

#### 2B. Pitch extraction (pure function using aus)
```rust
// src/pronunciation/features/pitch.rs
use aus::pyin;

const SAMPLE_RATE: u32 = 16_000;
const FRAME_HOP_MS: f32 = 10.0;

pub fn compute_pitch_frames(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let hop_samples = (SAMPLE_RATE as f32 * FRAME_HOP_MS / 1000.0) as usize;
    let config = pyin::PYINConfig::default();
    let pitches = pyin::pyin(samples, SAMPLE_RATE, &config);
    pitches.into_iter().step_by(hop_samples.max(1)).collect()
}
```

#### 2C. Stateless aligner (pure function)
```rust
// src/pronunciation/alignment/mod.rs
use crate::pronunciation::features::FeatureFrames;
use crate::pronunciation::session::AlignmentReport;
use std::time::Duration;

#[derive(Debug, Default, Clone, Copy)]
pub struct StatelessAligner;

impl StatelessAligner {
    pub fn new() -> Self {
        Self
    }

    pub fn align(
        &self,
        reference: &FeatureFrames,
        learner: &FeatureFrames,
        global_offset_ms: f32,
    ) -> AlignmentReport {
        align_features(reference, learner, global_offset_ms)
    }
}

pub fn align_features(
    reference: &FeatureFrames,
    learner: &FeatureFrames,
    global_offset_ms: f32,
) -> AlignmentReport {
    let similarity = compute_similarity(&reference.energy, &learner.energy);
    let contour = compute_contour_similarity(&reference.pitch, &learner.pitch);
    let confidence = compute_confidence(&similarity, &contour);

    AlignmentReport {
        reference_energy: reference.energy.clone(),
        learner_energy: learner.energy.clone(),
        reference_pitch: reference.pitch.clone(),
        learner_pitch: learner.pitch.clone(),
        similarity_band: similarity,
        contour_band: contour,
        phonemes: Vec::new(),  // Phase 3 will add phoneme detection
        total_duration: Duration::from_millis(learner.energy.len() as u64 * 10),
        global_time_offset_ms: global_offset_ms,
        confidence,
    }
}

fn compute_similarity(reference: &[f32], learner: &[f32]) -> Vec<f32> {
    let len = reference.len().min(learner.len());
    (0..len)
        .map(|i| frame_similarity(reference[i], learner[i]))
        .collect()
}

fn frame_similarity(ref_val: f32, learner_val: f32) -> f32 {
    let diff = (ref_val - learner_val).abs();
    let max_val = ref_val.max(learner_val).max(0.001);
    (1.0 - diff / max_val).clamp(0.0, 1.0)
}

fn compute_contour_similarity(ref_pitch: &[f32], learner_pitch: &[f32]) -> Vec<f32> {
    let len = ref_pitch.len().min(learner_pitch.len());
    (0..len)
        .map(|i| pitch_similarity(ref_pitch[i], learner_pitch[i]))
        .collect()
}

fn pitch_similarity(ref_hz: f32, learner_hz: f32) -> f32 {
    if ref_hz == 0.0 && learner_hz == 0.0 {
        return 1.0;
    }
    if ref_hz == 0.0 || learner_hz == 0.0 {
        return 0.0;
    }
    let ratio = (ref_hz / learner_hz).min(learner_hz / ref_hz);
    ratio.clamp(0.0, 1.0)
}

fn compute_confidence(similarity: &[f32], contour: &[f32]) -> f32 {
    let sim_avg = mean(similarity);
    let contour_avg = mean(contour);
    (sim_avg + contour_avg) / 2.0
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}
```

#### 2D. Module glue
```rust
// src/pronunciation/features/mod.rs
mod energy;
mod pitch;

pub use energy::compute_energy_frames;
pub use pitch::compute_pitch_frames;

#[derive(Debug, Clone, Default)]
pub struct FeatureFrames {
    pub energy: Vec<f32>,
    pub pitch: Vec<f32>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FeatureExtractor;

impl FeatureExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract(&self, samples: &[f32]) -> FeatureFrames {
        FeatureFrames {
            energy: compute_energy_frames(samples),
            pitch: compute_pitch_frames(samples),
        }
    }
}
```

#### 2E. Unit tests for feature extraction
```rust
// tests/feature_extraction.rs
use flowalyzer::pronunciation::features::{compute_energy_frames, compute_pitch_frames};

#[test]
fn energy_frames_from_silence() {
    let samples = vec![0.0; 320];  // 20ms at 16kHz
    let frames = compute_energy_frames(&samples);
    assert_eq!(frames.len(), 2);
    assert!(frames.iter().all(|&v| v < 0.001));
}

#[test]
fn energy_frames_from_tone() {
    let samples: Vec<f32> = (0..320).map(|i| (i as f32 * 0.1).sin()).collect();
    let frames = compute_energy_frames(&samples);
    assert_eq!(frames.len(), 2);
    assert!(frames.iter().all(|&v| v > 0.0));
}

#[test]
fn pitch_frames_empty_input() {
    let frames = compute_pitch_frames(&[]);
    assert!(frames.is_empty());
}
```

#### 2F. Unit tests for alignment
```rust
// tests/alignment.rs
use flowalyzer::pronunciation::alignment::align_chunk;

#[test]
fn align_identical_chunks_high_similarity() {
    let energy = vec![0.5, 0.6, 0.7];
    let pitch = vec![200.0, 210.0, 220.0];
    let report = align_chunk(&energy, &energy, &pitch, &pitch, 0.0);
    assert_eq!(report.similarity_band.len(), 3);
    assert!(report.similarity_band.iter().all(|&v| v > 0.99));
    assert!(report.confidence > 0.99);
}

#[test]
fn align_different_chunks_lower_similarity() {
    let ref_energy = vec![0.5, 0.6, 0.7];
    let learner_energy = vec![0.1, 0.2, 0.3];
    let pitch = vec![200.0, 210.0, 220.0];
    let report = align_chunk(&ref_energy, &learner_energy, &pitch, &pitch, 0.0);
    assert!(report.confidence < 0.8);
}

#[test]
fn global_offset_preserved() {
    let energy = vec![0.5];
    let pitch = vec![200.0];
    let report = align_chunk(&energy, &energy, &pitch, &pitch, 1500.0);
    assert_eq!(report.global_time_offset_ms, 1500.0);
}
```

### Phase End Verification
1. Run `cargo test` - expect 100% pass rate
2. Run `cargo clippy --all-targets --all-features` - fix ALL errors
3. Verify NO dead code - all functions used in tests
4. Update status: "Phase 2 complete - feature extraction and alignment"
5. Commit: `git add -A && git commit -m "Phase 2 (feature extraction and alignment) complete"`

---

## Phase 3: Session Engine & Runtime (Stateless Processing Loop)

### Code Style Checklist
- [ ] Functions <20 lines (preferably <10)
- [ ] Pure functions for processing logic
- [ ] Helper functions for chunk handling
- [ ] No defensive coding
- [ ] No future-proofing
- [ ] Every function gets a test

### Subagent: modular-builder

### Deliverables
1. `SessionEngine` that processes chunks statelessly
2. `SessionRuntime` that manages engine lifecycle
3. `SessionController` for start/stop commands
4. `SessionHandle` for UI communication
5. Integration tests for runtime lifecycle

### Files to Create
- `src/pronunciation/session/engine.rs` (processing loop)
- `src/pronunciation/session/runtime.rs` (controller and handle)
- `src/pronunciation/session/config.rs` (session configuration)
- `tests/stateless_runtime.rs` (integration tests)

### Files to Modify
- `src/pronunciation/session/mod.rs` (add module exports)

### Implementation Steps

#### 3A. Session configuration
```rust
// src/pronunciation/session/config.rs
use std::ops::RangeInclusive;

pub struct SessionConfig {
    pub sample_rate: u32,
    pub chunk_duration_ms: u32,
    pub latency_budget_ms: u32,
    pub latency_range: RangeInclusive<u32>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            chunk_duration_ms: 100,
            latency_budget_ms: 200,
            latency_range: 100..=200,
        }
    }
}
```

#### 3B. Session engine (stateless processor)
```rust
// src/pronunciation/session/engine.rs
use crate::audio::capture::LiveCapture;
use crate::pronunciation::alignment::align_chunk;
use crate::pronunciation::features::{compute_energy_frames, compute_pitch_frames};
use crate::pronunciation::session::{AlignmentReport, SessionConfig};

pub struct SessionEngine {
    reference_energy: Vec<f32>,
    reference_pitch: Vec<f32>,
    global_sample_counter: u64,
    chunk_samples: usize,
    sample_rate: u32,
}

impl SessionEngine {
    pub fn new(reference_samples: &[f32], config: &SessionConfig) -> Self {
        let reference_energy = compute_energy_frames(reference_samples);
        let reference_pitch = compute_pitch_frames(reference_samples);
        let chunk_samples = (config.sample_rate * config.chunk_duration_ms / 1000) as usize;

        Self {
            reference_energy,
            reference_pitch,
            global_sample_counter: 0,
            chunk_samples,
            sample_rate: config.sample_rate,
        }
    }

    pub fn process_chunk(&mut self, learner_samples: &[f32]) -> AlignmentReport {
        let learner_energy = compute_energy_frames(learner_samples);
        let learner_pitch = compute_pitch_frames(learner_samples);

        let ref_slice_start = self.reference_frame_index();
        let ref_slice_end = (ref_slice_start + learner_energy.len()).min(self.reference_energy.len());

        let ref_energy_slice = &self.reference_energy[ref_slice_start..ref_slice_end];
        let ref_pitch_slice = &self.reference_pitch[ref_slice_start..ref_slice_end];

        let global_offset_ms = self.global_offset_ms();

        let report = align_chunk(
            ref_energy_slice,
            &learner_energy,
            ref_pitch_slice,
            &learner_pitch,
            global_offset_ms,
        );

        self.advance_counter(learner_samples.len());
        report
    }

    fn reference_frame_index(&self) -> usize {
        let frame_hop_samples = self.sample_rate as usize / 100;  // 10ms hop
        (self.global_sample_counter as usize) / frame_hop_samples
    }

    fn global_offset_ms(&self) -> f32 {
        (self.global_sample_counter as f32 / self.sample_rate as f32) * 1000.0
    }

    fn advance_counter(&mut self, samples_processed: usize) {
        self.global_sample_counter += samples_processed as u64;
    }

    pub fn reset(&mut self) {
        self.global_sample_counter = 0;
    }
}
```

Key invariant: **Engine stores only reference features + sample counter. No learner history.**

#### 3C. Session runtime and controller
```rust
// src/pronunciation/session/runtime.rs
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio::capture::{CaptureConfig, LiveCapture};
use crate::pronunciation::{PronunciationError, RecordedClip, Result};
use crate::pronunciation::session::{
    PronunciationScores, SessionConfig, SessionEngine, SessionSnapshot, ClipVariant,
};

pub enum SessionCommand {
    Start,
    Stop,
    ReplayReference,
    StopReplay,
    Shutdown,
}

pub struct SessionRuntime {
    reference_clip: Arc<RecordedClip>,
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
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        let engine = SessionEngine::new(&reference_clip.samples, &config);

        let runtime = Self {
            reference_clip: Arc::new(reference_clip),
            engine: Arc::new(Mutex::new(engine)),
            config: config.clone(),
            snapshot_sender: snapshot_tx,
            command_receiver: command_rx,
        };

        let handle = SessionHandle {
            snapshot_receiver: snapshot_rx,
            config,
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
                    capture = self.start_capture();
                    self.engine.lock().unwrap().reset();
                }
                Ok(SessionCommand::Stop) => {
                    recording = false;
                    capture = None;
                }
                Ok(SessionCommand::Shutdown) => break,
                Ok(_) => {}
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
    }

    fn start_capture(&self) -> Option<LiveCapture> {
        let config = CaptureConfig {
            device_name: None,
            sample_rate: self.config.sample_rate,
            latency_ms: self.config.latency_range.clone(),
            duration: Duration::from_secs(3600),
        };
        LiveCapture::start(&config).ok()
    }

    fn process_capture_chunk(&self, capture: &LiveCapture) {
        let timeout = Duration::from_millis(self.config.chunk_duration_ms as u64);
        if let Some(samples) = capture.recv_chunk(timeout) {
            let report = self.engine.lock().unwrap().process_chunk(&samples);
            let snapshot = self.build_snapshot(report);
            let _ = self.snapshot_sender.send(snapshot);
        }
    }

    fn build_snapshot(&self, alignment: super::AlignmentReport) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            scores: PronunciationScores::default(),
            recording: true,
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
        while let Ok(snap) = self.snapshot_receiver.try_recv() {
            snapshots.push(snap);
        }
        snapshots
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    pub fn initial_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot::default()
    }

    pub fn controller(&self) -> SessionController {
        unimplemented!("controller cloning not yet supported")
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

    pub fn toggle_clip_variant(&self, _variant: ClipVariant) -> Result<()> {
        Ok(())  // Stub for Phase 4
    }

    pub fn apply_recipe(&self, _start: f64, _end: f64, _recipe: crate::types::Recipe) -> Result<()> {
        Ok(())  // Stub for Phase 4
    }
}
```

#### 3D. Module exports
```rust
// src/pronunciation/session/mod.rs
mod config;
mod engine;
mod runtime;
mod snapshot;

pub use config::SessionConfig;
pub use engine::SessionEngine;
pub use runtime::{SessionController, SessionHandle, SessionRuntime};
pub use snapshot::{
    AlignedPhoneme, AlignmentReport, ClipVariant, PronunciationScores,
    RecipeApplicationProgress, RecipeApplicationStage, SessionSnapshot,
};
```

#### 3E. Integration tests
```rust
// tests/stateless_runtime.rs
use std::time::Duration;
use flowalyzer::pronunciation::{RecordedClip, SessionConfig, SessionRuntime};

#[test]
fn runtime_emits_initial_snapshot_on_spawn() {
    let clip = RecordedClip::from_samples(vec![0.0; 16000], 16000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let snapshots = handle.drain_snapshots();
    // Initial snapshot arrives even before capture starts
    assert!(!snapshots.is_empty());
}

#[test]
fn start_stop_cycle_runs_cleanly() {
    let clip = RecordedClip::from_samples(vec![0.0; 16000], 16000);
    let config = SessionConfig::default();
    let (_handle, controller) = SessionRuntime::spawn(clip, config);

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    controller.stop().unwrap();
    std::thread::sleep(Duration::from_millis(20));

    controller.start().unwrap();
    std::thread::sleep(Duration::from_millis(20));
    controller.stop().unwrap();
}
```

### Phase End Verification
1. Run `cargo test` - expect 100% pass rate
2. Run `cargo clippy --all-targets --all-features` - fix ALL errors
3. Verify NO dead code
4. Update status: "Phase 3 complete - session engine and runtime"
5. Commit: `git add -A && git commit -m "Phase 3 (session engine and runtime) complete"`

### Phase 3 Course Correction

Plan to align the implementation with the spec before proceeding:

**Subphase A: Align SessionEngine with spec**
- Precompute and store reference feature frames (energy, pitch) instead of raw samples.
- Remove configurable chunk-memory limits; enforce only the minimal boundary buffer (1–2 chunks) strictly for overlap/fade at boundaries.
- Drop learner-history timelines; keep only the global sample counter plus the optional pending boundary chunk.
- Keep processing small/pure: process a chunk, compute features, align, advance the counter, return the report.

**Subphase B: Simplify SessionConfig and enforce invariants**
- Reduce config to Phase 3 fields (sample_rate, chunk_duration_ms, latency_budget_ms, latency_range); if boundary buffering is needed, keep it fixed (1–2 chunks) per invariant, not user-configurable.
- Enforce correct-by-construction: runtime spawn requires a valid RecordedClip reference; fail fast otherwise.

**Subphase C: Trim SessionRuntime to Phase 3 scope**
- Strip or isolate Phase 4/5 features (recipe application, clip variant toggling, audio playback) from the Phase 3 runtime path.
- Ensure start/stop only gates capture; runtime exists immediately after spawn and emits an initial snapshot.
- Keep snapshot construction stateless and per-chunk; no accumulation of histories.

**Subphase D: Align tests and verification**
- Update `tests/stateless_runtime.rs` to match lifecycle intent (spawn yields initial snapshot; start/stop gates capture).
- Adjust Phase 3 doc snippets to reflect corrected behavior.
- Run `cargo test --all` and `cargo clippy --all`; require 100% pass and zero warnings.

---

## Phase 4: UI Integration (History Ownership)

### Code Style Checklist
- [ ] Functions <20 lines (preferably <10)
- [ ] Pure functions for history trimming
- [ ] Helper functions for buffer management
- [ ] No defensive coding
- [ ] No future-proofing
- [ ] Every function gets a test

### Subagent: modular-builder

### Deliverables
1. Fix UI imports to use new session types
2. VecDeque-based history buffers in SessionApp
3. `append_chunk` and `trim_to_window` helpers
4. `clear_histories` on session restart
5. UI tests for history management

### Files to Modify
- `src/ui/screens/session.rs` (fix imports, add history logic)
- `src/pronunciation/mod.rs` (export session types)

### Files to Delete (if any dead code remains)
- None expected

### Implementation Steps

#### 4A. Export session types from pronunciation module
```rust
// src/pronunciation/mod.rs (add at end)
pub mod alignment;
pub mod features;
pub mod session;

pub use session::{
    AlignedPhoneme, AlignmentReport, ClipVariant, PronunciationScores,
    RecipeApplicationProgress, RecipeApplicationStage, SessionConfig,
    SessionController, SessionHandle, SessionRuntime, SessionSnapshot,
};
```

#### 4B. Fix UI imports
Replace the broken imports in `src/ui/screens/session.rs` with the new types.

Remove these imports:
```rust
// REMOVE
use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, ClipVariant, InitializationStage, RecipeApplicationStage,
    Result as SessionResult, SessionController, SessionHandle, SessionSnapshot,
    ALIGNMENT_TIMELINE_CAPACITY_FRAMES,
};
```

Add these imports:
```rust
use crate::pronunciation::{
    AlignedPhoneme, AlignmentReport, ClipVariant, RecipeApplicationStage,
    Result as SessionResult, SessionConfig, SessionController, SessionHandle,
    SessionSnapshot,
};
```

#### 4C. Remove legacy constants and flags
Remove references to:
- `ALIGNMENT_TIMELINE_CAPACITY_FRAMES` (use inline constant)
- `InitializationStage` (no longer exists)
- `warming_up` fields (not in new spec)
- `initializing` fields (not in new spec)
- `latency_ms` fields (not in new spec)
- `warmup_remaining_ms` (not in new spec)
- `init_progress` (not in new spec)
- `recipe_applying` (use recipe_state instead)
- `recipe_progress` (use recipe_state instead)

#### 4D. Convert Vec history to bounded behavior
The current `Vec<f32>` history buffers already have trimming logic. Keep the logic but ensure it aligns with spec:
- History window: 30 seconds (3000 frames at 10ms hop)
- Clear on session restart
- Clear on clip variant toggle

#### 4E. Simplify SessionApp fields
Remove fields that don't exist in new spec:
- `latency_budget_ms` (config has this)
- Fields related to warming up, initialization progress

Keep essential fields:
- `handle`, `controller`, `snapshot`
- `control_error`, `selected_phoneme`
- `reference_waveform`, `learner_waveform`, etc.
- All history buffers
- Recipe builder state

#### 4F. Update `show_top_panel` to remove legacy UI
Remove UI elements for:
- Initialization progress
- Warm-up status
- Latency guidance (simplify or remove)

Keep:
- Recording toggle
- Reference replay
- Clip variant toggle
- Scores display
- Error banners
- Recipe status

#### 4G. Tests for history management (already exist)
The existing tests in `src/ui/screens/session.rs` cover history behavior. Ensure they pass.

### Phase End Verification
1. Run `cargo test` - expect 100% pass rate
2. Run `cargo clippy --all-targets --all-features` - fix ALL errors
3. Verify NO dead code - remove all unused imports and functions
4. Verify UI compiles and displays correctly
5. Update status: "Phase 4 complete - UI integration"
6. Commit: `git add -A && git commit -m "Phase 4 (UI integration) complete"`

---

## Phase 5: Recipe & Clip Variant Integration

### Code Style Checklist
- [ ] Functions <20 lines (preferably <10)
- [ ] Pure functions for recipe application
- [ ] Helper functions for clip management
- [ ] No defensive coding
- [ ] No future-proofing
- [ ] Every function gets a test

### Subagent: modular-builder

### Deliverables
1. Flowalyzed clip caching in runtime
2. Clip variant toggling
3. Recipe application through controller
4. Integration tests for recipes

### Files to Modify
- `src/pronunciation/session/runtime.rs` (add recipe commands)
- `tests/recipe_application.rs` (integration tests)

### Implementation Steps

#### 5A. Add flowalyzed clip storage to runtime
```rust
// In SessionRuntime struct
flowalyzed_clip: Arc<Mutex<Option<RecordedClip>>>,
active_variant: Arc<Mutex<ClipVariant>>,
```

#### 5B. Implement recipe application command
```rust
pub enum SessionCommand {
    // ... existing
    ApplyRecipe { start_sec: f64, end_sec: f64, recipe: Recipe },
    ToggleClipVariant(ClipVariant),
}
```

#### 5C. Handle recipe in runtime loop
```rust
Ok(SessionCommand::ApplyRecipe { start_sec, end_sec, recipe }) => {
    match apply_recipe_to_clip(&self.reference_clip, start_sec, end_sec, &recipe) {
        Ok(flowalyzed) => {
            *self.flowalyzed_clip.lock().unwrap() = Some(flowalyzed);
            // Send success snapshot
        }
        Err(e) => {
            // Send error snapshot
        }
    }
}
```

#### 5D. Integration tests
```rust
#[test]
fn apply_recipe_creates_flowalyzed_clip() {
    let clip = RecordedClip::from_samples(vec![0.5; 32000], 16000);
    let config = SessionConfig::default();
    let (handle, controller) = SessionRuntime::spawn(clip, config);

    let recipe = Recipe { /* ... */ };
    controller.apply_recipe(0.0, 1.0, recipe).unwrap();

    std::thread::sleep(Duration::from_millis(100));
    let snapshots = handle.drain_snapshots();
    // Should have snapshot indicating flowalyzed clip exists
}
```

### Phase End Verification
1. Run `cargo test` - expect 100% pass rate
2. Run `cargo clippy --all-targets --all-features` - fix ALL errors
3. Verify NO dead code
4. Update status: "Phase 5 complete - recipe integration"
5. Commit: `git add -A && git commit -m "Phase 5 (recipe integration) complete"`

---

## Final Verification

After all phases complete:

1. **Full test suite**: `cargo test --all` - 100% pass
2. **Clippy clean**: `cargo clippy --all-targets --all-features` - no warnings
3. **No dead code**: No unused imports, functions, or types
4. **Documentation**: Update `docs/stateless_runtime_spec.md` with actual performance characteristics
5. **Manual test**: Start session, speak, verify waveforms align, stop and restart, verify histories clear

## Rejected Approaches (DO NOT IMPLEMENT)

1. **Storing learner history in engine** - Violates stateless contract
2. **Timeline fields in snapshots** - Creates hidden state
3. **Warm-up flags or initialization stages** - Unnecessary complexity
4. **Future-proofing with optional fields** - Write for current spec only
5. **Generic abstractions over concrete types** - Keep it simple

## Key Invariants

1. **Engine holds NO learner history** - Only reference features + sample counter; a single pending/previous chunk is allowed strictly to complete boundary processing (fade/overlap) and must never accumulate into timelines. At most two chunks may be held when the end boundary needs the same treatment.
2. **Snapshots are one-frame views** - Current chunk data only
3. **UI owns ALL visualization history** - VecDeque buffers in SessionApp
4. **Histories clear on restart** - Session restart = fresh start
5. **No hidden state** - If it's not in the spec, it doesn't exist
