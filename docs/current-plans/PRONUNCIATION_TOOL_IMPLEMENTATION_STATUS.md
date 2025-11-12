## Agreements Made
- (2025-11-10) User: "I am looking for a secondary binary in the current project. It:

1. Can record the user's voice

2. Can playback the voice

3. Can compare overall pronunciation to a WAV audio file (needs to ignore pitch, but I want to know that the intonation, pronuncation of sounds, etc. all match, and if not where it does not in a visualizable way)."
- (2025-11-10) User: "1. Interactive tool. (Can be desktop, can be browser-based. But must be real-time and interactive.)"
- (2025-11-10) User: "2. Requires spectral & phoneme analysis (don't know whether this is beyond whisper or not)"
- (2025-11-10) Phase 1 scope: Rust-only pronunciation pipeline with bundled CMU-style lexicon and a secondary binary entrypoint (`src/bin/pronunciation.rs`).
- (2025-11-10) Platform scope: macOS 15 only.
- (2025-11-10) Capture stream latency: default 100–200 ms buffer, configurable.
- (2025-11-10) Accessibility baseline: keyboard focus and sensible contrast; further accommodations pending future guidance.
- (2025-11-11) User: "This is an application to practice PRONUNCIATION in NON-ENGLISH LANGUAGES ... The application should record and show how User audio differs significantly from provided audio ... This should be a near real-time analysis. 100-200ms lag is fine, but doing the analysis and then requiring a separate tool is not."
- (2025-11-11) User: "Keep in mind, the key goal was not STRICTLY UNDER 200ms ... If heavier computation is needed to give a still interactive result that is significantly higher quality, that should be implemented."

## Explicitly Rejected
- (2025-11-10) User: "Do NOT edit the plan file itself."
- (2025-11-10) Python- and WhisperX-based alignment tooling for the pronunciation binary.
- (2025-11-11) User: "Pitch contours must be matched; direct pitch not." The prior interpretation that pitch should be ignored entirely (no contour enforcement) is invalid.

## Implementation Details
- Architecture captured in `docs/current-plans/PRONUNCIATION_TOOL_AUDIO_ONLY_REDESIGN.md` (capture -> features -> audio alignment -> visualization) with updated data contracts for the transcript-free workflow.
- Module layout: `src/bin/pronunciation.rs`, `src/pronunciation/mod.rs`, `src/pronunciation/features/mod.rs`, `src/pronunciation/alignment/mod.rs`, `src/pronunciation/metrics/mod.rs`, `src/ui/mod.rs`.
- Audio stack: `cpal` (capture), `rodio` (playback), `aus` (spectral analysis), `egui`/`eframe` (interactive UI). Alignment is currently an audio-only placeholder comparing MFCC similarity, spectral flux variance, and energy timing offsets; transcripts and CMU lexicon assets were removed.
- Default capture buffer remains 100–200 ms, surfaced through `CaptureSettings` inside `SessionConfig`.
- CLI is session-only: `pronunciation session` is the sole entrypoint, and the former `record`, `play`, `record-and-play`, and `analyze` helpers were removed in favour of in-session feedback.
- UI retains baseline accessibility (focus navigation, contrast, labels) with future enhancements pending guidance.
- Alignment weights (`mfcc`, `delta`, `delta_delta`, `mel`, `energy`, `flux`) are loaded from `assets/config/alignment_weights.json` and injected through `SessionConfig` so tuning never requires recompilation.
- Feature extraction (`src/pronunciation/features/`) still produces normalized mel/flux/energy/MFCC (+Δ/+ΔΔ) tensors via `aus` (25 ms Hann window, 10 ms hop, 80 mel bands, 13 coefficients) used by the placeholder alignment.
- Metrics (`src/pronunciation/metrics/mod.rs`) aggregate placeholder alignment data into timing, articulation, and intonation scores; per-segment diagnostics feed the visualisation.
- UI architecture (`src/ui/`) renders waveform, spectrogram, and timeline views driven by `SessionApp`, now populated by the audio-only alignment report.
- Integration smoke coverage lives in `tests/session_smoke.rs`, synthesising WAV fixtures and ensuring the session pipeline runs without UI enabled.
- Status doc updates must accompany each completed phase with dated notes and executed test summaries.
- Session orchestration uses `SessionRuntime` (Phase 3) to stream live capture chunks through incremental alignment, exposing `SessionHandle` to the UI for continuous feedback and latency telemetry. CLI now launches the runtime directly via `pronunciation session`.

## Issues Encountered
- (2025-11-11) Building the vendored `whisper-rs-sys` crate during `cargo test` required elevated file access to macOS SDK headers; re-ran the suite with `required_permissions: ['all']` to satisfy the sandbox guidance.

## Phase Progress
- (2025-11-11) Phase 1 – Session-only audio baseline reset: Removed transcript assets, collapsed the CLI to the `session` flow, introduced placeholder audio-vs-audio comparison, and updated smoke coverage (`tests/session_smoke.rs`). Commands executed: `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` (all passed).
- (2025-11-11) Phase 2 – Real-time audio alignment & scoring: Externalised DTW cost weights to `assets/config/alignment_weights.json`, injected alignment configuration through `SessionConfig`, strengthened alignment/metrics tests (`tests/alignment.rs`, `tests/metrics.rs`), and confirmed the pipeline with `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` (all passed).
- (2025-11-12) Phase 3 – Session runtime capture: Removed `SessionConfig::learner_wav`, added live `SessionRuntime` with `LiveCapture`, streaming alignment updates, reference playback control, and updated CLI/UI integration. Smoke + full suite verified via `cargo fmt`, `cargo test`.

