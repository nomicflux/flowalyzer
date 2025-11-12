## Phase 3 – Latency, UX Polish, and Release Prep

- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PRONUNCIATION_TOOL_PHASE3_PLAN.md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
      (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object
      doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do your changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Agreements Made
- (2025-11-12) User: "Verbatim copy phase 3 to your plan. Add in details for implementation. Trust your understanding of the key goals now; assume you will make mistakes if not guided completely by the TODOS in the plan in every detail."
- (2025-11-12) User: "THIS IS A REAL TIME APP. RECORDING IS REAL-TIME AND INTERACTIVE. YOU ARE _NOT_ FOR _ANY_ REASON TO LOAD A RECORDED FILE."

### Explicitly Rejected
- (2025-11-12) User: "Do NOT edit the plan file itself."

### Implementation Details
- Session orchestration uses `SessionRuntime` and `SessionHandle` to encapsulate live capture, alignment, and UI command channels.
- Live capture utilises `audio::capture::LiveCapture` to stream microphone audio in <200 ms chunks, resampling to `TARGET_SAMPLE_RATE`.
- Reference playback is managed via `ReferencePlayer` (Rodio), triggered alongside capture start to enforce synchronous shadowing.
- The alignment loop recomputes incremental matches on each chunk, emitting `SessionSnapshot` updates with latency telemetry and score deltas.
- UI layer consumes real-time snapshots, renders updated waveforms/spectrograms, and routes control events through `SessionController`.
- CLI now builds a session-only configuration (no learner WAV), launches the runtime, and defers to the UI/headless runner.

### Phase Tasks
- [ ] Tune capture/playback pipeline to guarantee ≤200 ms round-trip latency with device selection, buffer controls, and resilience to CoreAudio errors.
- [ ] Finalise UI interactions: immediate waveform/spectrogram updates, grading badges, per-metric tooltips, and replay toggles—all within the same session window.
- [ ] Add automated UI/state tests (serialization or headless) that exercise record → analyze → feedback loops and accessibility baseline (keyboard focus, contrast).
- [ ] Update user-facing documentation (README, tutorials, status doc) to state unequivocally that analysis happens inside the live session.

### Issues Encountered
- None yet this phase. Record all failures, corrections, and user clarifications here as work progresses.

### Deliverables
- Production-ready desktop session with guaranteed latency budget, comprehensive in-session feedback, accessibility coverage, and updated release docs.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) after each milestone, update the status document, and STOP for explicit approval before moving beyond Phase 3._

