# Pronunciation Tool Audio-Only Redesign Plan

## KEY GOAL – DECIDES EVERY DECISION
- This is an application to practice **pronunciation in non-English languages** with complex phonemic structure.
- The learner always shadows or responds to a **reference audio clip** inside the same interactive session.
- The application must **record, compare, and visualise differences in near real time** (≤200 ms pipeline latency).
- **Pitch is ignored**; timing, articulation, and prosody differences must be highlighted with actionable grading bands.
- All guidance is delivered **within the interactive session**. No separate offline analyzers, transcripts, or text alignment survive.
- ASK CLARIFYING QUESTIONS AS SOON AS ANY REQUIREMENT IS UNCLEAR.

---

## Phase 1 – Session-Only Audio Baseline
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
      (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object
      doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do your changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Tasks
- Delete CMU-lexicon assets, transcript-driven alignment modules, and any structs or tests that reference transcripts.
- Collapse the CLI into a **session-first workflow**: keep capture/playback helpers only if they directly support live sessions; delete the `analyze` command and any JSON export paths.
- Redesign `SessionConfig` around audio inputs plus capture/control metadata; ensure `run_session` and UI preparation run **without transcripts**.
- Provide a temporary audio-vs-audio comparison scaffold that surfaces coarse similarity/timing data so the UI can render meaningful feedback until Phase 2 lands.
- Update documentation (`README`, tutorials, status doc) to state unequivocally that analysis happens inside the live session.

### File Scope
- `src/pronunciation/mod.rs`
- `src/pronunciation/alignment/`
- `src/pronunciation/cli.rs`
- `src/bin/pronunciation.rs`
- `src/ui/`
- `tests/`
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`
- `README.md`

### Deliverables
- No module, test, or document mentions transcripts, CMU dictionaries, or offline analysis commands.
- Launching `pronunciation session` (or equivalent) loads the UI, records learner audio, compares against reference, and renders coarse feedback in-session.
- Build, lint, and tests succeed with the audio-only, session-first baseline.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) and update the status document upon success. STOP and await explicit approval before proceeding to Phase 2._

---

## Phase 2 – Real-Time Audio Alignment & Scoring
- [ ] **Planning Documentation**
- [ ] **Code Simplicity**
- [ ] **Code Modularity**
- [ ] **Scope Control**
- [ ] **No Dead Code**
- [ ] **No Fake Constructions**
- [ ] **Code Purpose**
- [ ] **Required Tests**

### Tasks
- Implement audio-only feature alignment (DTW or similar) across mel/MFCC trajectories with explicit pitch de-emphasis.
- Produce frame-level metrics (timing deviation, articulation variance, spectral similarity bands) that map directly to the grading buckets described in the Key Goal.
- Replace placeholder scores with deterministic audio-derived outputs and wire them through the UI visualisations.
- Add deterministic unit tests using synthetic audio pairs (aligned, shifted, mis-articulated) to guarantee invariants and latency budgets.

### File Scope
- `src/pronunciation/features/`
- `src/pronunciation/alignment/`
- `src/pronunciation/metrics/`
- `src/ui/screens/session.rs`
- `tests/alignment.rs`
- `tests/fixtures/alignment/`

### Deliverables
- End-to-end session run produces deterministic, language-agnostic audio alignment and scoring with tests proving timing/pitch handling.

_Reminder: run the FULL test suite and update the status document. STOP and await explicit approval before Phase 3._

---

## Phase 3 – Latency, UX Polish, and Release Prep
- [ ] **Planning Documentation**
- [ ] **Code Simplicity**
- [ ] **Code Modularity**
- [ ] **Scope Control**
- [ ] **No Dead Code**
- [ ] **No Fake Constructions**
- [ ] **Code Purpose**
- [ ] **Required Tests**

### Tasks
- Tune capture/playback pipeline to guarantee ≤200 ms round-trip latency with device selection, buffer controls, and resilience to CoreAudio errors.
- Finalise UI interactions: immediate waveform/spectrogram updates, grading badges, per-metric tooltips, and replay toggles—all within the same session window.
- Add automated UI/state tests (serialization or headless) that exercise record → analyze → feedback loops and accessibility baseline (keyboard focus, contrast).
- Update user-facing documentation (README, tutorials, changelog) to walk through the interactive session workflow and troubleshooting steps.

### File Scope
- `src/audio/capture.rs`
- `src/pronunciation/mod.rs`
- `src/ui/`
- `tests/ui/`
- `README.md`
- `docs/current-plans/`
- `CHANGELOG.md`

### Deliverables
- Production-ready desktop session with guaranteed latency budget, comprehensive in-session feedback, accessibility coverage, and updated release docs.

_Reminder: run the FULL test suite and update the status document. STOP and await explicit approval before concluding the project._

