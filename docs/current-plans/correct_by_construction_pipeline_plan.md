# Correct-by-Construction Pipeline Plan

**Status:** Draft (2025-??-??)

## Problem Statement
The current ingest → chunk → slice path mixes floating-point heuristics with runtime guesses. Pause detection gets parameters in the wrong order, `ChunkConfig` invents ratios (1.5× max, 0.3× overshoot), and slicer boundaries silently truncate. Even though the greedy streaming design is appropriate, the math backing every decision must be exact integers tied to the real sample clock, not heuristics. This plan rewrites the existing workflow so each step is derived directly from measured rates or transcript metadata without adding any new functionality.

## Phase 1 – Frame-Accurate Timebase Types
**Problem to Solve:** `AudioData`, `ChunkBoundary`, and the slicer exchange `f64` seconds, so rounding changes which samples belong to a chunk. We need canonical `FrameIndex`/`FrameCount` types bound to a `SampleRate` so slicing, metadata, and downstream logic all reference the exact same integers.

**Actionable Items & Code Locations**
- Add `SampleRate`, `FrameIndex`, and `FrameCount` structs to `src/types.rs`, plus helpers to convert between `(frames, rate)` and `Duration` without loss.
- Update `AudioData`, `AudioChunk`, and `ChunkBoundary` (same file) to store `FrameRange` (start + length) alongside the human-readable seconds.
- Change `audio/slicer.rs` to consume the new frame ranges directly instead of multiplying floats by the rate.
- Adjust `audio/decoder.rs` and capture/resample outputs so they populate the new frame metadata the moment audio is decoded or captured.
- Add regression tests around `slice_audio` proving the concatenation of output chunk spans exactly matches the source frames.

**Deliverable:** All audio buffers and chunk boundaries use the new integer frame types, and slicing operates solely on those integers (seconds exist only for UI/logging).

## Phase 2 – Integer Chunk Config & Greedy Logic
**Problem to Solve:** `ChunkConfig::new` multiplies the target duration by magic constants, and `chunking/accumulator.rs` compares floats with arbitrary EPS. We need integer limits derived from the actual analysis hop so the greedy planner enforces exact counts without heuristics.

**Actionable Items & Code Locations**
- Extend `ChunkConfig` (`src/types.rs`) to accept explicit `target_frames`, `max_frames`, and `overshoot_frames` computed from the audio sample rate and hop size.
- Update `main.rs::plan_chunks` and callers to pass the correct frame counts (using the types from Phase 1) instead of raw seconds.
- Rewrite `chunking/accumulator.rs` to operate on `FrameCount`/`FrameIndex`, making the comparisons integer-only and eliminating `EPS`.
- Adjust `chunking/spans.rs` to emit spans as frame ranges derived from transcript timestamps rounded consistently (e.g., `round_half_up`) once, so downstream code stays integer.
- Add property tests in `src/chunking/tests.rs` that confirm the greedy algorithm partitions the integer timeline exactly and never exceeds the configured limits.

**Deliverable:** Greedy chunk planning still streams spans in-order but now uses integer frame counts with deterministic limits—no floating-point thresholds remain.

## Phase 3 – Pause Detector Parameter Fix & Calibration
**Problem to Solve:** `main.rs:316-325` calls `detect_pauses(audio, min_silence_duration, silence_threshold, window_duration)` even though `pause_detector` expects `(window_ms, min_silence_ms, threshold)`. That bug, plus hard-coded thresholds, makes pause timestamps unreliable guesses.

**Actionable Items & Code Locations**
- Swap the call-site arguments in `main.rs::detect_pauses_for_chunking` so `window_duration` feeds the detector’s `window_ms` parameter, `min_silence_duration` feeds `min_silence_ms`, and the threshold stays threshold.
- In `audio/pause_detector.rs`, convert the API to accept integer frame counts (leveraging Phase 1 types) and compute RMS/mean using those windows.
- Add a small calibration helper in `audio/capture.rs` (or a new module) that measures noise floor and peak levels during the first few batches; feed that into `detect_pauses` so thresholds derive from actual captured audio rather than fixed constants.
- Write unit tests that simulate loud/quiet buffers verifying that pauses are only emitted when the calibrated silence duration (in frames) is truly met.

**Deliverable:** Pause detection operates on calibrated integer windows with correctly ordered parameters, producing deterministic pause timestamps that align with the sample clock.


## Phase 5 – Remove EPS & Floating Comparisons Everywhere Else
**Problem to Solve:** Even after the earlier phases, helpers like `spans.rs` and other modules still rely on `EPS = 1e-9` guards. We need a sweep to replace every floating compare tied to sample math with the integer constructs introduced earlier.

**Actionable Items & Code Locations**
- Audit `chunking/spans.rs`, `audio/resample.rs`, `audio/capture.rs`, and any other modules using `EPS` or `as f64` casts for control flow; replace with integer frame math or rational helpers.
- Where timestamps must be logged, provide conversion helpers that clearly document the rounding mode so no control flow depends on floats.
- Update unit tests to assert that conversions between frames and seconds round-trip to the same frame counts.
- Remove the `EPS` constant entirely, ensuring no code path depends on ad-hoc tolerances.

**Deliverable:** The entire audio + chunk pipeline compares and slices using integer math only; `EPS` and float-based guard clauses are gone.

## Architectural Goal Checklist
- [ ] Streaming order is preserved: spans and chunks are processed strictly in arrival order with no lookahead buffering beyond what already exists.
- [ ] Every duration and boundary is stored as integer frame counts tied to a declared `SampleRate`, with seconds used only for display/logging.
- [ ] Pause detection operates on calibrated measurements from the actual capture stream—no hard-coded thresholds or misordered parameters.
- [ ] Transcript granularity is derived from real Whisper metadata (tokens/timestamps) rather than duration-based guesses.
- [ ] Chunk length limits (target/max/overshoot) are computed directly from the analysis hop/sample rate, not from arbitrary multipliers.
- [ ] No new user-facing features are introduced; the existing workflow is simply rebuilt so each step is correct by construction.
- [ ] The design remains simple: greedy streaming logic stays intact, but all math is exact and documented.
- [ ] Tests cover every new invariant (integer slicing, chunk partitioning, calibrated pauses) so regressions are caught automatically.
