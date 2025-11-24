# Pronunciation Binary Overhaul Plan (Stateless, Source-Only, Guardrail-Free)

All rules are binary: MUST do X or MUST NOT do Y. Every substep lists acceptance checks and explicit examples. Guardrails are defined as any code that prevents a natural panic or fabricates data (pre-checks, clamping, padding, defaults, saturating math, placeholder emissions, synthetic frames/metrics).

## Key Architectural Concerns (apply to every change and test)
- MUST use source-only data: no fabricated frames, zero-padding, clamping, smoothing, defaults, or placeholders. Tail samples MUST contribute when frames span tail+chunk.
- MUST keep only minimal inevitable state: engine keeps tail buffer + global sample counter; UI may keep bounded history (~30s) only. No caches, speculative buffers, or smoothing windows.
- MUST avoid guardrails: no precondition asserts/guards, no saturating_sub/clamp/max/min/defaults to dodge panics, no padding. Invalid inputs fail where slices/divisions naturally panic.
- MUST compute raw metrics: direct energy/pitch math; no epsilons/thresholds/normalization.
- MUST keep minimal surface: commands limited to start/stop/shutdown/replay; snapshots emitted only after real alignment; no placeholder/default snapshots.
- Litmus test per line: does it prevent a panic or synthesize data? If yes, it violates the plan.

## Step 1: Feature Extraction Purity (guardrail-free, hop-phase preserved)
**Cross-cutting reminders for this phase:** no guardrails (no saturating/clamp/max/min/Default/asserts), source-only data, hop-phase preservation, no fabricated frames/metrics, legacy tests must be rewritten to reflect new invariants.

### 1.1 Feature types without heuristics (file: `src/pronunciation/features/mod.rs`)
- MUST expose only `FeatureConfig`, `FeatureExtractor`, `ReferenceFeatures`, `ChunkFeatures`; remove extra helpers/exports.
- MUST compute `frame_len_samples`/`hop_samples` in `from_sample_rate` via proportional math to 16k constants. MUST NOT use saturating/clamp/max/min/defaults or asserts. Acceptance: function is pure math; any invalid rate fails later via slice panic.
- MUST NOT implement `Default` that fabricates configs or features. Acceptance: no `Default` impls for `FeatureConfig`, `ReferenceFeatures`, `ChunkFeatures`, `AlignmentReport` that set sample values.
- Architectural tie: minimal surface, no guardrails.

### 1.2 Reference extraction uses only real frames (file: `src/pronunciation/features/mod.rs`, impl `FeatureExtractor`)
- MUST generate starts: `starts = { s | s = i*hop, s+frame_len <= samples.len() }` beginning at `s=0`. MUST NOT pad or emit partial frames.
- MUST derive energy/pitch from `samples[s..s+frame_len]` directly. MUST NOT zero-fill/smooth/threshold-skip.
- MUST NOT pre-check lengths or assert; insufficient input MUST panic at slice. Acceptance example: samples len < frame_len => natural slice panic.
- Architectural tie: source-only, raw metrics, no guardrails.

### 1.3 Chunk extraction preserves hop phase (file: `src/pronunciation/features/mod.rs`, impl `FeatureExtractor`)
- MUST build `window = tail + chunk`. Compute starts over full window: `full_starts = { s | s = i*hop, s+frame_len <= window.len() }` starting at `s=0`.
- MUST map to chunk-local: `chunk_starts = { s - tail_len | s in full_starts, s - tail_len >= 0 }`. MUST NOT re-anchor to 0 based on chunk. Explicit example: tail_len=864, hop=160, frame_len=1024 ⇒ full starts: 0,160,320,480,640,800,960,1120…; after subtracting tail: -864,-704,-544,-384,-224,-64,96,256… keep {96,256,…}. First chunk start MUST be 96, not 0.
- MUST compute energy/pitch from `window[s..s+frame_len]` using absolute `s`; frames spanning tail+chunk MUST include tail samples. MUST NOT use saturating_sub/clamp/guards; if too short, natural slice panic occurs.
- Acceptance checks: starts preserve hop phase from full window; first start matches example when tail_len not multiple of hop; no re-anchoring; no guardrail ops present.
- Architectural tie: source-only, minimal state, no guardrails.

### 1.4 Frame math is raw (file: `src/pronunciation/features/mod.rs`)
- MUST keep `frame_energy` as sum of squares (no sqrt/avg unless already required) over the frame slice; MUST keep `frame_pitch` as autocorrelation-based search with bounds derived only from `sample_rate` and `frame_len`.
- MUST NOT add asserts/guards/saturating math; invalid inputs fail via iteration/division panic. Acceptance: no clamps/eps; code uses direct math only.
- Architectural tie: raw metrics, no guardrails.

