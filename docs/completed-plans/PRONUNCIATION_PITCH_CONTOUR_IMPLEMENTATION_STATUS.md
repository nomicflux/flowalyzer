## Agreements Made
- (2025-11-11) User: "Pitch contours must be matched; direct pitch not."
- (2025-11-11) User: "if a woman speaks with a rising intonation, a man must also speak with a rising intonation."

## Explicitly Rejected
- (2025-11-11) Treating pitch as fully ignorable (no contour enforcement) is rejected; prior scaffolding that passed flat intonation regardless of contour must be replaced.

## Implementation Details
- Pitch contour extraction will rely on `aus::analysis::pyin_pitch_estimator` / `pyin_pitch_estimator_single` (confirmed on docs.rs) to produce frame-level F0 estimates aligned with existing STFT settings (25 ms window, 10 ms hop at 16 kHz).
- Contour normalisation strategy: convert F0 values to semitone offsets (`12 * log2(f0 / f0_ref)`) after smoothing; choose `f0_ref` as the clip’s median voiced F0 so absolute pitch shifts cancel while contour shape remains.
- Voiced/unvoiced handling: treat `pyin` unvoiced outputs as gaps; fill via last-observation-carried-forward followed by cubic smoothing to avoid false contour spikes.
- Contour comparison inputs will be stored alongside existing feature tensors in `PronunciationFeatures`, guaranteeing shared frame indices for alignment and scoring.
- Outstanding gap: `aus` does not provide derivative helpers for contour slope, so we will implement lightweight finite-difference gradients in Phase 2 to expose contour direction/velocity metrics.
- Implemented `src/pronunciation/features/contour.rs` to wrap pYIN outputs, normalise to semitone offsets, interpolate to mel frame count, and expose the series as `pitch_contour` within `PronunciationFeatures`.
- Added deterministic smoothing (5-frame moving average) and forward/backward filling to stabilise contour gaps while preserving rising/falling patterns; unit tests validate octave invariance and shape sensitivity.
- Fixtures (`tests/fixtures/alignment/*.json`, `tests/fixtures/features/reference_expected.json`) now explicitly carry `pitch_contour` vectors so downstream alignment/metric tests operate on the new contract.
- `AudioAligner` now weights contour distance (`AlignmentWeights.pitch`) during DTW cost construction, propagates per-segment contour similarity into `AlignmentReport`, and surfaces reference/learner contour series for UI visualisation.
- `MetricCalculator` derives the intonation score from contour similarity (energy now a secondary blend), while UI renders contour overlays, timeline badges, and per-segment contour diagnostics.

## Issues Encountered
- None yet; awaiting implementation phases.

## Phase Progress
- (2025-11-11) Phase 1 – Specification alignment complete: updated redesign plan, implementation status, tutorial, and new contour status doc with user quotes; confirmed `aus` pYIN availability and noted derivative gap. `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test` all succeeded.
- (2025-11-11) Phase 2 – Pitch feature extraction & normalisation complete: contour module added, `PronunciationFeatures` extended with `pitch_contour`, fixtures updated, and new tests (`tests/features_pitch.rs`) cover octave invariance plus contour mismatch detection. Commands run: `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` (all passed).
- (2025-11-11) Phase 3 – Alignment, scoring, and UI updated: DTW cost integrates contour deltas, reports expose contour bands, metrics rebase intonation on contour similarity, and the session UI now plots reference vs learner contours with per-segment contour scores. Commands run: `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` (all passed).

