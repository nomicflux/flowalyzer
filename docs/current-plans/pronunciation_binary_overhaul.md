# Pronunciation Binary Overhaul Plan (Stateless, Source-Only, Guardrail-Free)

All rules are binary: MUST do X or MUST NOT do Y. Every substep lists acceptance checks and explicit examples. Guardrails are defined as any code that prevents a natural panic or fabricates data (pre-checks, clamping, padding, defaults, saturating math, placeholder emissions, synthetic frames/metrics).

## Key Architectural Concerns (apply to every change and test)
- MUST use source-only data: no fabricated frames, zero-padding, clamping, smoothing, defaults, or placeholders. Tail samples MUST contribute when frames span tail+chunk.
- MUST keep only minimal inevitable state: engine keeps tail buffer + global sample counter; UI may keep bounded history (~30s) only. No caches, speculative buffers, or smoothing windows.
- MUST avoid guardrails: no precondition asserts/guards, no saturating_sub/clamp/max/min/defaults to dodge panics, no padding. Invalid inputs fail where slices/divisions naturally panic.
- MUST compute raw metrics: direct energy/pitch math; no epsilons/thresholds/normalization.
- MUST keep minimal surface: commands limited to start/stop/shutdown/replay; snapshots emitted only after real alignment; no placeholder/default snapshots.
- Litmus test per line: does it prevent a panic or synthesize data? If yes, it violates the plan.

## Guardrail Litmus (applies to every change)
- If a line prevents a natural panic (length/shape/rate/NaN check, early return, default/fallback like `unwrap_or`, clamp/saturating, padding/zeroing, placeholder snapshot/metric), it is a guardrail and must be removed.
- If a line fabricates or substitutes data (padding, smoothing, thresholds to skip work, placeholder values), it is a guardrail and must be removed.
- Only the defined math/slice is allowed; invalid inputs must panic where the language/runtime naturally would.

## Phase Preflight & Compliance Checklist (run per phase before declaring done)
- Confirm updated tests encode the new invariants; legacy tests enforcing old behavior are rewritten or removed. Cargo test must pass because invariants are correct, not because old expectations were kept.
- Manually audit touched code/tests against the guardrail litmus: delete any branches/guards/defaults/fallbacks/padding/placeholder emissions that would avoid natural panics or synthesize data.
- Re-read the phase’s cross-cutting reminders and acceptance examples (e.g., hop-phase/tail example) and ensure the implementation and tests satisfy them verbatim.
- Verify no placeholder snapshots/metrics/commands remain in the touched files.

## Step 1: Feature Extraction Purity (guardrail-free, hop-phase preserved)
**Cross-cutting reminders:** source-only data, hop-phase preservation, no fabricated frames/metrics, no guardrails (no asserts/pre-checks/clamp/saturating_sub/max/min/Default). Legacy tests must be rewritten to match the new invariants.

### 1.1 FeatureConfig contract (file: `src/pronunciation/features/mod.rs`)
- Deliverable: `FeatureConfig::from_sample_rate` derives `frame_len_samples`/`hop_samples` by proportional math to 16k constants only.
- Actions: remove any `Default` impls that fabricate configs; forbid clamping/saturating math or guards; keep function pure. Do not add new helpers/exports beyond `FeatureConfig`, `FeatureExtractor`, `ReferenceFeatures`, `ChunkFeatures`.
- Acceptance: function body is pure math; invalid sample_rate fails later via natural slice panic. Grep confirms no `Default`, clamp/saturating calls, or asserts.

### 1.2 Reference extraction algorithm (file: `src/pronunciation/features/mod.rs`, impl `FeatureExtractor`)
- Deliverable: reference features computed only from real samples, no padding or partial frames.
- Actions: starts = `{ s | s=i*hop, s+frame_len <= samples.len() }` starting at 0; energy/pitch from `samples[s..s+frame_len]` directly; remove any guards/length checks.
- Acceptance example: samples shorter than frame_len cause natural slice panic (no manual assert). No zero-fill/smoothing/threshold skips in code.

