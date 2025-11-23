# Pronunciation Binary Overhaul Plan (Fail-Fast, No Fallbacks)

Goal: rebuild the pronunciation binary so it deterministically loads reference audio, streams mic audio chunk-by-chunk with a single look-back tail, extracts real features, and compares aligned frames with zero heuristics, zero padding, zero fake data, and no defensive smoothing. Every error must abort immediately. Each substep introduces only the code required for that substep; no dead code.

## Substeps

### 1) Feature extraction (fail-fast, no padding)
- Replace `src/pronunciation/features` with `FeatureConfig { frame_len_samples, hop_samples }` derived from sample rate, plus `FeatureExtractor`.
- APIs: `extract_reference(samples: &[f32], sample_rate: u32, cfg) -> ReferenceFeatures` and `extract_chunk(prev_tail: &[f32], chunk: &[f32], sample_rate: u32, cfg) -> ChunkFeatures`. Preconditions: `sample_rate > 0`; `prev_tail.len() == frame_len - hop`; `chunk.len() >= frame_len_samples`; panic on violation. No zero-padding, no smoothing.
- Outputs: energy and pitch vectors with matching `frame_starts` (in samples) for every full frame. Energy/pitch share identical framing. Pitch must compute from the actual frame; if the frame cannot support a period, panic (do not return 0).
- Tests: frame count math for reference/chunk; boundary carry-over uses only real `prev_tail`; deterministic equality on repeat calls; panics on short inputs or wrong tail length.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 2) Alignment (pure index match, no heuristics)
- Rewrite `src/pronunciation/alignment/mod.rs` to accept `ReferenceFeatures`, `ChunkFeatures`, `start_frame_idx`, `global_offset_ms`, `sample_rate`, `hop_samples`.
- Preconditions: frame counts/starts match; `start_frame_idx + learner.len() <= reference.len()`; panic otherwise. No trimming or padding.
- Metrics: `energy_error = learner - reference`; `similarity = 1.0 - |energy_error| / |reference|` (no eps/clamp); `pitch_delta_cents = 1200 * log2(learner/ref)` using raw ratio. Do not guard against zero/NaN beyond the structural assertions—let invalid data fail.
- `AlignmentReport`: aligned reference/learner energy and pitch slices, `energy_error`, `similarity_band`, `pitch_delta_cents` (as `contour_band`), `start_frame_idx`, `end_frame_idx`, `hop_ms`, `global_time_offset_ms`, `total_duration`. No confidence/phoneme placeholders.
- Tests: panic on reference underrun or misaligned frame starts; deterministic metric values for known vectors; hop_ms derived from sample_rate/hop_samples.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 3) SessionEngine (single tail, precomputed reference)
- Replace `src/pronunciation/session/engine.rs` with an engine that holds: immutable `ReferenceFeatures`, fixed tail buffer (`frame_len - hop`), sample counter, feature cfg/extractor. No other state/history.
- API: `new(reference_samples, config)` precomputes reference features and asserts they contain at least one frame; `process_chunk(chunk)` asserts chunk is non-empty and `tail` is present; computes start_frame_idx = global_sample_counter / hop; extracts learner features with current tail; aligns; updates tail to last `frame_len - hop` samples of (tail+chunk); advances sample counter by chunk.len(). Panic on any precondition failure or reference exhaustion.
- Provide `seed_tail(samples)` to set the initial tail (must be >= required length) and set the counter accordingly; no zero-fill.
- Tests: tail carry-over across chunk boundaries; panic on missing/short tail or exhausted reference; frame index progression per chunk; deterministic reports on repeated calls.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 4) Runtime (fail-fast, no placeholders)
- Rewrite `src/pronunciation/session/runtime.rs` to precompute reference features once before spawning threads; build `SessionEngine` once. Commands limited to start/stop/shutdown/replay.
- Capture loop: resample deterministically to engine rate; collect exact chunk sizes; seed engine tail from the first chunk segment (panic if insufficient); stop processing and panic if reference frames would be exceeded. Any capture/resample/runtime error panics—remove error fields and default snapshots.
- Snapshot emission: only real `AlignmentReport` instances from the engine; do not send empty/default snapshots. Replay failures panic.
- Tests: mock `CaptureSource` to assert panic on capture/resample errors, panic on reference exhaustion, correct offsets derived from sample counter, and one-chunk-per-iteration behavior.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 5) UI (real data only, rolling history)
- Rewrite `src/ui/screens/session.rs` to refuse default/empty snapshots. Maintain `VecDeque` histories (>=30s) for reference/learner energy, reference/learner pitch, `energy_error`, `similarity`, `pitch_delta_cents`, using hop_ms for timing. Clear histories on start/stop/replay events.
- Rendering: three panels—reference overview, rolling timelines, comparison/error bands. No placeholders, loading badges, or fabricated values. If no real snapshot has arrived, exit with a fatal error instead of drawing.
- Tests: feed synthetic real snapshots to verify history trimming, hop_ms-based timing, and fatal behavior on empty inputs.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 6) CLI and docs (explicit fail-fast contract)
- Update `src/bin/pronunciation.rs` and docs to describe the deterministic pipeline: load reference, precompute features, start capture, stream chunk comparisons. Validate args (existing file, non-empty clip) and fail immediately on error; remove options suggesting heuristics/fallbacks.
- Document invariants: no fake data, no fallbacks, one-chunk look-back, fail-fast on invalid inputs or exhausted reference.
- Tests: argument validation failures, and a smoke test with synthetic reference + mock capture exercising the runtime path without placeholders.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.
