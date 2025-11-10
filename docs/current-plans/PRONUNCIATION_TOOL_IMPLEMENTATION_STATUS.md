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

## Explicitly Rejected
- (2025-11-10) User: "Do NOT edit the plan file itself."
- (2025-11-10) Python- and WhisperX-based alignment tooling for the pronunciation binary.

## Implementation Details
- Architecture captured in `docs/current-plans/PRONUNCIATION_TOOL_ARCHITECTURE.md` (capture -> features -> alignment -> visualization) with data contracts for each stage.
- Module layout for Phase 1 scaffolding: `src/bin/pronunciation.rs`, `src/pronunciation/mod.rs`, `src/pronunciation/features/mod.rs`, `src/pronunciation/alignment/mod.rs`, `src/pronunciation/metrics/mod.rs`, `src/ui/mod.rs`.
- Audio stack: `cpal` (capture), `rodio` (playback), `aus` (spectral analysis), `egui`/`eframe` (interactive UI). Alignment implemented in Rust via dynamic time warping over CMU-derived phoneme templates.
- CMU-style lexicon bundled under `assets/phonemes/lexicon.txt`, embedded through `build.rs`.
- Default capture buffer configured for 100–200 ms latency and exposed via `SessionConfig` for runtime adjustment.
- UI scaffold will ensure baseline accessibility (focus navigation, contrast, labels); extended accessibility left for future phases.
- Status doc updates must accompany each completed phase with dated notes and summary of executed tests.

## Issues Encountered
- None yet.

## Phase Progress
- (2025-11-10) Phase 1 scaffolding established: architecture doc captured module contracts, pronunciation binary and modules compile cleanly, and `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test` all succeeded.

