# Stateless Pronunciation Runtime – Clean-Room Specification

## Intent

Rebuild the pronunciation session stack from scratch with ruthless simplicity:

* **Engine is stateless.** Each capture chunk is processed once, aligned directly against the absolute reference timeline, and then discarded. The engine never stores prior learner data, never replays a historical vector, and never exposes “timeline” fields.
* **Snapshots are one-frame views.** A `SessionSnapshot` carries the most recent `AlignmentReport`, session flags (recording / clip variant / recipe state), and scores. Histories live exclusively in the UI.
* **UI owns visualization history.** Waveform/pitch/spectrogram buffers are purely UI concerns. Restarting a session or swapping clips clears the UI buffers; the engine doesn’t know they exist.
* **Impossibility by construction.** Public types and module boundaries do not permit timeline state to hide inside the runtime. Any attempt to reintroduce history requires changing the documented contract, catching regressions at review time instead of at runtime.

This specification supersedes all previous incremental “cleanups”. Implementation must begin with a teardown of the existing `pronunciation::session` tree and UI history plumbing before any new code is introduced.

## Module Overview

```
pronunciation/
├── alignment/
│   └── mod.rs            # StatelessAligner + geometry helpers
├── session/
│   ├── engine.rs         # SessionEngine + CaptureSource abstraction
│   ├── runtime.rs        # SessionRuntime, SessionHandle, SessionController
│   ├── snapshot.rs       # AlignmentReport, SessionSnapshot, SessionScores
│   └── mod.rs            # module glue + public exports
└── ui/
    └── screens/session.rs # UI histories + visualization logic
```

New submodules (`runtime.rs`, `snapshot.rs`) separate responsibilities so the compiler enforces the stateless contract.

## Data Model

### AlignmentReport

```rust
pub struct AlignmentReport {
    pub reference_energy: Vec<f32>,
    pub learner_energy: Vec<f32>,
    pub reference_pitch: Vec<f32>,
    pub learner_pitch: Vec<f32>,
    pub similarity_band: Vec<f32>,
    pub contour_band: Vec<f32>,
    pub phonemes: Vec<AlignedPhoneme>,
    pub total_duration: Duration,
    pub global_time_offset_ms: f32,
    pub confidence: f32,
}
```

* No timeline slices, no “history” arrays. Each vector describes **only the current chunk**.
* `global_time_offset_ms` is the absolute offset for the first frame of this chunk (since session start). Downstream consumers compute positions relative to this base.

### SessionSnapshot

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

There are **no** history vectors, warming-up flags, or latency timelines. If new metadata is required in the future, document the invariant before adding fields.

### UI Histories

`SessionApp` introduces five rolling buffers:

* `reference_energy_history`
* `learner_energy_history`
* `reference_pitch_history`
* `learner_pitch_history`
* `similarity_history` / `contour_history`

They are `VecDeque<f32>`s bounded to the configured visualization window (default 30 s). When a session restarts or the user clears the UI, the buffers reset to empty.

No other module is allowed to store these histories. Keep the fields `pub(crate)` or private to the UI module.

## Engine Responsibilities

1. **Capture:** Read a chunk from the active `CaptureSource`, resample to 16 kHz if needed, and clamp to the processing window length (`MAX_PROCESSING_WINDOW_MS`).
2. **Feature extraction:** Convert the chunk into per-frame energy + pitch. Only the learner chunk is processed; reference features are precomputed.
3. **Alignment:** Use `StatelessAligner` to align the chunk’s features against the precomputed reference features. Alignment uses the absolute sample counter to determine which reference slice to compare against.
4. **Snapshot:** Build an `AlignmentReport` + scores, populate a `SessionSnapshot`, and emit it immediately via the runtime channel.
5. **Reset:** Clear the learner chunk buffers and advance the absolute sample counter. The engine holds no other learner state between iterations.

Error handling: any failure (capture timeout, feature extraction, etc.) populates `SessionSnapshot.error` for the UI.

## Runtime & Controller

* `SessionRuntime` owns:
  * The reference clip (original + optional flowalyzed)
  * The `SessionEngine`
  * A bounded channel (e.g., `std::sync::mpsc::sync_channel`) for snapshots
  * The command channel for `SessionController`
* `SessionController` exposes: `start`, `stop`, `replay_reference`, `stop_replay`, `apply_recipe`, `toggle_clip_variant`.
* `SessionHandle` provides `try_recv()` and `drain_snapshots()` to the UI. It never mutates snapshots; it only forwards them.

Commands are processed serially on the runtime thread. The runtime loops:

1. Drain commands.
2. Poll the engine if recording.
3. Sleep/poll again.

No hidden timers or warm-up states unless explicitly reintroduced in the spec.

## UI History Handling

`SessionApp::accumulate_histories` appends the latest chunk vectors into the rolling buffers:

```rust
const HISTORY_WINDOW_MS: u32 = 30_000;

fn accumulate_histories(&mut self, report: &AlignmentReport) {
    self.reference_energy_history.push_chunk(&report.reference_energy);
    self.learner_energy_history.push_chunk(&report.learner_energy);
    // same for pitch/similarity/contour
    self.trim_histories();
}
```

`trim_histories` ensures the total number of frames matches `HISTORY_WINDOW_MS / FRAME_HOP_MS`. Restarting the session or resetting the UI calls `clear_histories()`.

Visualization widgets always read from the histories; they never access `snapshot.alignment` directly once the buffers are seeded.

## Architectural Guardrails

* **Compile-time:** keep history helpers inside `ui::screens::session`; do not re-export them. Running `cargo doc` should reveal no `*_history` fields outside `SessionApp`.
* **Runtime:** provide debug assertions in the engine to ensure `processed_samples` increases monotonically and chunk lengths stay within the configured window.
* **Documentation:** update `README.md` / `docs/` references to highlight the stateless contract.

## Implementation Plan (High-Level)

1. Delete `src/pronunciation/session/*` and `SessionSnapshot` definitions from `pronunciation::mod.rs`.
2. Create new `session` submodules (`snapshot.rs`, `engine.rs`, `runtime.rs`) matching this spec.
3. Rewire the CLI (`run_session`) and UI to the new runtime.
4. Move UI histories into `SessionApp` exactly as described.
5. After the rebuild, add integration tests that exercise the new runtime through `SessionHandle`.

Hold implementation until this spec is approved. Once it is, any deviations must be explicitly documented and reviewed. 