### 1.3 Chunk extraction preserves hop phase (file: `src/pronunciation/features/mod.rs`, impl `FeatureExtractor`)
- Deliverable: chunk frame starts derived from full window (tail+chunk) without re-anchoring; tail contributes to first frames.
- Actions: window = tail + chunk; `full_starts = { s | s=i*hop, s+frame_len <= window.len() }`; map to chunk with preserved hop offset so boundary frames spanning tail+chunk are kept and tail contributes; first chunk start in the tail_len=864, hop=160, frame_len=1024 example remains 96 with hop spacing. Use absolute `s` for energy/pitch; no guards/clamps.
- Acceptance: starts preserve hop phase and keep the boundary frame that includes tail samples; first start matches the example; no re-anchoring; no guardrail ops present.

### 1.4 Raw frame math only (file: `src/pronunciation/features/mod.rs`)
- Deliverable: frame energy/pitch computed directly from frame slices with no heuristics.
- Actions: energy = sum of squares; pitch via autocorrelation bounds from sample_rate/frame_len only. Strip eps/clamps/thresholds/asserts.
- Acceptance: code contains direct math only; invalid inputs panic naturally via iteration/division/slice.

### 1.5 Feature tests pin phase and tail (file: `tests/feature_extraction.rs`)
- Deliverable: tests enforce hop-phase preservation and tail contribution; legacy assumptions removed.
- Actions: add `#[should_panic]` for reference/chunk shorter than frame_len (natural panic). Add non-multiple tail/hop test (tail=864, hop=160) asserting first `frame_starts` == 96 and hop spacing; assert first frame energy >0 when tail nonzero and chunk zeros. Delete/replace tests expecting `frame_starts.first()==0` or zero energy with tail.
- Acceptance: cargo test passes with new invariants; grep shows no guardrail patterns (`saturating`, `clamp`, `max(`, `min(`, `Default`, asserts) in feature code/tests.

## Step 2: Alignment Without Heuristics
**Cross-cutting reminders:** source-only data, hop-phase preservation, respect provided frame_starts/start_frame_idx, no guardrails (no asserts/pre-checks/clamp/saturating_sub/max/min/Default). Legacy tests must be rewritten.

### 2.1 Alignment API trusts inputs (file: `src/pronunciation/alignment/mod.rs`)
- Deliverable: `align_features(reference, learner, start_frame_idx, global_offset_ms, sample_rate, hop_samples) -> AlignmentReport` with no optional/guarding params.
- Actions: slice reference with `start_frame_idx..start_frame_idx+learner.len()` directly; no precondition asserts/clamps. Restate spacing invariant: frame_starts spaced by hop and aligned to start_frame_idx; do not recompute/ignore provided starts.
- Acceptance: out-of-range or mismatch panics naturally via slice; no guardrails present.

### 2.2 Metrics are direct math (file: `src/pronunciation/alignment/mod.rs`)
- Deliverable: energy/pitch error bands computed with raw math only.
- Actions: `energy_error = learner.energy[i] - ref_energy[i]`; `similarity_band = 1.0 - |error|/|ref_energy|`; `contour_band = 1200*log2(learner_pitch/ref_pitch)`; `hop_ms = hop_samples as f32 / sample_rate as f32 * 1000.0`; `total_duration = hop_ms * learner.len()`. No eps/clamps/thresholds.
- Acceptance example: hop=160, sr=16_000 ⇒ hop_ms=10.0; len=5 ⇒ total=50.0. Divide-by-zero panics naturally if ref pitch/energy zero.

### 2.3 AlignmentReport is source-only (file: `src/pronunciation/session/snapshot.rs`)
- Deliverable: report contains only real metrics/indices; no defaults/placeholders.
- Actions: keep fields: reference_energy, learner_energy, reference_pitch, learner_pitch, energy_error, similarity_band, contour_band, start_frame_idx, end_frame_idx, hop_ms, global_time_offset_ms, total_duration. Remove confidence/phoneme/recipe/defaults. No `Default` impl that fabricates data.
- Acceptance: cannot construct without real alignment vectors; serialization reflects actual data only.

### 2.4 Alignment tests codify contract (file: `tests/alignment.rs`)
- Deliverable: tests that panic on bad shapes and check numeric outputs exactly.
- Actions: add `#[should_panic]` for length mismatch/out-of-range start_frame_idx (natural panic). Add deterministic metric test (ref energy [1,1], learner [2,2] ⇒ error [1,1], similarity [0,0]; ref pitch [100], learner [200] ⇒ contour 1200; hop_ms/total_duration exact). Restate hop-phase invariant in test names/comments. Remove legacy tests relying on confidence/placeholder/re-anchoring.
- Acceptance: cargo test passes with new invariants; grep shows no guardrail patterns in alignment code/tests.

