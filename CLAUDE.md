# CLAUDE.md – Flowalyzer Overview (2025-11-08)

Read `IMPLEMENTATION_PLAN.md` (same directory) before making changes; it tracks agreements, rejected ideas, and per-phase status.

## Current Snapshot
- CLI: `cargo run -- <INPUT> <OUTPUT_DIR> --recipe-json '{...}' [--target-duration <seconds>] [--start <time>] [--end <time>]`
- Output: each chunk is rendered to `<OUTPUT_DIR>/chunk_{NNNN}/processed.wav`
- Recipes: JSON array of steps (`repeat_count`, `speed_factor`, `silent` flag for silence steps); either inline JSON or `--recipe-file`
- Tests: `cargo test` → 34 passed, 1 ignored; `cargo clippy --all-targets --all-features` → clean
- Prerequisites: C++14 toolchain, `cmake`, Whisper GGML model (default `./models/ggml-base.en.bin`)

## Quick Start
```bash
cargo build
cargo run -- input.mp3 out_dir \
  --recipe-file recipes/language_learning.json \
  --target-duration 1.8 \
  --start 00:01:00 \
  --end 00:03:30
```

`language_learning.json` example:
```json
{
  "name": "language-learning",
  "steps": [
    {"repeat_count": 3, "speed_factor": 0.5, "silent": false},
    {"repeat_count": 1, "speed_factor": 0.5, "silent": true},
    {"repeat_count": 3, "speed_factor": 1.0, "silent": false},
    {"repeat_count": 1, "speed_factor": 1.0, "silent": true},
    {"repeat_count": 3, "speed_factor": 1.5, "silent": false},
    {"repeat_count": 1, "speed_factor": 1.5, "silent": true}
  ]
}
```

### Runtime Logging
- Decode stage prints sample count and sample rate.
- Transcription logs sentence/word segment counts plus a preview of the first few transcript lines.
- Chunk planning logs average transcript segments per chunk.
- Per-chunk processing reports progress every 10 chunks and final write location.

## Key Modules
| File | Responsibility |
| --- | --- |
| `src/main.rs` | CLI parsing, orchestration, per-chunk output writing |
| `src/types.rs` | Core data types (no unused scaffolding) |
| `src/transcription/mod.rs` | Whisper transcription with sentence/word tagging |
| `src/chunking/mod.rs` | Greedy chunk boundary calculation respecting min/max duration |
| `src/audio/{decoder,encoder,slicer,assembler}.rs` | Audio I/O, slicing, crossfade assembly |
| `src/operations/{repeat,silence,speed}.rs` | Pure operations; `speed` uses Signalsmith stretch with latency compensation |
| `src/operations/recipe.rs` | Applies recipe steps (speed → repeat → optional silence) |

## Build & Test Commands
```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

## Requirements
- Install Whisper model: `wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin -P ./models/`
- Ensure Xcode Command Line Tools (macOS) or `clang` (Linux) for ssstretch.
- Set `WHISPER_MODEL_PATH` if the model lives elsewhere.

## Architectural Notes
- All processing modules are pure; only CLI orchestrates I/O.
- Recipes control operations; legacy `--operations` flag has been removed.
- Fast playback truncation was fixed by padding/flush/removing latency before trimming.
- Transcript metadata (segment text, granularity, boundary source IDs) is actively logged to keep types in use.

## Troubleshooting
- **Missing model**: download GGML file or point `WHISPER_MODEL_PATH`.
- **Decode failure**: verify input format supported by Symphonia.
- **Empty chunk outputs**: check recipe JSON; `repeat_count` must be >0 and `speed_factor` >0.
- **Performance**: use `cargo build --release` for large files.

## Reference
- Planning/Status: `docs/current-plans/JSON_RECIPE_ENHANCEMENTS.md`
- State snapshot: `ACTUAL_STATE.md`
- Recipes examples: `recipes/` (create as needed)
- Whisper model default path: `./models/ggml-base.en.bin`
