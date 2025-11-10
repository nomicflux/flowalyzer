# RECORDING AND ANALYSIS Pronunciation Tool Implementation Plan

> Research has already been completed. This plan covers the implementation-only phases for the Rust-only pronunciation assessment binary.

## Phase 1 – Architecture Sign-off & Scaffolding
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Goals & Tasks**
- Produce `docs/current-plans/PRONUNCIATION_TOOL_ARCHITECTURE.md` summarizing the Rust-only pipeline derived from prior research (capture → features → phoneme alignment → visualization).
- Update `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md` with finalized requirements: no Python tooling, CMU-style lexicon bundled, secondary binary entrypoint.
- Create module scaffolding without behavior: `src/bin/pronunciation.rs`, `src/pronunciation/mod.rs`, directories for `features`, `alignment`, `metrics`, `ui` with placeholder TODO-free stubs.

**File Scope**
- `docs/current-plans/PRONUNCIATION_TOOL_ARCHITECTURE.md`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`
- `src/bin/pronunciation.rs`
- `src/pronunciation/mod.rs`
- `src/pronunciation/features/mod.rs`
- `src/pronunciation/alignment/mod.rs`
- `src/pronunciation/metrics/mod.rs`
- `src/ui/mod.rs`

**Deliverables**
- Architecture document capturing module boundaries, data contracts, and dependencies.
- Compiling scaffolding with empty (but non-dead) modules ready for implementation.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) once scaffolding compiles cleanly. Upon 100% success, update the status document and await explicit approval before Phase 2._

## Phase 2 – Audio Capture & Playback Foundations
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Goals & Tasks**
- Implement microphone capture using `cpal`, including device selection, sample-rate conversion to 16 kHz mono, and deterministic buffering into `AudioData`.
- Implement playback via `rodio` for reference WAVs and recorded takes, with safe downmix/upmix handling.
- Extend `src/bin/pronunciation.rs` to expose CLI commands for record/playback only (no analysis yet).
- Add targeted unit tests for downmixing, linear resampling, and CLI argument parsing.

**File Scope**
- `Cargo.toml`
- `src/audio/capture.rs`
- `src/audio/playback.rs`
- `src/audio/mod.rs`
- `src/bin/pronunciation.rs`
- `tests/audio_capture_playback.rs`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`

**Deliverables**
- Secondary binary capable of reliable audio capture and playback with tests covering conversion utilities.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) and update the status document upon 100% success. Await explicit approval before Phase 3._

## Phase 3 – Spectral Feature Pipeline (Pitch-Invariant)
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Goals & Tasks**
- Implement `src/pronunciation/features.rs` using `aus` to compute mel spectrograms, spectral flux, energy contours, MFCC + delta coefficients, normalized for both learner and reference audio.
- Add `tests/features.rs` with golden fixtures (bundled WAV snippets + expected JSON) to verify deterministic outputs and pitch invariance.
- Document feature parameters and configuration toggles (window size, hop, mel band count) in the status doc.

**File Scope**
- `Cargo.toml` (enable `aus` features)
- `src/pronunciation/features.rs`
- `tests/features.rs`
- `tests/fixtures/features/`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`

**Deliverables**
- Deterministic feature extraction library delivering aligned tensors ready for phoneme alignment, with tests guarding regression.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) and update the status document upon success. Await explicit approval before Phase 4._

## Phase 4 – Rust-Only Phoneme Alignment & Timing Analysis
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Goals & Tasks**
- Implement `src/pronunciation/dictionary.rs` parsing a bundled CMU-style phoneme lexicon (stored under `assets/phonemes/`) and mapping transcript tokens to phoneme targets.
- Implement `src/pronunciation/alignment.rs`:
  - Spectral template/likelihood generation for phoneme segments.
  - Dynamic Time Warping aligning learner vs. reference phoneme trajectories (timing + similarity).
  - Outputs per-phoneme timing offsets, confidence scores, and articulation variance metrics.
- Create `tests/alignment.rs` with synthetic audio + transcripts validating dictionary mapping and DTW correctness.

**File Scope**
- `build.rs` (for asset embedding)
- `assets/phonemes/lexicon.txt`
- `src/pronunciation/dictionary.rs`
- `src/pronunciation/alignment.rs`
- `tests/alignment.rs`
- `tests/fixtures/alignment/`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`

**Deliverables**
- Fully Rust-based phoneme alignment engine with automated verification, zero external tooling.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) and update the status document upon success. Await explicit approval before Phase 5._

## Phase 5 – Pronunciation Metrics & Interactive Visualization
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Goals & Tasks**
- Implement `src/pronunciation/metrics.rs` calculating overall pronunciation scores (timing deviation, articulation similarity, intonation variance) from alignment outputs.
- Build interactive UI with `egui`/`eframe` in `src/ui/`:
  - Real-time waveform + mel spectrogram overlays.
  - Phoneme timeline with per-phoneme deviation indicators.
  - Controls for recording, playback, and comparison sessions.
- Add regression tests for metric calculations and UI state serialization; produce manual QA checklist for macOS 15.

**File Scope**
- `Cargo.toml` (`eframe`, `egui_extras`, plotting crates)
- `src/pronunciation/metrics.rs`
- `src/ui/mod.rs`
- `src/ui/screens/session.rs`
- `src/ui/components/`
- `assets/ui/`
- `tests/metrics.rs`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`

**Deliverables**
- Pronunciation scoring engine and interactive desktop UI providing real-time feedback, with accompanying automated tests.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) and update the status document upon success. Await explicit approval before Phase 6._

## Phase 6 – Integration, Packaging, and Release Readiness
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Goals & Tasks**
- Unify CLI + GUI flows, ensuring configuration files, lexicon assets, and fixtures are discovered reliably.
- Add integration tests (`tests/integration/full_session.rs`) covering record → analyze → visualize pipeline using short bundled fixtures.
- Document installation, configuration, and troubleshooting in `README.md` and update the status document with completion notes.
- Prepare release checklist: binary artifacts, macOS signing/notarization guidance, changelog entry.

**File Scope**
- `src/bin/pronunciation.rs`
- `src/config/mod.rs`
- `tests/integration/full_session.rs`
- `README.md`
- `CHANGELOG.md`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`

**Deliverables**
- Production-ready secondary binary with complete documentation, automated coverage, and release artifacts/checklists.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) and update the status document upon success. Await explicit approval before concluding the project._
