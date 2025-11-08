## Agreements Made
- *2025-11-08*: User specified CLI should accept JSON recipes: "JSON is actually probably more flexible, let's use JSON."
- *2025-11-08*: User requested single inclusive trim range per run: "Single range is fine. We can re-run the app on multiple ranges."
- *2025-11-08*: User outlined recipe semantics and goals (configurable per run, per-chunk outputs, fix fast playback truncation, start/stop trimming).

## Explicitly Rejected
- No alternatives rejected yet.

## Implementation Details
- Phase 1 focuses on parsing JSON recipe descriptions and optional start/end times at the CLI boundary.
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

## Issues Encountered
- None encountered during Phase 1 implementation to date.