### 1.5 Feature tests enforce hop phase and tail contribution (file: `tests/feature_extraction.rs`)
- MUST include `#[should_panic]` for `extract_reference` and `extract_chunk` with input shorter than `frame_len`; panics must be natural slice panics (no manual asserts).
- MUST add non-multiple tail/hop case (e.g., tail_len=864, hop=160) asserting first `frame_starts` equals 96 (from example) and starts spaced by hop. MUST assert first frame energy is >0 when tail nonzero & chunk zeros (tail contributes).
- MUST remove/replace legacy tests that assert `frame_starts.first()==0` or expect zero energy when tail has signal; cargo test must pass only with corrected tests, not by preserving unfit legacy expectations.
- Architectural tie: source-only, hop-phase preservation, no guardrails.
- Phase preflight: grep Phase 1 files for `saturating`, `clamp`, `max(`, `min(`, `Default` impls, and precondition asserts; remove/replace anything that dodges natural panics.

## Step 2: Alignment Without Heuristics
**Cross-cutting reminders for this phase:** no guardrails (no saturating/clamp/max/min/Default/asserts), source-only data, hop-phase preservation, respect provided frame_starts/start_frame_idx, legacy tests must be rewritten to reflect new invariants.

### 2.1 Alignment API uses raw shapes (file: `src/pronunciation/alignment/mod.rs`)
- MUST define `align_features(reference, learner, start_frame_idx, global_offset_ms, sample_rate, hop_samples) -> AlignmentReport` with no optional params.
- MUST NOT add guards/precondition asserts/clamps/saturating math. Mismatches MUST panic via slicing. Acceptance: direct slicing `reference[start_frame_idx..start_frame_idx+learner.len()]` with no checks.
- MUST NOT use placeholder/confidence/default fields. Architectural tie: no guardrails, minimal surface, raw metrics.
- MUST restate spacing invariant: reference.frame_starts and learner.frame_starts are spaced by hop and align with start_frame_idx; alignment must respect provided starts without recomputing/clamping or inserting guards (trust inputs).

### 2.2 Metrics are direct calculations (file: `src/pronunciation/alignment/mod.rs`)
- MUST compute `energy_error = learner.energy[i] - ref_energy[i]`, `similarity_band = 1.0 - |error|/|ref_energy|`, `contour_band = 1200 * log2(learner_pitch/ref_pitch)`.
- MUST derive `hop_ms = hop_samples as f32 / sample_rate as f32 * 1000.0` and `total_duration = hop_ms * learner.len()`. Example: hop=160, sr=16_000 ⇒ hop_ms=10.0; len=5 ⇒ total=50.0.
- MUST NOT clamp/epsilon/threshold. Acceptance: zeros propagate; divide-by-zero panics naturally.
- Architectural tie: raw metrics, no guardrails.

### 2.3 AlignmentReport carries only real metrics (file: `src/pronunciation/session/snapshot.rs`)
- MUST contain only: `reference_energy`, `learner_energy`, `reference_pitch`, `learner_pitch`, `energy_error`, `similarity_band`, `contour_band`, `start_frame_idx`, `end_frame_idx`, `hop_ms`, `global_time_offset_ms`, `total_duration`.
- MUST remove defaults/placeholders/confidence/phoneme/recipe fields; MUST NOT implement `Default` that fabricates data.
- Acceptance: constructing report without real alignment data is impossible; serialization reflects real vectors/indices only.
- Architectural tie: source-only, minimal surface, raw metrics.

### 2.4 Alignment tests enforce raw behavior (file: `tests/alignment.rs`)
- MUST add `#[should_panic]` for length mismatches or out-of-range `start_frame_idx`; panics MUST be natural from slicing.
- MUST add deterministic metric test (e.g., ref energy [1,1], learner [2,2] ⇒ error [1,1], similarity [0,0]; ref pitch [100], learner [200] ⇒ contour 1200). Check hop_ms/total_duration exactly.
- MUST restate hop-phase invariant: `start_frame_idx` derived from engine counter/hop; frame_starts must be spaced by hop (no clamping).
- MUST delete/replace legacy tests that rely on confidence/placeholder behavior or re-anchoring starts; cargo test must pass with updated tests reflecting new invariants, not by keeping legacy expectations.
- Architectural tie: no guardrails, raw metrics, minimal surface.
- Phase preflight: grep alignment module/tests for `saturating`, `clamp`, `max(`, `min(`, `Default`, guard asserts; remove.

## Step 3: Session Engine Statelessness
**Cross-cutting reminders for this phase:** no guardrails (no saturating/clamp/max/min/Default/asserts), source-only data, hop-phase preservation, legacy tests must be rewritten to reflect new invariants. start_frame_idx/global_time_offset_ms must follow the numeric example unless math dictates otherwise.

