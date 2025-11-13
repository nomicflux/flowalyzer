## Agreements Made
- *2025-11-08*: User specified CLI should accept JSON recipes: "JSON is actually probably more flexible, let's use JSON."
- *2025-11-08*: User requested single inclusive trim range per run: "Single range is fine. We can re-run the app on multiple ranges."
- *2025-11-08*: User outlined recipe semantics and goals (configurable per run, per-chunk outputs, fix fast playback truncation, start/stop trimming).

## Explicitly Rejected
- No alternatives rejected yet.

## Implementation Details
- Phase 1 focuses on parsing JSON recipe descriptions and optional start/end times at the CLI boundary.
- Phase 2 rewires pipeline output to trim input audio to the requested range, apply recipes per chunk, and emit each chunk as an individual WAV under the requested output directory.
- Phase 3 ensures time-stretching preserves tail audio by compensating for stretcher latency, flushing remaining samples, and verifying that recipe steps always reuse the original chunk.
- Recipe format updated to express silence explicitly (`silent: true` steps) instead of implicit `add_silence_after` flag.
- Deprecated scaffolding (`Operation`, `ProcessingPlan`, chunk metadata) removed to keep the codebase lean and clippy-clean; runtime logging now exercises transcript text, granularity, and boundary source segment data.
- JSON structure must capture ordered steps including repeat counts, speed factors, and whether to add silence.
- CLI should accept either inline JSON or a path flag (TBD during implementation) while validating structure.
- Start/end arguments should accept HH:MM:SS.mmm or seconds; validation must ensure 0 ≤ start < end ≤ audio duration (when available) and start < end > start.
- Existing `Recipe` type is static; introduce runtime recipe types without breaking current recipe-based modules.
- Maintain functions under 20 lines where feasible; add helper modules if necessary.
- Tests required for new parsing helpers/functions.
- Implementation adds `--recipe-json` / `--recipe-file` flags (mutually exclusive, one required) plus `--start` / `--end` trimming options.
- `RuntimeRecipe` / `RuntimeRecipeStep` structs reside in `types.rs`, supporting serde parsing, validation, and conversion to existing `Recipe`.
- Helper functions in `main.rs` parse recipe sources, validate time strings, and feed runtime recipes into the existing pipeline (still single-output for now).
- Whisper dependency robustness addressed by vendoring `whisper-rs-sys` and patching its build script to purge incomplete copies before rebuilding, ensuring `cargo test` self-heals after interrupted builds.
- `cargo test` (2025-11-08) executed after Phase 1 changes: 35 passed, 1 ignored, 0 failed; warnings unchanged (unused exports, metadata fields).

## Issues Encountered
- None encountered during Phase 1 implementation to date.

## Phase 1 – CLI & Recipe Input Status
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/JSON_RECIPE_ENHANCEMENTS.md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [x] **Code Purpose**: Do your changes accomplish the plan purpose and not just mechanical checklists?
- [x] **Required Tests**: Have you added tests for any new functions?

**Deliverables**
- CLI now accepts `--recipe-json` / `--recipe-file` and optional `--start` / `--end` trimming controls.
- Runtime recipe parsing & validation added (`RuntimeRecipe`, `RuntimeRecipeStep`) with conversion into existing `Recipe`.
- Helper functions for recipe/time parsing implemented with unit coverage.
- Whisper build robustness patch applied via vendored `whisper-rs-sys`.

**Tests**
- `cargo test` (2025-11-08): 35 passed, 1 ignored, 0 failed. Whisper build regenerates cleanly after interruption due to new build-script guard.
- `cargo clippy --all-targets --all-features` (2025-11-08): new Phase 1 code is lint-clean; remaining warnings stem from pre-existing unused types/fields (e.g., `Operation`, `ProcessingPlan`). These will be resolved when we simplify or reuse them in subsequent phases.

**Next Steps**
- Await user approval before marking `phase-2-pipeline` in progress.
- No additional actions until approval is received.

## Phase 2 – Pipeline Output Changes Status
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/JSON_RECIPE_ENHANCEMENTS.md?
- [x] **Code Simplicity**
- [x] **Code Modularity**
- [x] **Scope Control**
- [x] **No Dead Code**
- [x] **No Fake Constructions**
- [x] **Code Purpose**
- [x] **Required Tests**

**Deliverables**
- CLI now treats `OUTPUT_DIR` as a directory, validates it, and creates it when needed.
- Decoder output is trimmed to the requested `[start, end]` window before transcription; logging reflects effective ranges.
- For each audio chunk, the recipe is applied, assembled, and written to `chunk_{index}/processed.wav` under the output directory.
- Legacy single-file assembly removed; processing logs updated to track per-chunk outputs.

**Tests**
- `cargo clippy --all-targets --all-features` (2025-11-08): no new warnings beyond pre-existing unused-type/dead-code items.
- `cargo test` (2025-11-08): 35 passed, 1 ignored, 0 failed.

**Issues Encountered**
- None during Phase 2 implementation; per-chunk outputs and trimming behaved as expected.

## Phase 3 – Fast Playback Integrity & Testing Status
- [x] **Planning Documentation**
- [x] **Code Simplicity**
- [x] **Code Modularity**
- [x] **Scope Control**
- [x] **No Dead Code**
- [x] **No Fake Constructions**
- [x] **Code Purpose**
- [x] **Required Tests**

**Deliverables**
- Updated `operations::speed::change_speed` to request extra output, flush residual samples, drop leading latency, and normalize length, preventing fast-playback truncation.
- Added regression tests `test_speed_fast_preserves_tail_energy` and `test_recipe_uses_original_chunk_each_step` to confirm tail integrity and recipe reuse of the source chunk.

**Tests**
- `cargo clippy --all-targets --all-features` (2025-11-08): clean (no warnings).
- `cargo test` (2025-11-08): 32 passed, 1 ignored, 0 failed.

**Issues Encountered**
- Signalsmith Stretch introduces equal input/output latency (~2646 samples). Without trimming those leading samples, fast outputs ended in silence. Resolved by padding output length, flushing the stretcher, and discarding the latency window before truncation.

## Phase 4 – Documentation & Finalization Status
- [x] **Planning Documentation**
- [x] **Code Simplicity**
- [x] **Code Modularity**
- [x] **Scope Control**
- [x] **No Dead Code**
- [x] **No Fake Constructions**
- [x] **Code Purpose**
- [x] **Required Tests**

**Deliverables**
- Refreshed `ACTUAL_STATE.md` to capture JSON-configurable recipes, per-chunk output directories, trimming controls, latency fixes, new logging, and current test/clippy status.
- Rewrote `CLAUDE.md` usage instructions to document JSON recipe input (`--recipe-json/--recipe-file`), per-chunk outputs, start/end trimming, and updated build/test counts.
- Updated CLI documentation to note the removal of the legacy `--operations` flag and provide example JSON recipe usage.

**Tests**
- `cargo clippy --all-targets --all-features` (2025-11-08): clean.
- `cargo test` (2025-11-08): 34 passed, 1 ignored, 0 failed.

**Issues Encountered**
- None; documentation updates proceeded smoothly once the pipeline refactor was in place.

