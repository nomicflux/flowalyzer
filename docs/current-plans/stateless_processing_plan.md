# Stateless Processing & UI History Plan

## Current Status
- **Teardown complete.** Every pronunciation subsystem that was not part of the recipe workflow has been removed from the repository. Only the recipe helpers (`RecordedClip`, `apply_recipe_to_range`, etc.) remain.
- **Specification drafted.** See `docs/stateless_runtime_spec.md` for the clean-room design that will replace the deleted engine/runtime/UI history stack.

Do **not** attempt incremental changes or resurrect deleted modules. The next commits must introduce brand-new code that adheres to the specification.

## Key Goals
The rebuild must satisfy both the stateless requirements and the “Data Integrity & Real-Time Alignment” goals:
1. Load the reference clip at startup, extract the true reference features immediately, and keep the clip around for playback/recipes.
2. Derive every metric from genuine alignment data (no zero-filled fallbacks).
3. Begin learner comparisons within 0.5 s and surface all captured audio, including buffered lead-in chunks.
4. Process learner chunks incrementally without ever cloning or storing historical learner buffers inside the engine.
5. Maintain seamless playback/toggling between original and flowalyzed clips using the single cached source of truth.

## Architecture Overview

### Engine (SessionRuntime / SessionEngine)
1. **Capture loop**: read live chunk, resample to 16 kHz, apply gain (per data-integrity plan) so learner RMS matches reference.
2. **Processing window**: keep only the last `MAX_PROCESSING_WINDOW_MS` of audio per chunk (overwriting prior data).
3. **Feature extraction**: run the new chunk through the learner feature extractor (energy + pitch only for Phase 1).
4. **Alignment**: use absolute sample counters to choose the reference slice; align learner features to that slice without storing history.
5. **Snapshot emission**: build a `SessionSnapshot` containing the chunk’s `AlignmentReport`, scores, active clip, and recipe state; immediately send it to the runtime queue.
6. **Reset**: zero out the learner buffer and advance the absolute counter.

### Snapshots
- Carry only the current chunk’s vectors plus metadata (recording flags, clip variant, recipe state). No timelines or historical vectors.
- Provide the sole interface for the UI—every visualization must derive from snapshots or UI-owned histories.

### UI Responsibilities
1. Maintain rolling histories (`VecDeque<f32>`) for reference/learner energy, reference/learner pitch, similarity, and contour.
2. Trim histories to the configured visualization window (default 30 s) every time new data arrives.
3. Clear histories on session restart or clip-switch to guarantee stateless restarts.
4. Use cached reference features for playback/recipes without touching learner histories.

## Work Breakdown

### Phase 1 – Reintroduce Stateless Core (per spec)

| Substep | Key Code to Add | Tests (unit + phase integration) |
|---------|-----------------|----------------------------------|
| 1A. Snapshot data model | Create `src/pronunciation/session/snapshot.rs` defining `AlignmentReport`, `AlignedPhoneme`, `SessionSnapshot`, `SessionScores`, and enums from scratch, matching the spec field-by-field. Remove any `pub mod` exports not in scope. | Unit tests inside `snapshot.rs` verifying default values and serialization/deserialization (if `serde` remains). |
| 1B. Stateless aligner | Implement `StatelessAligner` in `src/pronunciation/alignment/mod.rs` to slice reference features based on sample offsets and compute per-chunk vectors. No caching of learner history. | Unit tests covering: absolute offset math, reference slice padding, similarity band ranges. |
| 1C. SessionEngine skeleton | Build a new `SessionEngine` in `src/pronunciation/session/engine.rs` (capture loop, feature extraction stub, `poll` that emits snapshots). Engine stores only reference features + sample counter. | Unit tests using a mock `CaptureSource` to confirm: (a) multiple chunks produce equal-length reports, (b) `global_time_offset_ms` advances per chunk. |
| 1D. Runtime/controller | Implement `SessionRuntime`, `SessionHandle`, `SessionController` in `src/pronunciation/session/runtime.rs` and re-export from `mod.rs`. Commands: start/stop, replay toggle, recipe apply, clip variant toggle. | Integration test `tests/stateless_runtime.rs` that spins up the runtime with a mock capture and verifies start→poll→stop flows (no UI yet). |
| 1E. CLI rewire | Update `src/pronunciation/cli.rs` and `run_session` to use the new runtime. Ensure reference clip loading + feature extraction happen immediately (goal #1). | Integration test in `tests/command_wiring.rs` covering session startup errors and recipe commands using the new API only. |

### Phase 2 – UI-Owned History

| Substep | Key Code to Add | Tests |
|---------|-----------------|-------|
| 2A. History buffers | In `src/ui/screens/session.rs`, define private `VecDeque<f32>` buffers and helper methods (`append_chunk`, `trim_to_window`, `clear_histories`). Remove any lingering timeline logic. | Unit tests (if feasible via a `#[cfg(test)]` module) ensuring `append_chunk` + `trim_to_window` enforce capacity correctly. |
| 2B. Visualization wiring | Update waveform/pitch/spectrogram components to read only from the histories. Snapshots serve solely as “latest chunk” to seed empties. | UI-focused test (headless) verifying `sync_visuals` produces normalized outputs when histories are populated/cleared. |
| 2C. Restart semantics | Wire controller actions (start/stop/toggle clip) to call `clear_histories` and ensure UI resets. | Integration test `tests/ui/history_reset.rs` that feeds mock snapshots into `SessionApp` and asserts histories drop to zero on restart. |

### Phase 3 – Recipe & Flowalyzer Hooks

| Substep | Key Code to Add | Tests |
|---------|-----------------|-------|
| 3A. Flowalyzed caching | Implement `cache_flowalyzed_features` + clip variant toggling within the new engine/runtime. Ensure only one source of truth for each clip (goal #5). | Integration test reusing `tests/clip_toggle.rs` logic but targeting the new runtime, verifying toggles + errors without timelines. |
| 3B. Recipe builder wiring | Reconnect UI builder → controller → runtime, ensuring recipe application uses the stateless engine and surfaces errors via snapshots. | Integration test `tests/recipe_application.rs` updated to the new API; ensures recipe errors propagate immediately. |
| 3C. Latency/lead-in handling | Ensure the runtime surfaces buffered lead-in audio and begins comparisons within ≤0.5 s (goals #2 and #3). Adjust capture timeouts/gain constants as needed. | Integration test `tests/latency_window.rs` measuring time from `start()` to first snapshot; unit test verifying gain constants. |
| 3D. Final verification | `cargo fmt`, `cargo test --all`, `cargo clippy --all -- -D warnings`. Document performance characteristics in `docs/stateless_runtime_spec.md`. | N/A (tooling). |

## Validation
- `cargo test --all` and `cargo clippy --all -- -D warnings` must pass after each phase.
- Manual verification: start recording, speak during a reference silence, confirm the waveform/pitch align exactly in the UI; stop and restart shadowing and ensure histories clear.

This plan only proceeds once the repository contains zero legacy code beyond the recipe helpers—a condition now met. Every substep references `docs/stateless_runtime_spec.md` and addresses the data-integrity goals explicitly. Tests focus solely on the code introduced in the current phase; no future-phase scaffolding is to be created. 
