# Pronunciation Tool Architecture (Phase 1)

## Context
- (2025-11-10) User: "I am looking for a secondary binary in the current project. It:
  1. Can record the user's voice
  2. Can playback the voice
  3. Can compare overall pronunciation to a WAV audio file (needs to ignore pitch, but I want to know that the intonation, pronuncation of sounds, etc. all match, and if not where it does not in a visualizable way)."
- (2025-11-10) User: "1. Interactive tool. (Can be desktop, can be browser-based. But must be real-time and interactive.)"
- (2025-11-10) User: "2. Requires spectral & phoneme analysis (don't know whether this is beyond whisper or not)"
- (2025-11-10) Platform scope: macOS 15 only.
- Phase 1 forbids Python tooling and WhisperX; the alignment pipeline must be pure Rust and operate entirely on audio (no transcripts).

## Data Flow Summary
1. **Capture** (`src/audio/`, invoked by secondary binary): record mono 16 kHz PCM frames from the default (or selected) microphone using `cpal`. Store raw samples as `RecordedClip` (Vec<f32>, 16 kHz, mono) plus metadata (duration, RMS, capture timestamp).
   - Default stream latency: 100–200 ms buffer duration, surfaced as a configurable parameter to balance responsiveness and glitch resistance.
2. **Feature Extraction** (`src/pronunciation/features/`): transform `RecordedClip` and reference WAV audio into `FeatureBatch` objects. Core features (via `aus`):
   - 25 ms Hann-window STFT with 10 ms hop.
   - 80-band mel spectrogram (log-scaled).
   - Spectral flux and frame energy.
   - 13-coefficient MFCCs with delta and delta-delta.
   All tensors normalized frame-wise and clipped to match reference length.
3. **Audio Alignment** (`src/pronunciation/alignment/`): compare reference/learner feature trajectories (MFCC similarity, spectral flux variance, energy timing offsets) to produce `AlignmentReport` segments summarising coarse timing and articulation differences.
4. **Metrics** (`src/pronunciation/metrics/`): aggregate `AlignmentReport` into `PronunciationScores` (overall score, timing deviation score, articulation score, intonation variance). Provide segment-level diagnostics for visualization.
5. **Visualization / UI** (`src/ui/`): immediate-mode `egui`/`eframe` surface showing waveform, mel spectrogram overlays, and session feedback badges. Consumes `RecordedClip`, `AlignmentReport`, `PronunciationScores`.

## Module Responsibilities
- `src/bin/pronunciation.rs`: entrypoint driving the session-only CLI flow (`pronunciation session`). Responsible for dispatching to library modules and handling configuration.
- `src/pronunciation/mod.rs`: central module exporting submodules, shared types (`PronunciationError`, `RecordedClip`, `FeatureBatch`, `AlignmentReport`, `PronunciationScores`, `SessionConfig`, `CaptureSettings`), and the top-level orchestration helper (`run_session`).
- `src/pronunciation/features/mod.rs`: defines `FeatureExtractor` and supporting helpers (window generation, mel filter bank caching, normalization).
- `src/pronunciation/alignment/mod.rs`: hosts the audio-only alignment placeholder that derives `AlignmentReport` from feature statistics.
- `src/pronunciation/metrics/mod.rs`: provides scoring combinators, thresholds, and serialization of results.
- `src/ui/mod.rs`: wraps `eframe` application bootstrap and view composition (`launch_ui`, `SessionApp`, reusable widgets for waveform/spectrogram/timeline).
## Data Contracts
- `RecordedClip`
  - `samples: Arc<[f32]>` (normalized to [-1.0, 1.0])
  - `sample_rate: u32` (fixed at 16_000)
  - `channels: NonZeroU8` (fixed at 1)
  - `duration: Duration`
  - `captured_at: DateTime<Utc>`
