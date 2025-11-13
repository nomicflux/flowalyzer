# Pronunciation Tool Library Research

## Overview
- Target stack focuses on cross-platform capture/playback, spectral analysis, phoneme alignment, and an interactive UI tailored to macOS 15 requirements.
- Recommendations prioritise pure-Rust components where possible, while documenting Python/Kaldi tooling as fallbacks.

### Summary Matrix
| Capability      | Primary Choice          | Alternatives        | Key Dependencies |
|-----------------|-------------------------|---------------------|------------------|
| Recording       | `cpal`                  | `wavy`              | CoreAudio (macOS), ALSA/JACK (Linux), WASAPI/ASIO (Windows) |
| Playback        | `rodio`                 | —                   | `cpal`, Symphonia codecs |
| Spectral        | `aus`                   | Custom FFT pipeline | `rustfft` (via `aus`) |
| Alignment       | WhisperX                | MFA, Gentle         | Python 3.10+, CUDA *optional*, Kaldi/Conda for MFA |
| Visualization   | `egui` (`eframe`)       | `tauri`             | `winit`, `wgpu`/`glow`, OS webviews for `tauri` |

## Audio Capture and Playback

- `cpal`
  - **Capabilities**: Cross-platform audio host enumeration, device selection, and PCM input/output streams in pure Rust; supports macOS via CoreAudio along with Windows WASAPI, Linux ALSA/JACK, iOS, Android, and WebAssembly targets.[^cpal]
  - **Pros**: Fine-grained control over buffer formats and latency; integrates cleanly with existing Flowalyzer sample pipelines; no required runtime beyond Rust.
  - **Cons**: Low-level API requires manual stream management and resampling; platform quirks (e.g., exclusive mode, device format negotiation) must be handled manually.
  - **Integration Notes**: Will serve as the microphone capture backend; reuse for playback via higher-level helpers to share device selection logic.

- `rodio`
  - **Capabilities**: High-level playback API layered on `cpal`, with Symphonia-backed decoding for common codecs (FLAC, MP3, Vorbis, WAV) and optional codec-specific backends.[^rodio]
  - **Pros**: Simplifies queuing and mixing of audio sources; minimal boilerplate for sample conversion.
  - **Cons**: Playback-only; still requires `cpal` backend availability; async mixing not configurable without extending internals.
  - **Integration Notes**: Use for reference audio playback and recording review; convert Flowalyzer `AudioData` into `rodio::Source` wrappers.

- `wavy`
  - **Capabilities**: Asynchronous recording and playback abstractions with built-in buffering and simple event loop integration via the `pasts` executor; ships with `fon` audio buffer utilities.[^wavy]
  - **Pros**: Rapid prototypes with less boilerplate; built-in stereo buffer helpers.
  - **Cons**: Imposes additional dependencies (`pasts`, `fon`); diverges from Flowalyzer’s synchronous pipeline; less flexibility around device configuration.
  - **Integration Notes**: Documented as an alternative but not selected—would complicate alignment with existing code conventions.

## Spectral Analysis

- `aus`
  - **Capabilities**: STFT/ISTFT, window generation, spectral statistics (centroid, entropy, flatness, slopes), harmonicity, log spectrogram transforms, and pYIN pitch estimation utilities.[^aus]
  - **Pros**: Pure Rust toolkit with feature breadth matching pronunciation scoring needs; integrates with `rustfft` internally; no external runtime.
  - **Cons**: Requires authoring custom glue for MFCC/DWT or DTW comparisons; documentation assumes familiarity with DSP terminology.
  - **Integration Notes**: Primary engine for mel-spectrograms, spectral flux, and feature extraction prior to alignment comparisons.

## Phoneme Alignment and Pronunciation Scoring

