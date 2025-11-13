# Flowalyzer–Pronunciation Integration Plan
*Prepared: 2025-11-13*

## Agreements Made
- *(2025-11-13)* “the user should be able to toggle between the reference clip (kept as the original) and produce a NEW clip with the transformations”

## Explicitly Rejected
- *(2025-11-13)* Replacing or mutating the original reference clip when applying Flowalyzer recipes (keep pristine copy).

## Implementation Details
### Current Architecture Summary
- `RecordedClip`, `PronunciationFeatures`, and `SessionSnapshot` data models live in `src/pronunciation/mod.rs` and `src/pronunciation/session.rs`.
- Runtime thread handles commands (`SessionCommand`) and publishes immutable snapshots to the UI.
- Flowalyzer recipe pipeline (`src/operations/recipe.rs`, `src/types.rs`) outputs `AudioData` fragments assembled and encoded downstream.

### Target Enhancements
- Maintain parallel clip variants: original reference clip + Flowalyzer-generated clip.
- Cache pronunciation analyses keyed by clip identity; reuse analyses when toggling between variants.
- Enforce 5-minute maximum duration for both original and generated clips; fail fast and surface UI messages.
- Provide UI controls for range selection, recipe construction, apply/reapply, and clip toggling.
- Add runtime commands to construct flowalyzed clips, purge stale caches, and update snapshots.

### Testing Expectations
- Unit tests for duration guards, cache invalidation, recipe-to-clip conversion, UI state helpers.
- Integration/UI tests for range selection logic, recipe serialization, toggle behavior.
- Regression coverage to ensure base pronunciation workflow remains intact when Flowalyzer features unused.

## Issues Encountered
- *(Pending updates once phases execute; section reserved for future notes.)*

## Phase Status Overview
| Phase | Scope | Status | Notes |
| --- | --- | --- | --- |
| Phase 1 | Documentation & planning groundwork | Not started | Produce, review, and land this plan document. |
| Phase 2 | Backend clip management & caching | Blocked on Phase 1 | Implement variant storage, caching, recipe hooks. |
| Phase 3 | UI range selection & recipe builder | Blocked on Phase 2 | Front-end interactions, validation, UX state. |
| Phase 4 | Apply/toggle wiring | Blocked on Phase 3 | Command flow, clip generation, toggles. |
| Phase 5 | Persistence, UX polish, regression | Blocked on Phase 4 | Final persistence decisions, docs, regression sweep. |

---

## Phase 1 – Architecture Documentation & Data Contracts
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker? (e.g. fake credentials, a blank user state)? This means the code should be rearchitected so that either the object doesn't need to be passed, or a real instance passed through instead.
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

### Phase 1.1 – Agreements & Rejections Ledger
- Populate Agreements, Explicitly Rejected, and Issues Encountered headings with dated entries extracted from user instructions.
- Files: `docs/current-plans/pronunciation_flowalyzer_integration.md` (new).
- Deliverable: Initial planning doc scaffolding containing agreement quote and rejection list.

### Phase 1.2 – Data & Flow Research Summary
- Document existing pronunciation session architecture (clip loading, feature caching, runtime loop, UI snapshot handling).
- Outline Flowalyzer recipe APIs and how their outputs map into pronunciation data structures.
- Describe target contracts for original vs flowalyzed clips, cache invalidation rules, 5-minute guardrails, and UI command flow.
- Files: `docs/current-plans/pronunciation_flowalyzer_integration.md` (update).
- Deliverable: Implementation Details section with traceable references to structs/functions plus narrative of planned data flow.

### Phase 1.3 – Phase Checkpoint Plan
- Expand this plan into explicit backend/UI/persistence subphases (Phases 2–5) and align with testing expectations.
- Add status tracking table keyed by subphase with success criteria and test suites.
- Files: `docs/current-plans/pronunciation_flowalyzer_integration.md` (update).
- Deliverable: Completed status table and testing checklist for ensuing phases.

**Completion Reminder:** When Phase 1 work is implemented, run `cargo test --all`, `cargo fmt`, and `cargo clippy`. After 100% success, update this document’s status section and obtain explicit approval before continuing.

---

## Phase 2 – Backend Clip Management & Analysis Caching
- [ ] **Planning Documentation**
- [ ] **Code Simplicity**
- [ ] **Code Modularity**
- [ ] **Scope Control**
- [ ] **No Dead Code**
- [ ] **No Fake Constructions**
- [ ] **Code Purpose**
- [ ] **Required Tests**

### Phase 2.1 – Clip Variant Storage & Limits
- Extend runtime state to maintain original + flowalyzed `RecordedClip` variants, metadata timestamps, active flag.
- Enforce 5-minute duration cap for both loaded and generated clips, returning surfaced errors for UI display.
- Files: `src/pronunciation/session.rs`, `src/pronunciation/mod.rs`, `src/pronunciation/tests/` (new or updated).

