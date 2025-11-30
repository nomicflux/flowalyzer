# Pronunciation Binary Code Map (function-level)

This map traces the pronunciation binary end-to-end, noting call sites, test-only hooks, and dead/duplicated code. Scope: current `src/bin/pronunciation.rs` pipeline and the modules it exercises.

## Entry & UI
- `src/bin/pronunciation.rs`
  - `main`: parses CLI args, loads reference via `pronunciation::load_clip`, builds `SessionConfig`, spawns `SessionRuntime`, and launches `SessionApp`.
- `src/ui/screens/session.rs` (egui UI)
  - `SessionApp::new(handle, controller)`: seeds local state from `SessionHandle::initial_snapshot`.
  - `SessionApp::update`: polls snapshots (`SessionHandle::drain_snapshots`), renders status + plots, sends Start/Stop/Replay/StopReplay via `SessionController`.
  - `show_top_panel`, `show_status`, `show_visualizations`: UI helpers only.
  - Drop impl calls `SessionController::shutdown`.
  - Tests (`dummy_app` helpers, history trimming) cover UI state only.

## Session Runtime Thread (`src/pronunciation/session/runtime.rs`)
- Construction
  - `SessionRuntime::create_engine`: builds `FeatureConfig`, `FeatureExtractor`, `ReferenceFeatures`, `SessionEngine`. Called by `spawn_with_capture_builder` and tests.
  - `spawn`/`spawn_with_capture_builder`: wires channels, constructs runtime, spawns thread, sends initial empty snapshot, returns `SessionHandle`/`SessionController`. Join handle stored in `SessionHandle::join` (used in tests).
- Command loop (`run`)
  - Processes `SessionCommand::{Start,Stop,ReplayReference,StopReplay,Shutdown}` only.
  - `start_capture`: unwraps builder start; capture errors panic in the thread (tested via `runtime_panics_on_capture_failure`).
  - `process_capture_chunk`: drain capture into buffer, resample (`collect_resampled_chunk_from_buffer`), seed tail once, then align chunks via `SessionEngine::process_chunk`; emit snapshots.
  - `start_reference_playback`/`stop_reference_playback`/`poll_playback_completion`: rodio playback and snapshot emission of last alignment.
  - `snapshot_for`/`snapshot_from_last`/`snapshot_internal`: snapshot assembly; only populated from real `AlignmentReport`.
- Helpers
  - `required_tail_len`: frame_len - hop derived per `FeatureConfig`.
  - Resample helpers: `required_raw_samples`, `collect_resampled_chunk` (test-only), `collect_resampled_chunk_from_buffer` (runtime + tests). No guards; insufficient data returns `Ok(None)` and drops work silently (note for future tightening).
- SessionHandle / SessionController API
  - `SessionHandle::{drain_snapshots, config, initial_snapshot, join}`; `join` used only in tests to observe thread panic.
  - `SessionController::{start, stop, replay_reference, stop_replay, shutdown}` send commands over mpsc.

## Session Engine (`src/pronunciation/session/engine.rs`)
- State: reference features, tail buffer, global sample counter, sample rate, feature config/extractor.
- Functions:
  - `new`: seeds extractor/config/reference features.
  - `seed_tail`: copies last `frame_len - hop` samples; sets global counter to input len.
  - `process_chunk`: builds tail+chunk window, extracts chunk features, computes start_frame_idx/global offset, aligns via `align_features`, updates tail/counter.
  - `reset`: clears tail/counter (used on Start).
  - Accessors: `sample_rate`, `required_tail_len`, `tail_samples`, `global_sample_counter`, `global_offset_ms` (private). All used in runtime or tests.

## Alignment (`src/pronunciation/alignment/mod.rs`)
- Public:
  - `align_features`: slices reference by `start_frame_idx`, computes metrics, builds `AlignmentReport`. Used by `SessionEngine`.
  - `StatelessAligner::align`: thin wrapper used nowhere else (dead alias; same behavior as `align_features`).
- Metrics:
  - `compute_energy_error`: learner - reference per frame.
  - `compute_similarity`: `-(|ln(energy_ratio)| + |ln(pitch_ratio)|)`; more negative = worse. Consumed by UI plots.
  - `compute_contour_band`: 1200 * log2(pitch ratio).
  - `hop_ms`/`total_duration` derived in `align_features`.
  - All helpers are used; no guards.

## Feature Extraction (`src/pronunciation/features/mod.rs`)
- Config:
  - `FeatureConfig::from_sample_rate`: scales frame_len/hop from 16k constants. Used across engine/tests.
- Extractor:
  - `FeatureExtractor::{new, extract_reference, extract_chunk}`:
    - `extract_reference`: frames from reference samples on hop grid; panics on short input.
    - `extract_chunk`: tail+chunk window, preserves hop phase, maps absolute starts back to chunk indices (drops negatives).
  - `impl Default for FeatureExtractor`: unused alias for `new` (dead helper).
- Math:
  - `scaled_samples` (used by config), `frame_energy`, `frame_pitch` (autocorrelation), `autocorrelation` (all used).

## Session Types (`src/pronunciation/session/snapshot.rs`, `config.rs`, `mod.rs`, `pronunciation/mod.rs`)
- `SessionSnapshot`: `AlignmentReport` + `recording` + `reference_playing`. Produced only from real alignment data.
- `AlignmentReport`: per-chunk vectors and indices; used throughout runtime/UI/tests.
- `SessionConfig`: defaults for sample/capture rates, chunk duration, latency range. Used by CLI and tests.
- `mod.rs` re-exports: `AlignmentReport`, `SessionSnapshot`, `SessionConfig`, `SessionEngine`, `SessionRuntime`, `SessionHandle`, `SessionController`, `SessionCommand`.
- `PronunciationError`, `RecordedClip`, `load_clip` in `src/pronunciation/mod.rs` used by CLI/runtime tests.

## Pronunciation Binary Call Flow Summary
1. `main` (bin) → load reference → `SessionRuntime::spawn`.
2. Runtime thread sends initial empty snapshot → UI seeds state.
3. UI sends `SessionCommand::Start` → runtime starts capture, seeds tail when enough data, processes chunks via `SessionEngine`.
4. `SessionEngine::process_chunk` → `FeatureExtractor::extract_chunk` → `align_features` → snapshots emitted to UI.
5. Replay commands manage playback; Stop/Shutdown halt processing.

## Tests & Coverage Notes
- Runtime tests (`tests/stateless_runtime.rs`): start/stop cycles, capture-driven processing, panic on capture failure, tail seeding before first snapshot, natural stop when reference exhausted. Some panic tests still operate on helpers (`capture_builder_error_panics`, `resample_with_zero_rate_panics`).
- Engine tests: tail seeding, hop-phase, counter/offset math, panic on short reference or missing seed.
- Alignment tests: metric values and panic on bad shapes.
- Feature tests: hop-phase/tail contribution, panics on short inputs.

## Dead / Test-Only Code
- Dead/unused: `FeatureExtractor::default` (alias for `new`), `StatelessAligner` wrapper (no call sites), `FeatureConfig::default` is absent by design; no other defaults present.
- Test-only helpers: `collect_resampled_chunk`, `BufferCapture*` builders in `tests/stateless_runtime.rs`, sine wave generators in tests.
