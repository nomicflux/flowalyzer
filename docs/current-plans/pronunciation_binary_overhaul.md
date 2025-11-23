# Pronunciation Binary Overhaul Plan

Goal: rebuild the pronunciation binary so it deterministically loads reference audio, streams mic audio chunk-by-chunk with a one-chunk look-back, extracts real features for both, and compares aligned frames without heuristics, fallbacks, or fake data. Each substep replaces existing code (delete freely) and must leave no dead code behind.

## Substeps

### 1) Deterministic feature extraction baseline
- Replace `src/pronunciation/features` with a minimal module defining `FeatureConfig { frame_len_samples, hop_samples }` derived from sample rate, plus `FeatureExtractor`.
- APIs: `extract_reference(samples: &[f32], sample_rate: u32, cfg: FeatureConfig) -> ReferenceFeatures` and `extract_chunk(prev_tail: &[f32], chunk: &[f32], sample_rate: u32, cfg: FeatureConfig) -> ChunkFeatures`. `prev_tail` must be exactly `frame_len - hop`; panic on mismatched lengths, empty chunks, or `sample_rate == 0`.
- Outputs include vectors for energy/pitch and matching frame start indices in samples; energy and pitch share identical framing and lengths. No padding, zero-filling, smoothing, or clamping—pure math only. Inline comments should document framing math.
- Tests: exact frame count math given buffer length and hop, boundary carry-over (context used only for windowing; returned frames anchored to the new chunk), and deterministic equality across repeated calls with identical inputs.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 2) Alignment math: index-to-index only
- Rebuild `src/pronunciation/alignment/mod.rs` to accept `ReferenceFeatures` and `ChunkFeatures` with absolute frame indices/hop and compute per-frame comparisons strictly by matching indices (no DTW, no smoothing, no defaults). Panic if the requested reference window overruns available frames.
- Metrics: `energy_error = learner_rms - reference_rms`; `pitch_delta_cents = 1200 * log2(learner/ref)` with explicit panic on non-positive or NaN inputs; `similarity = 1.0 - |energy_error| / max(|reference_rms|, eps)` with a single documented `eps`. Any aggregate is a documented mean of real values; remove confidence heuristics.
- `AlignmentReport` carries aligned slices (reference/learner energy and pitch), per-frame error bands, start/end frame indices, hop_ms, and `global_time_offset_ms` derived from the sample counter. Remove phoneme placeholders and zeroed bands.
- Tests: assert index alignment (frame i to reference i), panic on insufficient reference, deterministic metric values for known vectors, and hop_ms propagation into reports.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 3) SessionEngine rewrite (stateless + one-chunk look-back)
- Replace `src/pronunciation/session/engine.rs` with an engine holding only immutable `ReferenceFeatures`, a sample counter, and a fixed tail buffer (`frame_len - hop` samples). No other history or buffers.
- `process_chunk(chunk: &[f32])`: validate non-empty; prepend tail for windowing; extract chunk features; compute starting frame index from `global_sample_counter / hop_samples`; align via reference features (panic on overrun); emit `AlignmentReport`; update tail to the last `frame_len - hop` samples of the concatenated tail+chunk; advance sample counter by `chunk.len()`.
- Remove VecDeque boundary logic and any on-the-fly reference slicing; all reference access uses precomputed features and frame indices.
- Tests: tail carry-over correctness at chunk boundaries, frame index progression per chunk, panic on empty chunk/sample-rate mismatch/reference exhaustion, deterministic reports across repeated calls.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 4) Runtime/controller hardening (capture → engine → snapshots)
- Rework `src/pronunciation/session/runtime.rs` to precompute reference features before spawning threads, construct `SessionEngine` once, and treat all capture/resample/runtime errors as fatal (panic/unwrap). Remove default/empty snapshots; only real alignment data reaches the UI.
- Commands: start/stop/shutdown/replay/variant toggle only; remove recipe plumbing unless fully wired. Capture loop resamples deterministically to engine rate, enforces fixed chunk sizes, and stops when reference frames are exhausted (panic if more audio arrives).
- Snapshot emission: send only real `AlignmentReport` objects; no “error” string propagation. Replay/variant toggle failures should panic rather than fall back.
- Tests: mock `CaptureSource` to assert fatal handling on capture/resample errors, stop when reference is consumed, correct global offsets from the engine sample counter, and deterministic one-chunk processing per loop iteration.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 5) UI wiring to real data with rolling history
- Rewrite `src/ui/screens/session.rs` to consume only real snapshots (no defaults). Maintain `VecDeque` histories (>=30s) for reference/learner energy, reference/learner pitch, energy_error/similarity, pitch_delta_cents; derive time using hop_ms. Clear histories on start/stop/variant change.
- Rendering: three panels—(1) static reference overview, (2) rolling learner/reference timelines, (3) comparison/error bands. No placeholders or “loading” badges; if data is missing, surface a fatal error and exit.
- Remove recipe/flowalyzed controls unless backed by real data in this step; keep UI focused on capture/compare display. Require first real snapshot before plotting.
- Tests: feed synthetic snapshots to verify 30s trimming, timeline mapping using hop_ms, and fatal behavior when given empty vectors.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.

### 6) CLI entry + docs coherence
- Update `src/bin/pronunciation.rs` (and README/docs) to describe the deterministic pipeline: load reference, precompute features, start capture, stream comparisons. Remove flags/paths implying heuristics or fallbacks; fail fast on missing files, sample-rate mismatches, or empty reference.
- Document invariants in `docs/pronunciation/` or README: no fake data, no fallbacks, stateless engine with one-chunk look-back, fatal errors on invalid inputs.
- Tests: argument validation (missing reference, unsupported sample rate), and a smoke test using a synthetic reference plus mock capture running through runtime/controller/UI without placeholders.
run cargo test -all and cargo clippy -all, and fix any warnings or errors.