## Step 3: Session Engine Statelessness
**Cross-cutting reminders:** source-only data, hop-phase preservation, no guardrails (no asserts/pre-checks/clamp/saturating_sub/max/min/Default), legacy tests rewritten. start_frame_idx/global_time_offset_ms must follow the numeric example unless the math differs.

### 3.1 Engine owns only minimal state (file: `src/pronunciation/session/engine.rs`)
- Deliverable: engine holds only reference, tail, sample_rate, feature_cfg, extractor, global_sample_counter.
- Actions: remove queues/caches/speculative buffers; construct with precomputed `ReferenceFeatures` only. No guards around short references; failure happens when slicing later.
- Acceptance: struct has no extra buffers; construction does not pad/guard; short reference panics naturally when used.

### 3.2 Tail seeding is deterministic (file: `src/pronunciation/session/engine.rs`)
- Deliverable: `seed_tail` copies the last `required_tail_len = frame_len - hop` samples exactly and sets `global_sample_counter = samples.len()`.
- Actions: compute required_tail_len with pure math (no saturating_sub/clamp). Copy tail via slice; no padding or guards. No conditional seeding elsewhere.
- Acceptance: tail equals exact suffix; missing/short input panics naturally via slice; counter equals input len.

### 3.3 Chunk processing uses counter-derived indices (file: `src/pronunciation/session/engine.rs`)
- Deliverable: `process_chunk` builds window, extracts chunk features, aligns using counter-derived start_frame_idx, and advances tail/counter without guards.
- Actions: require tail pre-seeded (no fallback). window = tail + chunk; `start_frame_idx = global_sample_counter / hop_samples - 1`; `global_time_offset_ms = global_sample_counter as f32 / sample_rate as f32 * 1000.0`; call `align_features`; update tail to last `required_tail_len` of window; increment counter by chunk len.
- Acceptance example: tail_len=864, hop=160, frame_len=1024 ⇒ after seeding counter=864, start_frame_idx=4; after chunk len L, counter=864+L; start_frame_idx=(counter/hop)-1; offset_ms=(counter/sample_rate)*1000. First frame energy for zero chunk + nonzero tail is nonzero. No guardrails present.

### 3.4 Engine tests lock invariants (file: `tests/session_engine.rs`)
- Deliverable: tests enforce tail requirement, hop-phase, tail contribution, and counter/offset math; legacy expectations removed.
- Actions: add `#[should_panic]` when `process_chunk` called before `seed_tail`; add `#[should_panic]` when reference < frame_len. Add non-multiple tail/hop test (tail=864, hop=160) asserting first chunk-local start matches preserved phase and tail-after-processing equals last `frame_len - hop` of window. Assert first frame energy includes tail when chunk zeros; assert start_frame_idx advances by chunk_len/hop; assert `global_time_offset_ms` == `global_sample_counter / sample_rate * 1000` after each chunk. Remove tests expecting re-anchoring or guard behavior.
- Acceptance: cargo test passes with new invariants; grep shows no guardrail patterns in engine code/tests.

## Step 4: Runtime Thread Behavior
**Cross-cutting reminders:** source-only data, hop-phase preservation, bounded UI history (~30s), no guardrails (no asserts/pre-checks/clamp/saturating_sub/max/min/Default). Legacy tests must be rewritten.

### 4.1 Startup builds full pipeline (file: `src/pronunciation/session/runtime.rs`)
- Deliverable: runtime constructs `FeatureConfig`, `FeatureExtractor`, `ReferenceFeatures`, and `SessionEngine` upfront; no placeholder snapshots.
- Actions: remove any setup guards/retries/default snapshot emission. Keep runtime state minimal; reiterate UI history bound if touched.
- Acceptance: no snapshot before real alignment; IO/resample errors panic naturally; no guardrails in code (grep check).

### 4.2 Start command seeds real tail only (file: `src/pronunciation/session/runtime.rs`)
- Deliverable: start buffers until `required_tail_len`, seeds tail, then processes remaining buffer as first chunk with preserved hop phase.
- Actions: no padding/shortening; propagate capture/resample errors as panics; do not emit default/error snapshots.
- Acceptance: first snapshot only after real tail seeding; simulated capture errors panic; no fallback branches.

