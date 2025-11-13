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
- CLI now builds a session-only configuration (no learner WAV) and launches the interactive runtime.

## Phase 3 – UI Real-Time Feedback (`phase3-ui`)

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

### File Scope
- `src/pronunciation/session.rs`
- `src/ui/mod.rs`
- `src/ui/screens/session.rs`
- `src/ui/components/phoneme_timeline.rs`
- `src/ui/components/pitch.rs`
- `src/ui/components/spectrogram.rs`
- `src/ui/components/waveform.rs`
- `tests/ui/` (egui coverage)
- `assets/` (if additional UI configuration assets are required)
- `docs/current-plans/PRONUNCIATION_TOOL_IMPLEMENTATION_STATUS.md`

### Detailed Implementation Steps
1. **Real-time snapshot ingestion**
   - Extend `SessionHandle` polling to surface a `VecDeque` of the most recent `SessionSnapshot` values, ensuring UI draws the latest frame without blocking egui’s event loop.
   - Guard UI refresh intervals to <16 ms so waveform/pitch panels reflect latency within the ≤200 ms goal.
2. **Control surface & accessibility**
   - Replace ad-hoc record button with a dedicated `ControlStrip` helper (new file) that exposes record/stop, reference replay toggles, and a latency badge with ARIA-style descriptions.
   - Add keyboard shortcuts (`Space` to toggle recording, `R` to replay reference) and ensure focus order matches visual layout. Colour tokens must meet WCAG AA contrast.
3. **Dynamic visualisations**
   - Upgrade waveform, spectrogram, and pitch components to accept incremental append deltas rather than whole vectors, clipping to last N frames (≈4 s) to emphasise live response.
   - Annotate phoneme timeline entries with tooltips summarising timing/articulation/intonation deltas and contour compliance, highlighting segments below threshold bands.
4. **Reference/learner synchrony**
   - Surface the current playback position and alignment window, verifying pitch contour overlays track key goal requirement: matching relative motion while ignoring absolute pitch.
5. **Error & latency UX**
   - Display latency warnings whenever `SessionSnapshot::error` references budget overruns; include actionable guidance (increase buffer, reduce device load) inline.
6. **UI tests**
   - Add `tests/ui/session_focus.rs` to simulate egui frames, asserting focus traversal, keyboard shortcuts, and latency badge rendering.
   - Add `tests/ui/session_snapshots.rs` to feed synthetic `SessionSnapshot` streams and confirm incremental waveform/pitch updates without panics.
7. **Documentation Alignment**
   - Update status doc to reflect UI interactivity and contour-focused feedback; if UI instructions change, capture them in README/live session walkthrough (Phase 3 docs task).

### Deliverables
- UI reacts to streaming capture within ≤200 ms, highlighting timing, articulation, and prosody gaps with actionable bands.
- Controls enforce live recording only; no UI affordance exists for loading prerecorded audio, satisfying the key goal.
- Accessibility coverage documented (focus order, contrast, shortcut summary).
- Automated UI/state tests prove continuous feedback loop and latency warnings.

Implementation explicitly upholds the KEY GOAL: users shadow a reference clip inside the same session; live capture + alignment updates drive all visualisations; pitch contour matching (relative movement) is surfaced in the pitch panel and phoneme tooltips; no offline analysis or prerecorded learner files are introduced.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) after completing `phase3-ui`, update the status document with dated results, and STOP for explicit approval before moving on._

### Phase Tasks
- [ ] Tune capture/playback pipeline to guarantee ≤200 ms round-trip latency with device selection, buffer controls, and resilience to CoreAudio errors.
- [ ] Finalise UI interactions: immediate waveform/spectrogram updates, grading badges, per-metric tooltips, and replay toggles—all within the same session window.
- [ ] Add automated UI/state tests (serialization-driven) that exercise record → analyze → feedback loops and accessibility baseline (keyboard focus, contrast).
- [ ] Update user-facing documentation (README, tutorials, status doc) to state unequivocally that analysis happens inside the live session.

### Issues Encountered
- None yet this phase. Record all failures, corrections, and user clarifications here as work progresses.

### Deliverables
- Production-ready desktop session with guaranteed latency budget, comprehensive in-session feedback, accessibility coverage, and updated release docs.

_Reminder: run the FULL test suite (`cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`) after each milestone, update the status document, and STOP for explicit approval before moving beyond Phase 3._

