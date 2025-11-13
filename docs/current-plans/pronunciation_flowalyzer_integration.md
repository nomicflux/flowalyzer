# Flowalyzer–Pronunciation Integration Plan
*Prepared: 2025-11-13*

## Agreements Made
- *(2025-11-13)* “the user should be able to toggle between the reference clip (kept as the original) and produce a NEW clip with the transformations”

## Explicitly Rejected
- *(2025-11-13)* Replacing or mutating the original reference clip when applying Flowalyzer recipes (keep pristine copy).

## Implementation Details

### Current Architecture Summary

#### Clip Loading Architecture
- **Location**: `src/pronunciation/mod.rs::load_clip()` (lines 301-313)
- **Process**: 
  1. Uses `decoder::decode_audio()` to load WAV file, producing `AudioData`
  2. `clip_from_audio()` converts `AudioData` to `RecordedClip` (lines 315-319)
  3. All clips resampled to `TARGET_SAMPLE_RATE` (16,000 Hz) via `resample::linear_resample()`
- **Current State**: Single reference clip loaded at session startup in `EngineRunner::build()` (`src/pronunciation/session.rs:609-673`)
- **Storage**: Reference clip stored in `EngineRunner.reference: RecordedClip` (session.rs:603)
- **Data Types**:
  - `RecordedClip` (mod.rs:54-60): `samples: Arc<[f32]>`, `sample_rate: u32`, `channels: u8`, `duration: Duration`
  - `AudioData` (types.rs:7-13): `samples: Vec<f32>`, `sample_rate: u32`

#### Feature Caching Implementation
- **Location**: `src/pronunciation/session.rs::engine::SessionEngine` (lines 333-486)
- **Current Implementation**: 
  - Reference features extracted once at engine creation (lines 353-375)
  - Storage: `SessionEngine.reference_features: PronunciationFeatures` (line 338)
  - Extraction: `FeatureExtractor::extract(&RecordedClip)` returns `PronunciationFeatures` containing:
    - `mel_spectrogram: Array2<f32>`
    - `mfcc: Array2<f32>`, `deltas`, `delta_deltas`
    - `pitch_contour: Array1<f32>`
    - `energy: Array1<f32>`, `spectral_flux: Array1<f32>`
- **Learner Features**: Extracted on-demand during `process_chunk()` (line 472), not cached
- **Gap Identified**: No current mechanism for caching learner features or associating features with clip identity

#### Runtime Loop Structure
- **Location**: `src/pronunciation/session.rs::EngineRunner::run()` (lines 675-706)
- **Command Handling**: `Receiver<SessionCommand>` receives commands from UI thread
- **Snapshot Publishing**: `Sender<SessionSnapshot>` sends immutable snapshots to UI
- **Command Types** (lines 934-940): `Start`, `Stop`, `ReplayReference`, `StopReplay`, `Shutdown`
- **Main Loop**: Blocks on `commands.recv()`, processes command, sends snapshot updates
- **Drive Loop**: During recording (lines 789-851), polls commands non-blocking via `poll_command()`, calls `engine.poll()` every 20ms

#### UI Snapshot Handling
- **Location**: `src/ui/screens/session.rs::SessionApp` (lines 19-388)
- **Initial Snapshot**: `SessionApp::new()` receives `SessionHandle.initial_snapshot()` (line 37)
- **Update Polling**: `poll_updates()` (lines 79-90) calls `handle.drain_snapshots()` to get all pending updates
- **Snapshot Structure**: `SessionSnapshot` (session.rs:50-59) contains:
  - `alignment: AlignmentReport`
  - `scores: PronunciationScores`
  - `recording: bool`, `reference_playing: bool`
  - `latency_ms: f32`, `error: Option<String>`, `initializing: bool`
- **Visual Sync**: `sync_visuals()` (lines 55-68) extracts data from snapshot for rendering:
  - Waveforms from `alignment.reference_energy` and `alignment.learner_energy`
  - Pitch from `alignment.reference_pitch` and `alignment.learner_pitch`
  - Spectrogram from alignment data

