# Repository Guidelines

## Project Structure & Module Organization
`src/` contains the pronunciation engine, with `src/bin/pronunciation` wiring the egui real-time UI. Shared DSP utilities live under `src/audio`, while adapters that talk to Whisper and flow recipes sit in `src/pipeline`. Integration harnesses reside in `tests/`, sample clips and UI assets under `assets/`, model weights in `models/`, and longer diagnostic transcripts in `docs/` and `examples/`. Keep transient experiment output inside `out_chunks/` or `target/` so the git tree stays clean.

## Build, Test, and Development Commands
- `cargo build --release` compiles the real-time engine with the optimized profiles declared in `Cargo.toml`.
- `cargo run --bin pronunciation session --reference assets/demo.wav` launches the UI against the bundled clip for manual checks.
- `cargo test` runs the mock-capture suites that lock in `SessionEngine` timing guarantees.
- `cargo fmt && cargo clippy -- -D warnings` enforce formatting and refuse lints that would otherwise slip into CI.

## Coding Style & Naming Conventions
- Rust edition 2021 defaults apply: four-space indentation, trailing commas in multi-line literals, and module names in `snake_case`. Exported types and structs should use `UpperCamelCase`, internal helpers stay `snake_case`, and async tasks end with `_task` to flag their lifecycle. Always run `cargo fmt` before opening a PR, and fix clippy diagnostics locally so CI stays green.
- Functions should be <20 lines, and modules <200 lines. Prefer small helper functions and submodules.
- Build code for the current task. Do not future-proof. Do not write dead code for future phases.
- Wheenever possible, use pure functions and stateless architectures. Keep side effects and state to the periphery of
  projects, and use as little as possible to accomplish the goals.

## Testing Guidelines
- Add unit tests for every pure function, and add integrations tests when the tests accurately test the application flow (no complicated test harnesses - prefer manual testing.)
- Run `cargo test --all` until there are no test failures, even if they are in not in code that you touched
- Run `cargo clippy --all` until there are no warnings. Dead code is not acceptable.
- Test & clippy commands must be run after EVERY completed unit of work. Do not say "Tests not requested." If you
  touched code, you run tests and clippy.

## Commit & Pull Request Guidelines
- Commits follow short, imperative statements (`clip playback simplified`). Keep related code and asset updates together, and reference issues with `Refs #NN` when applicable. Pull requests should include: 1) a concise summary, 2) reproduction steps or `cargo run` flags used to verify the UI, 3) screenshots or gifs when UI panels change, and 4) notes on performance or audio latency impacts. Tag reviewers by subsystem (`audio`, `ui`, `inference`) so work can be triaged quickly.

## Git History
- NEVER for ANY reason run `git checkout`, `git restore`, or `git reset`. You do not understand the git worktree for the
  project. You MUST undo changes manually.

## Agent Workflow Tips
- When multiple agents collaborate, announce ownership of files in the PR thread, push small increments, and leave TODO comments prefixed with `AGENT:` plus your initials to avoid collisions. Reset model caches (`assets/tmp`) between runs so results stay deterministic for the next contributor.
- Update status documentation in docs/current-plans/ every time a task is completed.
- When the user gives you clear directions about what to do next, do it; do not present the plan back to the user.
- Conversely, when the user asks you about which approach to take or for more information, DO NOT make any code changes
  until the user approves your plan.
- Refer to `docs/LESSONS_LEARNED.md` for lessons agents have learned in the past in order to do better work. When making
  mistakes that cause the user to intensely reject your work, ask the user about the core principles violated and add a
  new lesson to `docs/LESSONS_LEARNED.md`.
- ALWAYS ask before deleting files, and explain your reasoning.
- You will be given an initial scope. All subsequent requests must fit within the rubric given by the scope. If you are
  unclear on whether an action fits in the prompt's scope, after searching the code and previously given instructions,
  then ask the user a clarifying question.
- If you feel ANY need to go outside the prompt scope for ANY reason, you must ask the user for permission.
- Plans are not suggestions. Plans are hard guidelines. If you think that rules contradict each other, ask for
  clarification, but you CANNOT ignore plan directions.
