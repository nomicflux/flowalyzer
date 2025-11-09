## Phase 1 – Pause Detection Helper
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PAUSE_AWARE_CHUNKING.md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [x] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [x] **Required Tests**: Have you added tests for any new functions?

## Notes
- Added docs/current-plans/PAUSE_AWARE_CHUNKING.md for tracking.
- Implemented `audio::pause_detector::detect_pauses` with windowed energy analysis and unit tests.
- Ran `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test` (37 passed, 1 ignored).

## Phase 2 – Integrate Pauses into Chunking
- [x] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/PAUSE_AWARE_CHUNKING.md?
- [x] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [x] **Code Modularity**: Are you following modularity rules? (helper functions, low cyclomatic complexity)
- [x] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [x] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [x] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [x] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [x] **Required Tests**: Have you added tests for any new functions?

### Notes
- Added `chunking::calculate_chunk_boundaries` to split transcript segments at detected pause midpoints before greedy aggregation.
- Updated `main::plan_chunks` to detect pauses from raw audio using a 50 ms analysis window, 0.04 relative amplitude threshold, and target-duration-scaled silence duration.
- Added regression test `test_prefers_pause_boundaries` to confirm that pause-aware chunking avoids mid-phrase cuts.
- Refactored `chunking::calculate_chunk_boundaries` into dedicated `planner`, `accumulator`, and `spans` modules so each function stays under 20 lines while preserving chunking behavior.
- Moved chunking regression tests into `chunking::tests` for cleaner module structure.
- Ran `cargo fmt`, `cargo clippy --all-targets --all-features` (clean), and `cargo test` (38 passed, 1 ignored).