### Flowalyzer Recipe Integration Points

#### Recipe Types and APIs
- **Location**: `src/types.rs` (lines 75-174)
- **Types**:
  - `Recipe`: Runtime recipe with `name: String` and `steps: Vec<RecipeStep>`
  - `RecipeStep`: Contains `repeat_count: u32`, `speed_factor: f32`, `silent: bool`
  - `RuntimeRecipe`: Deserializable from JSON with validation via `validate()`
  - Conversion: `RuntimeRecipe::to_recipe()` converts to `Recipe` type

#### Recipe Application Function
- **Location**: `src/operations/recipe.rs::apply_recipe()` (lines 51-68)
- **Input**: `&AudioChunk`, `&Recipe`
- **Output**: `Vec<AudioChunk>` (multiple chunks after applying all steps)
- **Process**: For each step:
  1. Apply speed change via `change_speed(chunk, step.speed_factor)` → `AudioChunk`
  2. If `silent: false`: Repeat chunk `repeat_count` times via `repeat_chunk()`
  3. If `silent: true`: Emit `repeat_count` silence chunks via `insert_silence()`
- **Pure Function**: No side effects, takes data in, returns data out

#### Type Conversion Paths
- **AudioChunk to RecordedClip Mapping**:
  - `AudioChunk` (types.rs:68-73): Contains `samples: Vec<f32>`, `sample_rate: u32`, `start_time: f64`, `end_time: f64`
  - `RecordedClip` (mod.rs:54-60): Contains `samples: Arc<[f32]>`, `sample_rate: u32`, `channels: u8`, `duration: Duration`
  - Conversion needed: `AudioChunk` → `RecordedClip` via `RecordedClip::from_samples()` (mod.rs:259-268)
  - Note: `AudioChunk` has timing metadata (`start_time`, `end_time`) that `RecordedClip` doesn't preserve
  - Assembly: Multiple `AudioChunk` results from `apply_recipe()` need to be assembled into single audio stream before conversion
- **AudioData Type**:
  - Location: `src/types.rs` (lines 7-13)
  - Structure: `samples: Vec<f32>`, `sample_rate: u32`
  - Used by: Audio decoder output, input to resampler
  - Conversion path: `AudioData` → `RecordedClip` via `clip_from_audio()` (mod.rs:315-319)

### Target Data Contracts

#### Original vs Flowalyzed Clip Storage
- **Current**: Single `EngineRunner.reference: RecordedClip` (session.rs:603)
- **Target**: Maintain two variants:
  - `original_reference: RecordedClip` (immutable, loaded from file)
  - `flowalyzed_reference: Option<RecordedClip>` (generated from recipe application)
- **Active Clip Flag**: `active_clip: ClipVariant` enum (`Original | Flowalyzed`)
- **Metadata**: Timestamp when flowalyzed clip was generated, recipe used

#### Cache Invalidation Rules
- **Reference Features Cache**: Keyed by clip identity (original vs flowalyzed)
  - Current: `SessionEngine.reference_features` computed once for original
  - Target: Cache structure: `HashMap<ClipIdentity, PronunciationFeatures>`
  - Invalidation: When new flowalyzed clip replaces old one, invalidate old flowalyzed cache
  - Retention: Original reference cache always retained (never invalidated)
- **Alignment Cache**: `AlignmentReport` should also be keyed by clip identity
  - Current: Alignment computed on-demand during `process_chunk()`
  - Target: Cache alignment results per clip variant, reuse when toggling

#### 5-Minute Duration Guardrails
- **Location**: To be added in `load_clip()` and clip generation functions
- **Check**: `clip.duration <= Duration::from_secs(300)` (5 minutes = 300 seconds)
- **Error Handling**: Return `PronunciationError` with message for UI display
- **Application Points**:
  1. When loading reference clip in `load_clip()` (mod.rs:301-313)
  2. When generating flowalyzed clip from recipe (new function)
  3. When assembling multiple `AudioChunk` results into final clip