### Phase 2.2 – Analysis Cache Management
- Associate cached `PronunciationFeatures`/`AlignmentReport` with clip identity to enable reuse between toggles.
- Invalidate previous flowalyzed caches when a new variant replaces them; retain original reference caches.
- Files: `src/pronunciation/session.rs`, `src/pronunciation/alignment.rs`, tests verifying cache reuse/invalidation.

### Phase 2.3 – Flowalyzer Recipe Application Hook
- Integrate Flowalyzer recipe pipeline to generate new `AudioData` segments, convert to `RecordedClip`.
- Handle error propagation and ensure resulting clip obeys duration guard.
- Files: `src/pronunciation/mod.rs`, `src/operations/recipe.rs`, `src/types.rs`, unit tests for conversion and error paths.

**Completion Reminder:** After Phase 2 implementation, run `cargo test --all`, `cargo fmt`, `cargo clippy`. On success, update this document’s status section and stop pending approval.

---

## Phase 3 – UI Range Selection & Recipe Builder
- [ ] **Planning Documentation**
- [ ] **Code Simplicity**
- [ ] **Code Modularity**
- [ ] **Scope Control**
- [ ] **No Dead Code**
- [ ] **No Fake Constructions**
- [ ] **Code Purpose**
- [ ] **Required Tests**

### Phase 3.1 – Waveform Range Selection
- Extend waveform component to render draggable start/end handles with visual feedback.
- Enforce maximum 5-minute span and display validation errors.
- Files: `src/ui/components/waveform.rs`, `src/ui/screens/session.rs`, possible new helper module & UI tests.

### Phase 3.2 – Recipe Builder UI
- Build recipe editor panel (steps list, repeat/speed/silence controls, presets, validation messages).
- Serialize into Flowalyzer runtime recipe format for backend consumption.
- Files: `src/ui/components/recipe_builder.rs` (new), `src/ui/screens/session.rs`, UI serialization tests.

### Phase 3.3 – UI State Persistence & Feedback
- Persist selection/recipe state in session app until user applies changes; reset on session reloads.
- Display summary of staged recipe and selection metrics.
- Files: `src/ui/screens/session.rs`, tests for state reset behavior.

**Completion Reminder:** Following Phase 3 implementation, run `cargo test --all`, `cargo fmt`, `cargo clippy`. Record status updates here and wait for approval.

---

## Phase 4 – Flowalyzer Application Trigger & Toggle Controls
- [ ] **Planning Documentation**
- [ ] **Code Simplicity**
- [ ] **Code Modularity**
- [ ] **Scope Control**
- [ ] **No Dead Code**
- [ ] **No Fake Constructions**
- [ ] **Code Purpose**
- [ ] **Required Tests**

### Phase 4.1 – Command Wiring
- Define UI-to-runtime command carrying range + recipe payload; extend `SessionCommand`/controller APIs.
- Ensure runtime queues work without blocking audio capture loop.
- Files: `src/pronunciation/session.rs`, `src/pronunciation/ui.rs`, `src/ui/screens/session.rs`, command-handling tests.

### Phase 4.2 – Flowalyzed Clip Generation & Storage
- Runtime applies recipe, stores new clip/analysis, drops stale flowalyzed caches, emits progress snapshots.
- Files: `src/pronunciation/session.rs`, `src/pronunciation/mod.rs`, tests covering command execution.

### Phase 4.3 – Clip Toggle Mechanics
- Implement UI toggle control between original and flowalyzed analyses; update snapshots with active clip flag.
- Ensure backend swaps active analysis efficiently using cached data.
- Files: `src/ui/screens/session.rs`, `src/pronunciation/session.rs`, tests verifying toggle state and cache reuse.

**Completion Reminder:** After Phase 4, run `cargo test --all`, `cargo fmt`, `cargo clippy`; update this doc and pause for approval.

---

## Phase 5 – Final Polish & Persistence Hooks
- [ ] **Planning Documentation**
- [ ] **Code Simplicity**
- [ ] **Code Modularity**
- [ ] **Scope Control**
- [ ] **No Dead Code**
- [ ] **No Fake Constructions**
- [ ] **Code Purpose**
- [ ] **Required Tests**

### Phase 5.1 – Persistence & Reload Semantics
- Decide whether flowalyzed clip analyses persist beyond live session (seek user confirmation if requirements change).
- Implement storage/reload path if persistence required; otherwise document in-memory behavior explicitly.
- Files: `src/pronunciation/session.rs`, possible new persistence module/tests.

### Phase 5.2 – UX Feedback & Documentation
- Add UI messaging for apply success/failure, active clip labels, timestamps.
- Update README/tutorial docs (`docs/pronunciation/PROGRAM_FLOW.md`, `README.md`) to describe new workflow and toggles.
- Files: UI modules, documentation files.

### Phase 5.3 – Regression Sweep & Wrap-up
- Run targeted regression tests ensuring pronunciation session works without Flowalyzer enhancements enabled.
- Finalize planning doc with completion notes, residual risks, and any deferred follow-ups.
- Files: regression test suites, this planning doc.

**Completion Reminder:** After Phase 5, run `cargo test --all`, `cargo fmt`, `cargo clippy`; once everything is green, record final status here and present deliverables for sign-off.