- `WhisperX`
  - **Capabilities**: Batched Whisper inference (~70× real-time) plus wav2vec2-based forced phoneme alignment; outputs word/phoneme timestamps and optional diarization.[^whisperx]
  - **Pros**: Reuses Whisper transcripts already central to Flowalyzer; produces high-resolution alignment without manual modeling.
  - **Cons**: Python package with GPU-focused acceleration; requires CUDA for optimal performance and HuggingFace access tokens for diarization models.
  - **Integration Notes**: Run as an external process with managed venv; fallback path required when CUDA unavailable (CPU int8 inference supported).

- Montreal Forced Aligner (MFA)
  - **Capabilities**: Kaldi-powered forced alignment with multilingual acoustic and pronunciation models; command-line workflows for training and adaptation.[^mfa]
  - **Pros**: Supports language adaptation and offline alignment; robust to long-form audio.
  - **Cons**: Heavy dependency stack (conda, Kaldi, OpenFST); slower than WhisperX; separate pipeline from Whisper transcripts.
  - **Integration Notes**: Document as fallback when WhisperX alignment unavailable or when custom language models needed.

- Gentle
  - **Capabilities**: Kaldi-based forced alignment service with REST API and CLI; targeted at English datasets.[^gentle]
  - **Pros**: Lightweight deployment, browser UI, Docker image available.
  - **Cons**: English-only; project status largely archival; limited phoneme detail compared to WhisperX.
  - **Integration Notes**: Use as an English fallback for offline installations where Python dependencies are undesirable.

## Interactive Visualization

- `egui` / `eframe`
  - **Capabilities**: Immediate-mode GUI for native and web targets; supports plots, custom widgets, and AccessKit accessibility; official `eframe` harness covers macOS, Windows, Linux, Android, and WASM.[^egui]
  - **Pros**: Pure Rust stack; rapid prototyping; integrates with Flowalyzer binaries without web tooling.
  - **Cons**: Immediate-mode layout requires careful state management; styling less “native” than platform toolkits.
  - **Integration Notes**: Primary UI framework for real-time feedback, spectrogram plots, and control panels.

- `tauri`
  - **Capabilities**: Desktop framework coupling Rust backend with system webviews (WKWebView, WebView2, WebKitGTK); bundles installers and system tray support.[^tauri]
  - **Pros**: Allows reuse of web visualization stacks; produces small binaries with native installers.
  - **Cons**: Requires Node/web tooling; heavier build pipeline; runtime depends on system webview availability.
  - **Integration Notes**: Documented as an alternative path for browser-based UX, but not the primary target for this project.

## Summary Recommendations

| Category | Primary Choice | Rationale |
| --- | --- | --- |
| Capture | `cpal` | Low-level control, pure Rust, aligns with existing audio pipeline |
| Playback | `rodio` | Simplifies playback while reusing `cpal` backend |
| Spectral Analysis | `aus` | Comprehensive DSP utilities without external runtimes |
| Alignment | `WhisperX` | Leverages Whisper outputs; precise phoneme timings |
| Fallback Alignment | MFA → Gentle | MFA for multilingual, Gentle for lightweight English-only |
| UI | `egui` (`eframe`) | Native Rust GUI for real-time visualization |

[^cpal]: https://raw.githubusercontent.com/RustAudio/cpal/master/README.md
[^rodio]: https://raw.githubusercontent.com/RustAudio/rodio/master/README.md
[^wavy]: https://docs.rs/wavy/latest/wavy/
[^aus]: https://docs.rs/aus/latest/aus/analysis/index.html
[^whisperx]: https://raw.githubusercontent.com/m-bain/whisperX/main/README.md
[^mfa]: https://montreal-forced-aligner.readthedocs.io/en/latest/user_guide/index.html
[^gentle]: https://raw.githubusercontent.com/lowerquality/gentle/master/README.md
[^egui]: https://raw.githubusercontent.com/emilk/egui/master/README.md
[^tauri]: https://raw.githubusercontent.com/tauri-apps/tauri/dev/README.md