### 4.3 Command/snapshot surface stays minimal (files: `src/pronunciation/session/runtime.rs`, `src/pronunciation/session/snapshot.rs`)
- Deliverable: command enum limited to Start/Stop/Shutdown/ReplayReference/StopReplay; snapshots only from real alignment data.
- Actions: remove extra commands; remove recipe/error/placeholder fields; no `Default` for snapshots.
- Acceptance: cannot construct snapshot without alignment data; no extra commands remain; grep shows no guardrails.

### 4.4 Runtime tests ban fallbacks (file: `tests/stateless_runtime.rs` or similar)
- Deliverable: tests that panic on insufficient start buffer and capture/resample errors; first snapshot must follow real tail seeding and contain real vectors.
- Actions: add `#[should_panic]` for Start with < required_tail_len; add panic test on capture/resample error. Add test that first snapshot has non-empty alignment vectors and occurs only after seeding; processing stops naturally when reference exhausted. Remove legacy tests expecting default/error snapshots or re-anchoring.
- Acceptance: cargo test passes with new invariants; grep shows no guardrail patterns in runtime code/tests.

## Step 5: CLI and Documentation Contract
**Cross-cutting reminders:** source-only data, hop-phase preservation, bounded UI history (~30s), no guardrails (no asserts/pre-checks/clamp/saturating_sub/max/min/Default). Legacy tests/docs must be rewritten; cargo test must pass with updated expectations.

### 5.1 CLI entry is strict (file: `src/bin/pronunciation.rs`)
- Deliverable: binary requires explicit inputs; no demo/default/zero-padding fallbacks.
- Actions: load reference once, pass directly to runtime; remove flags/paths implying fallbacks; let IO/resample errors bubble (no guards).
- Acceptance: binary cannot start without user-provided inputs; no placeholder snapshots/paths; panics propagate.

### 5.2 Wiring passes data directly (file: `src/bin/pronunciation.rs`)
- Deliverable: CLI options map directly to `FeatureConfig`/engine/runtime with no warm-up snapshots or guards.
- Actions: ensure UI displays only after real processing; reiterate bounded UI history (~30s) if UI touched; remove intermediate defaults.
- Acceptance: runtime receives exact user inputs; first UI data follows real alignment; no guardrails in wiring (grep check).

### 5.3 Document the stateless contract (file: `docs/pronunciation/README.md` or new doc)
- Deliverable: documentation that encodes the invariants and examples subagents must follow.
- Actions: describe source-only processing, minimal command surface, tail seeding requirement, bounded UI history, hop-phase preservation with explicit example (tail=864, hop=160 ⇒ first chunk start 96). Warn against guardrails/placeholders; state errors panic naturally.
- Acceptance: doc contains explicit examples and banned patterns; matches plan invariants; references the hop/tail example and no-guardrail rule.

## Learned From Mistakes
- Always prioritize the stated contract over legacy behavior. Rewrite or remove legacy tests that encode old heuristics before coding; do not adapt implementation to keep them green. Encode the new invariants in tests first.
- Guardrails are any code that prevents natural failures or fabricates data. If a line changes behavior for invalid inputs (pre-checks, clamps, saturating math, defaults, padding), it violates the contract—delete it and let slice/index/math panics surface.
- Derive computations from real data with the prescribed algorithm, not shortcuts. For chunk starts, scan the full tail+chunk window from 0 at hop spacing, subtract tail length, and drop negatives—no re-anchoring or tail-offset seeding.
- Do not invent expectations on raw metrics (e.g., asserting energy > 0). Accept whatever the real samples yield; enforce only structural invariants (frame start grid, hop spacing) and natural panics.
- Apply the same discipline in future phases: restate invariants, remove old assumptions, ban guardrails, and prove behavior with tests that reflect the contract rather than past code.
- Never delete files without explicit user permission.
- You are given problem scope. You must stay within the problem scope. Before every action, you must confirm whether the
  action fits the problem scope as you have explicitly repeated back to the user. If not, you MUST discard the action.
- "Urgency" is not an issue. We are taking as long as necessary to do this CORRECTLY, because the time it would take to
  debug a RUSHED version is even longer.
- When the plan appears contradictory (e.g., hop-phase start vs. tail contribution), stop and ask the user for clarification before coding—do not guess or “fix” around the spec.
- When tests diverge from the plan, update the tests to match the plan rather than bending implementation to legacy expectations.
- Treat silent filters/clamps (like dropping negative starts) as guardrails; removing asserts is not enough.
- Preserve tail contribution and hop phase exactly as written; do not reinterpret examples or adjust offsets without confirmation.
