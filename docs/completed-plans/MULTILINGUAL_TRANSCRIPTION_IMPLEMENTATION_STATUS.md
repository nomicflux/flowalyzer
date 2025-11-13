## Agreements Made
- 2025-11-10: User said "Set this all up. detect_language will be the default."
- 2025-11-10: User said "The app is specifically chunking audio in different languages, so that is quite the drawback."

## Explicitly Rejected
- _None recorded._

## Implementation Details
- Default transcription must load a multilingual Whisper model instead of the English-only checkpoint.
- Transcription parameters must leave language unset to trigger Whisper's built-in auto detection, while keeping translation disabled unless explicitly requested.
- CLI should expose controls so runs can pick the Whisper model path and optionally override language detection with an explicit language code.
- Created `TranscriptionSettings` with defaults (`./models/ggml-base.bin`, `detect_language = true`) and plumbed it through `transcribe_audio` and `transcribe_with_logging`.
- Added CLI flags `--whisper-model` and `--whisper-language`, plus banner logging and resolver to produce `TranscriptionSettings`.
- Added unit tests asserting detection defaults and language override behavior.
- English-only model paths (e.g., `*.en.bin`) now force `language = "en"` with detection disabled to match model capabilities.
- Transcript preview truncation now respects Unicode code points to avoid UTF-8 boundary panics.

## Issues Encountered
- _None recorded._

