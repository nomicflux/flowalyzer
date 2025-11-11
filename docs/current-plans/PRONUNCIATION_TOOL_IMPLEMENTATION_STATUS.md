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

## Issues Encountered
- (2025-11-11) Building the vendored `whisper-rs-sys` crate during `cargo test` required elevated file access to macOS SDK headers; re-ran the suite with `required_permissions: ['all']` to satisfy the sandbox guidance.

## Phase Progress
- (2025-11-10) Phase 1 scaffolding established: architecture doc captured module contracts, pronunciation binary and modules compile cleanly, and `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` all succeeded.
- (2025-11-10) Phase 2 capture/playback complete: new audio capture/playback modules integrated, CLI now exposes record/play/record-and-play flows, regression tests added (`tests/audio_capture_playback.rs`), and `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` all passed.
- (2025-11-11) Phase 3 spectral feature pipeline complete: `FeatureExtractor` produces normalized mel/flux/energy/MFCC(+Δ/+ΔΔ) tensors backed by `aus`, fixtures in `tests/features.rs` validate deterministic outputs, and `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` all executed successfully (with sandbox permission escalation noted above).
- (2025-11-11) Phase 4 phoneme alignment complete: bundled CMU lexicon embedded, `PhonemeAligner` now maps transcripts → phoneme templates → DTW segments, metrics consume populated `AlignmentReport`, and regression coverage added (`tests/alignment.rs`, `tests/alignment/dtw.rs`, `tests/alignment/dictionary.rs`). `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` all passed post-integration.
- (2025-11-11) Phase 5 metrics & UI: scoring helpers finalized with deterministic tests (`tests/metrics.rs`), `run_session` now drives `ui::launch_ui` through `VisualizationState`, eframe components render waveform/spectrogram/timeline views, and `examples/session_ui.rs` provides the runnable manual-QA entry point (`cargo run --example session_ui`). `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` (requiring `required_permissions: ['all']` for whisper build) all succeeded.
- (2025-11-11) Phase 6 integration & release prep: CLI gains `analyze`/`session` commands, asset discovery centralized in `AppConfig`, deterministic integration test (`tests/integration/full_session.rs`) validates the record→analyze→visualize pipeline, and documentation now includes installation, troubleshooting, and release checklist. `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` (with `required_permissions: ['all']`) all executed post-changes.
- (2025-11-11) Phase 1 audio-only reset (in progress): transcript assets removed, CLI reduced to session-first flow, placeholder audio alignment added, and smoke test (`tests/session_smoke.rs`) validates the transcript-free pipeline.
- (2025-11-11) Note: Earlier phase progress entries describing transcript/lexicon workflows are superseded by the audio-only reset.