### 3.1 Engine holds only minimal state (file: `src/pronunciation/session/engine.rs`)
- MUST keep fields: `reference: ReferenceFeatures`, `tail: Vec<f32>`, `sample_rate`, `feature_cfg`, `extractor`, `global_sample_counter`. MUST remove queues/boundary buffers/reference waveforms caching.
- MUST build engine with precomputed `ReferenceFeatures`; no lazy reference computation. MUST NOT add guards/saturating math; short reference panics later via slice.
- Acceptance: struct has no extra buffers; construction does not pad/guard; reference shorter than frame leads to natural panic when used.
- Architectural tie: minimal state, no guardrails.

### 3.2 Tail seeding is explicit (file: `src/pronunciation/session/engine.rs`)
- MUST provide `required_tail_len = frame_len - hop` (pure math, no saturating_sub). MUST NOT clamp.
- MUST implement `seed_tail` copying the last `required_tail_len` samples; if shorter, natural slice panic occurs. Set `global_sample_counter = samples.len()` with no guards.
- Acceptance: tail equals exact last samples; no padding/zeroing; no guard checks.
- Architectural tie: minimal state, no guardrails, source-only.

### 3.3 Chunk processing is pure (file: `src/pronunciation/session/engine.rs`)
- MUST assume tail already seeded; MUST NOT conditionally seed or fallback. If tail missing, natural panics should occur (no asserts).
- MUST build window = tail + learner, extract chunk features, compute `start_frame_idx = global_sample_counter / hop_samples - 1` (because chunk starts after tail’s last hop), call `align_features`.
- MUST update tail to last `required_tail_len` of window; increment `global_sample_counter` by chunk len. No bounds checks/guards/clamps.
- Acceptance: first frame energy for zero chunk + nonzero tail is nonzero; start_frame_idx follows counter/hop; failures are natural panics.
- MUST add numeric example: tail_len=864, hop=160, frame_len=1024: seed tail sets `global_sample_counter=864`; first chunk_len arbitrary; `start_frame_idx = 864 / 160 - 1 = 4`; alignment uses reference frames starting at index 4. After processing a chunk of length L, `global_sample_counter = 864 + L`. Use this pattern for all chunks: `start_frame_idx = (global_sample_counter / hop) - 1`, `global_time_offset_ms = global_sample_counter / sample_rate * 1000`.
- Architectural tie: minimal state, source-only, no guardrails, hop-phase preserved.

### 3.4 Engine tests cover phase and tail contribution (file: `tests/session_engine.rs`)
- MUST `#[should_panic]` when `process_chunk` called before `seed_tail`; MUST `#[should_panic]` when reference < frame_len.
- MUST test non-multiple tail/hop case: first chunk start matches preserved phase (e.g., tail 864, hop 160 ⇒ start_frame_idx offset reflects tail frames), tail after processing equals last `frame_len - hop` of window.
- MUST assert first frame energy includes tail when chunk zeros; start_frame_idx advances by `chunk_len / hop`. Remove legacy tests that assume re-anchoring or guarded behavior.
- MUST assert `global_time_offset_ms` equals `global_sample_counter / sample_rate * 1000` after each chunk. Example: tail 864 at 16k ⇒ offset after seeding = 864/16000*1000 = 54.0 ms; after chunk len L, offset = (864+L)/16000*1000.
- Architectural tie: minimal state, source-only, no guardrails. Cargo test must pass with corrected tests; do not preserve failing legacy assumptions.
- Phase preflight: grep engine/tests for `saturating`, `clamp`, `max(`, `min(`, `Default`, guard asserts; remove.

## Step 4: Runtime Thread Behavior
**Cross-cutting reminders for this phase:** no guardrails (no saturating/clamp/max/min/Default/asserts), source-only data, hop-phase preservation, bounded UI history (~30s), legacy tests must be rewritten to reflect new invariants. First snapshot must follow tail seeding and preserved hop phase.

### 4.1 Startup builds full pipeline upfront (file: `src/pronunciation/session/runtime.rs`)
- MUST precompute `FeatureConfig`, `FeatureExtractor`, `ReferenceFeatures` before spawning; construct `SessionEngine` with them.
- MUST NOT emit default/placeholder `SessionSnapshot`; MUST NOT add setup guards/retries. IO/resample errors MUST panic naturally.
- Acceptance: no snapshot sent before real alignment; no guards added.
- Architectural tie: minimal surface, no guardrails, source-only.
- MUST reassert UI bounded history constraint here: runtime must not maintain unbounded history; UI (outside scope here) should limit to ~30s if touched.
- Phase preflight: grep runtime for `saturating`, `clamp`, `max(`, `min(`, `Default`, guard asserts; remove.