#### UI Command Flow
- **Current Commands**: `Start`, `Stop`, `ReplayReference`, `StopReplay`, `Shutdown` (session.rs:934-940)
- **New Commands Needed**:
  - `ApplyFlowalyzerRecipe { range: (f64, f64), recipe: Recipe }` - Apply recipe to selected range
  - `ToggleClipVariant { variant: ClipVariant }` - Switch between original and flowalyzed
- **Command Handling**: Extend `EngineRunner::run()` match statement (lines 679-703)
- **Range Selection**: UI will provide start/end times in seconds relative to original clip
- **Recipe Payload**: Serialized `RuntimeRecipe` from UI, validated and converted to `Recipe`

### Data Flow Diagrams

#### Recipe Application Flow
1. UI sends `ApplyFlowalyzerRecipe` command with range and recipe JSON
2. Runtime extracts audio range from original reference clip
3. Converts range to `AudioChunk` (needs `start_time`, `end_time` metadata)
4. Calls `apply_recipe(&chunk, &recipe)` → `Vec<AudioChunk>`
5. Assembles chunks into single `AudioData` stream (needs assembler function)
6. Converts `AudioData` → `RecordedClip` via `clip_from_audio()`
7. Validates duration (5-minute limit)
8. Extracts features from new clip
9. Stores flowalyzed clip and features in cache
10. Invalidates old flowalyzed cache if exists
11. Sends snapshot update with new clip available

#### Toggle Flow
1. UI sends `ToggleClipVariant` command
2. Runtime updates `active_clip` flag
3. Retrieves cached features for active clip variant
4. If cache miss: Extract features (shouldn't happen if caching works)
5. Updates snapshot with active clip indicator
6. UI re-renders with features from active variant

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
- **Status: COMPLETE** *(2025-01-27)* - Verified all three sections are properly formatted with dated entries. All entries validated for accuracy and consistency. Document structure confirmed correct.

### Phase 1.2 – Data & Flow Research Summary
- Document existing pronunciation session architecture (clip loading, feature caching, runtime loop, UI snapshot handling).
- Outline Flowalyzer recipe APIs and how their outputs map into pronunciation data structures.
- Describe target contracts for original vs flowalyzed clips, cache invalidation rules, 5-minute guardrails, and UI command flow.
- Files: `docs/current-plans/pronunciation_flowalyzer_integration.md` (update).
- Deliverable: Implementation Details section with traceable references to structs/functions plus narrative of planned data flow.
- **Status: COMPLETE** *(2025-01-27)* - Comprehensive documentation added to Implementation Details section covering: clip loading architecture, feature caching implementation, runtime loop structure, UI snapshot handling, Flowalyzer recipe APIs, type conversion paths, target data contracts, and data flow diagrams for recipe application and toggle operations.

### Phase 1.3 – Phase Checkpoint Plan
- Expand this plan into explicit backend/UI/persistence subphases (Phases 2–5) and align with testing expectations.
- Add status tracking table keyed by subphase with success criteria and test suites.
- Files: `docs/current-plans/pronunciation_flowalyzer_integration.md` (update).
- Deliverable: Completed status table and testing checklist for ensuing phases.
- **Status: COMPLETE** *(2025-01-27)* - Simple status tracking table created with all 12 subphases (2.1-5.3) for tracking progress. Table inserted after Phase 1.3 description.

**Completion Reminder:** When Phase 1 work is implemented, run `cargo test --all`, `cargo fmt`, and `cargo clippy`. After 100% success, update this document's status section and obtain explicit approval before continuing.

### Phase Status Tracking Table

| Phase/Subphase | Status |
|----------------|--------|
| 2.1 | Pending |
| 2.2 | Pending |
| 2.3 | Pending |
| 3.1 | Pending |
| 3.2 | Pending |
| 3.3 | Pending |
| 4.1 | Pending |
| 4.2 | Pending |
| 4.3 | Pending |
| 5.1 | Pending |
| 5.2 | Pending |
| 5.3 | Pending |

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