- `FeatureBatch`
  - `mel_spectrogram: Array2<f32>` (frames x 80 bands)
  - `spectral_flux: Array1<f32>` (frames)
  - `energy: Array1<f32>` (frames)
  - `mfcc: Array2<f32>` (frames x 13)
  - `deltas: Array2<f32>` (frames x 13)
  - `delta_deltas: Array2<f32>` (frames x 13)
  - `frame_hop_ms: u32` (10) and `frame_window_ms: u32` (25)
- `AlignmentReport`
  - `phonemes: Vec<AlignedPhoneme>`
  - `total_duration: Duration`
  - `reference_path_cost: f32`
  - `learner_path_cost: f32`
  - `global_time_offset_ms: f32`
  - `confidence: f32`
  - `AlignedPhoneme`
    - `symbol: String`
    - `reference_start_ms: f32`
    - `reference_end_ms: f32`
    - `learner_start_ms: f32`
    - `learner_end_ms: f32`
    - `timing_delta_ms: f32`
    - `similarity: f32`
    - `articulation_variance: f32`
- `PronunciationScores`
  - `overall: f32`
  - `timing: f32`
  - `articulation: f32`
  - `intonation: f32`
  - `per_phoneme: Vec<PhonemeScore>`
- `SessionConfig`
  - `reference_wav: PathBuf`
  - `learner_wav: PathBuf`
  - `assets_root: PathBuf`
  - `capture: CaptureSettings`
  - `ui_enabled: bool`

## External Dependencies
- `cpal`: low-level capture and playback backend (CoreAudio on macOS). Required for microphone input and audio output streams.
- `rodio`: queued playback of reference and recorded audio; shares `cpal` backend.
- `aus`: spectral feature extraction (STFT, mel, MFCC, spectral statistics).
- `rustfft`: transitively pulled by `aus`; ensure SIMD feature flags align with target CPU.
- `eframe` / `egui`: immediate-mode GUI, spectrogram plotting, interaction.
- `hound` or `symphonia`: parse reference WAV files; convert to 16 kHz mono `RecordedClip`.
- `chrono`, `serde`, `serde_json`: timestamping and serialization for session persistence (Phase 1 scaffolds types; serialization fully implemented later).

## Assets & Build Integration
- `assets/phonemes/lexicon.txt` bundled via `build.rs` (e.g., `include_bytes!` or packing into `OUT_DIR`).
- Additional fixtures (sample recordings, reference WAVs) stored under `assets/samples/` for development and integration tests (future phases).
- Configuration defaults baked into `SessionConfig`; CLI arguments (Phase 2) override values. Phase 1 scaffolding only establishes structure.
- Capture latency configuration exposed via `SessionConfig` (default 100–200 ms) to maintain near-real-time feedback without underruns.
- UI accessibility baseline: keyboard focus, readable contrast palette, and descriptive labels; revisit extended accessibility needs in future phases if requested.

## Execution Flow
1. `src/bin/pronunciation.rs` parses arguments (Phase 1 stub), initializes logging, and constructs `SessionConfig`.
2. Capture pipeline records `RecordedClip`.
3. Reference WAV decoded to `RecordedClip`.
4. `FeatureExtractor` produces `FeatureBatch` for learner and reference audio; caches filter banks for reuse.
5. `alignment::AudioAligner` compares reference and learner features to produce `AlignmentReport`.
6. `MetricCalculator` generates `PronunciationScores`.
7. `ui::launch_ui` displays results inside the session window.

## Testing Outlook
- Unit tests: Feature extraction determinism (mel/MFCC fixtures), placeholder alignment behaviour using synthetic audio pairs, scoring aggregation sanity checks.
- Integration tests (later phases): end-to-end record -> session -> visualize workflow using short fixtures.
- Static analysis: `cargo fmt`, `cargo clippy --all-targets --all-features` required before Phase handoff.

## Open Questions
- None; outstanding items addressed on 2025-11-10 (macOS 15 only, capture latency 100–200 ms configurable, baseline accessibility).


