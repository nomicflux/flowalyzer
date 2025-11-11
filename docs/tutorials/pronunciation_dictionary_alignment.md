# Audio-Only Alignment Walkthrough

This tutorial describes the intermediate, transcript-free alignment approach used after the Phase 1 audio-only reset. It explains how the placeholder implementation compares learner audio to a reference clip and surfaces timing/similarity metrics inside the session UI.

## Goals

- Operate entirely on audio features (mel, MFCC, spectral flux, energy).
- Provide coarse but actionable feedback while the fully fledged audio alignment arrives in later phases.
- Preserve the near real-time requirement (≤200 ms pipeline latency).

## Feature Inputs

- `PronunciationFeatures` is produced by `src/pronunciation/features/FeatureExtractor` for both reference and learner clips.
- The extractor uses `aus` to compute:
  - Log-magnitude mel spectrogram (80 bands)
  - Spectral flux
  - Frame energy
  - MFCC vectors plus deltas and delta-deltas
- Everything is normalized frame-wise to keep statistics comparable across clips.

## Placeholder Alignment (`AudioAligner`)

- **Module**: `src/pronunciation/alignment/mod.rs`
- Key steps per segment:
  1. Slice the feature tensors into fixed-size windows (~120 ms, 12 frames).
  2. Compute MFCC L¹ distance averaged across coefficients → similarity score (`1 / (1 + distance)`).
  3. Compare spectral flux between clips → articulation variance proxy.
  4. Locate energy peaks → timing delta (converted to milliseconds).
- Segments are labeled sequentially (`#1`, `#2`, …) until the end of the shorter clip is reached.
- Aggregated statistics (mean similarity, articulation, timing offset) become the `AlignmentReport` used by metrics and the UI.

## Metrics

- **Module**: `src/pronunciation/metrics/mod.rs`
- Consumes the placeholder `AlignmentReport` and computes:
  - Overall confidence (weighted combination of segment similarity and cost)
  - Timing score (penalises mean timing deltas)
  - Articulation score (penalises spectral flux variance)
  - Intonation score (re-uses similarity as a stand-in until pitch-insensitive scoring lands)
- Per-segment values populate the timeline and badges in the session UI.

## UI Integration

- `src/ui/screens/session.rs` renders waveform, spectrogram, and segment summaries.
- Each synthetic “phoneme” slot corresponds to an `AlignmentReport` segment with timing, similarity, and articulation values.
- When the audio-only DTW alignment is implemented in Phase 2, the UI wiring remains the same; only the upstream data source changes.

## Testing

- `tests/session_smoke.rs` synthesises short sine waves, runs `run_session` headlessly, and asserts that the placeholder alignment returns at least one segment with finite scores.
- Additional unit tests can target `AudioAligner` helpers (e.g., MFCC similarity, energy peak detection) if finer-grained coverage is required.

## Roadmap

1. Replace the placeholder metrics with a mel/MFCC DTW implementation that enforces monotonic alignment but remains transcript-free.
2. Expand the UI to visualise continuous timing deviations rather than discrete segment markers.
3. Fold pitch-deemphasised scoring into the metrics module (e.g., explicitly filter pitch bands before similarity computations).

For the latest contract and planning details, see `docs/current-plans/PRONUNCIATION_TOOL_AUDIO_ONLY_REDESIGN.md`.