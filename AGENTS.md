# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the pronunciation engine, with `src/bin/pronunciation` wiring the egui real-time UI. Shared DSP utilities live under `src/audio`, while adapters that talk to Whisper and flow recipes sit in `src/pipeline`. Integration harnesses reside in `tests/`, sample clips and UI assets under `assets/`, model weights in `models/`, and longer diagnostic transcripts in `docs/` and `examples/`. Keep transient experiment output inside `out_chunks/` or `target/` so the git tree stays clean.

## Build, Test, and Development Commands
- `cargo build --release` compiles the real-time engine with the optimized profiles declared in `Cargo.toml`.
- `cargo run --bin pronunciation session --reference assets/demo.wav` launches the UI against the bundled clip for manual checks.
- `cargo test` runs the mock-capture suites that lock in `SessionEngine` timing guarantees.
- `cargo fmt && cargo clippy -- -D warnings` enforce formatting and refuse lints that would otherwise slip into CI.

## Coding Style & Naming Conventions
Rust edition 2021 defaults apply: four-space indentation, trailing commas in multi-line literals, and module names in `snake_case`. Exported types and structs should use `UpperCamelCase`, internal helpers stay `snake_case`, and async tasks end with `_task` to flag their lifecycle. Always run `cargo fmt` before opening a PR, and fix clippy diagnostics locally so CI stays green.

## Testing Guidelines
Add integration cases in `tests/session_*` to cover regressions around latency windows, and unit tests alongside the modules they exercise. Name tests after the behavior, e.g., `enforces_latency_floor`, and prefer table-driven inputs when comparing pitch contours. Aim for full coverage of new branches; if a real-time scenario cannot be automated, document the manual checklist in the PR and link to any captured logs.

## Commit & Pull Request Guidelines
Commits follow short, imperative statements (`clip playback simplified`). Keep related code and asset updates together, and reference issues with `Refs #NN` when applicable. Pull requests should include: 1) a concise summary, 2) reproduction steps or `cargo run` flags used to verify the UI, 3) screenshots or gifs when UI panels change, and 4) notes on performance or audio latency impacts. Tag reviewers by subsystem (`audio`, `ui`, `inference`) so work can be triaged quickly.

## Agent Workflow Tips
When multiple agents collaborate, announce ownership of files in the PR thread, push small increments, and leave TODO comments prefixed with `AGENT:` plus your initials to avoid collisions. Reset model caches (`assets/tmp`) between runs so results stay deterministic for the next contributor.