### 4.2 Start command seeds tail from real capture only (file: `src/pronunciation/session/runtime.rs`)
- MUST buffer capture until `required_tail_len`, call `engine.seed_tail`, then process remaining buffer as first chunk. MUST NOT pad/shorten requirement.
- MUST propagate capture/resample errors as panics (no error fields/guards). No placeholder snapshots.
- Acceptance: first snapshot only after real tail seeding; simulated capture errors panic; no guard branches.
- Architectural tie: source-only, minimal surface, no guardrails.

### 4.3 Command surface and snapshots stay minimal (files: `src/pronunciation/session/runtime.rs`, `src/pronunciation/session/snapshot.rs`)
- MUST limit `SessionCommand` to `Start`, `Stop`, `Shutdown`, `ReplayReference`, `StopReplay`; remove others.
- MUST ensure snapshots only from real `AlignmentReport`; remove recipe/error/placeholder fields; no `Default` for snapshots.
- Acceptance: cannot construct snapshot without alignment; no extra commands remain.
- Architectural tie: minimal surface, source-only, no guardrails.

### 4.4 Runtime tests prove no fallbacks (file: `tests/stateless_runtime.rs` or similar)
- MUST `#[should_panic]` when `Start` gets fewer samples than `required_tail_len`; MUST `#[should_panic]` on capture/resample error.
- MUST assert first emitted snapshot has non-empty alignment vectors and occurs only after tail seeding; processing stops when reference exhausted (natural panic).
- MUST remove/replace legacy tests expecting default/error snapshots or re-anchoring behavior; cargo test must pass with updated tests, not by keeping unfit legacy behavior.
- Architectural tie: no guardrails, minimal surface, source-only.
- Phase preflight: grep runtime tests for `saturating`, `clamp`, `max(`, `min(`, `Default`, guard asserts; remove.

## Step 5: CLI and Documentation Contract
**Cross-cutting reminders for this phase:** no guardrails (no saturating/clamp/max/min/Default/asserts), source-only data, hop-phase preservation, bounded UI history (~30s), legacy tests/docs must be rewritten to reflect new invariants; cargo test must pass with updated tests only.

### 5.1 CLI entry avoids heuristics (file: `src/bin/pronunciation.rs`)
- MUST load reference once and pass directly to runtime; remove demo/default/silent fallbacks.
- MUST strip flags implying fallbacks (recipes, zero-padding). IO/resample failures MUST bubble; no pre-flight guards.
- Acceptance: binary cannot start without explicit inputs; no default snapshots/data paths; panics propagate.
- Architectural tie: minimal surface, no guardrails, source-only.

### 5.2 Runtime wiring is direct (file: `src/bin/pronunciation.rs`)
- MUST pass CLI options straight into `FeatureConfig`/engine/runtime with no intermediate guards or warm-up snapshots.
- MUST ensure UI displays only after real processing begins; no placeholder emissions.
- Acceptance: runtime receives exact user inputs; first UI data follows real alignment.
- MUST reiterate bounded UI history (~30s) if UI code is touched; no unbounded buffers.
- Architectural tie: minimal surface, no guardrails.
- Phase preflight: grep CLI/UI changes for `saturating`, `clamp`, `max(`, `min(`, `Default`, guard asserts; remove.

### 5.3 Document the stateless contract (file: `docs/pronunciation/README.md` or new doc)
- MUST state all features/alignment come solely from real audio; no padding/smoothing/fabrication.
- MUST describe minimal command surface, tail seeding requirement, bounded UI history (~30s), hop-phase preservation, and tail contribution example (e.g., tail 864/hop 160 ⇒ first chunk start 96).
- MUST warn against reintroducing guardrails/placeholders; errors panic naturally.
- Acceptance: doc includes explicit examples and banned patterns (no guardrails); matches plan invariants.
- Architectural tie: all concerns.

## Learned From Mistakes (this session)
- Always prioritize the stated contract over legacy behavior. Rewrite or remove legacy tests that encode old heuristics before coding; do not adapt implementation to keep them green. Encode the new invariants in tests first.
- Guardrails are any code that prevents natural failures or fabricates data. If a line changes behavior for invalid inputs (pre-checks, clamps, saturating math, defaults, padding), it violates the contract—delete it and let slice/index/math panics surface.
- Derive computations from real data with the prescribed algorithm, not shortcuts. For chunk starts, scan the full tail+chunk window from 0 at hop spacing, subtract tail length, and drop negatives—no re-anchoring or tail-offset seeding.
- Do not invent expectations on raw metrics (e.g., asserting energy > 0). Accept whatever the real samples yield; enforce only structural invariants (frame start grid, hop spacing) and natural panics.
- Apply the same discipline in future phases: restate invariants, remove old assumptions, ban guardrails, and prove behavior with tests that reflect the contract rather than past code.
