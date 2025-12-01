# Pronunciation Metric and Visualization Rework Plan

## Phase 1: Session-Scaled Similarity Metric
- Subphase 1.1 (tests first): Delete current similarity-related tests in `tests/alignment.rs`. Add new BDD-style tests in `tests/alignment.rs` covering silence vs silence (top score), silence vs speech (worst score), speech match (near top), speech mismatch (degraded), and preservation of negative scores. No coding until these tests exist.
  - Deliverable: Updated `tests/alignment.rs` with new BDD tests and removed old similarity tests.
  - Checklist: BDD cases present; old similarity assertions removed; `cargo test --all` clean; `cargo clippy --all` clean.
- Subphase 1.2 (tests first, then implementation): Before any coding, delete/replace any alignment tests that conflict with session scaling; add new BDD tests for session-scaled behavior (distribution-derived scaling, negatives preserved). After tests are in place, update `src/pronunciation/alignment/mod.rs::compute_similarity` (and helpers) to unbounded, session-scaled similarity with explicit silence scoring (no clamping), adding helper inputs for session scale if required.
  - Deliverable: New similarity computation in the alignment module with supporting tests.
  - Checklist: New BDD session-scale tests pass; no unused code; `cargo test --all` clean; `cargo clippy --all` clean.

## Phase 2: Pitch/Energy Reliability (Noise vs Voice)
- Subphase 2.1 (tests first): Remove conflicting pitch/noise expectations in `tests/feature_extraction.rs` (or related). Add new BDD tests in `tests/pitch_voicing.rs` (or updated file) for white noise -> pitch 0, silence -> pitch 0, voiced sine -> correct pitch, soft->loud ramp -> stable pitch. No coding until these tests exist.
  - Deliverable: New BDD voicing/reliability tests; old ones removed.
  - Checklist: Tests compile; `cargo test --all` clean; `cargo clippy --all` clean.
- Subphase 2.2 (tests first, then implementation): Before coding, delete/replace any remaining tests around `frame_pitch` that conflict with reliability gating; add BDD tests for periodicity threshold behavior (noise stays 0, voiced stays voiced, ramp stays stable). Then update `src/pronunciation/features/mod.rs::frame_pitch` (and extractor helpers) with periodicity/reliability checks; keep explicit silence handling (pitch=0); adjust extractor as needed.
  - Deliverable: Revised pitch extractor with reliability gating and aligned tests.
  - Checklist: Voicing BDD tests pass; no panics on edge cases; `cargo test --all` clean; `cargo clippy --all` clean.

## Phase 3: Timeline Alignment Correctness
- Subphase 3.1 (tests first): Delete/replace start-frame expectations in `tests/session_engine.rs`. Add BDD tests for first chunk after tail includes first hop, frame starts stay on-grid across chunks, and silence spans align with reference indices. No coding until these tests exist.
  - Deliverable: Updated `tests/session_engine.rs` with BDD alignment cases; old misaligned tests removed.
  - Checklist: Tests reflect no skipped first hop; `cargo test --all` clean; `cargo clippy --all` clean.
- Subphase 3.2 (tests first, then implementation): Before coding, delete/replace any residual alignment tests that assume the old offset behavior; add BDD tests capturing correct first-hop inclusion and grid sync under varied chunk sizes. Then modify `src/pronunciation/features/mod.rs::extract_chunk` and `src/pronunciation/session/engine.rs::process_chunk` start-frame math to emit initial frames and keep grid sync.
  - Deliverable: Corrected frame indexing and grid alignment with updated tests.
  - Checklist: Alignment BDD tests pass; `cargo test --all` clean; `cargo clippy --all` clean.

## Phase 4: UI Visualization Mapping (No Hidden Clamps)
- Subphase 4.1 (tests first): Delete fixed 0–1 similarity UI expectations in `src/ui/screens/session.rs` tests. Add BDD tests asserting negative similarities render as bad, silence-vs-speech shows bad, silence-vs-silence shows good, and dynamic scaling reflects provided ranges. No coding until these tests exist.
  - Deliverable: Updated UI tests; old fixed-range tests removed.
  - Checklist: UI tests pass headless; `cargo test --all` clean; `cargo clippy --all` clean.
- Subphase 4.2 (tests first, then implementation): Before coding, delete/replace any residual UI tests that assume clamped ranges; add BDD tests for dynamic scale inputs and color mapping of negative values. Then update `draw_comparison_panel` and helpers in `src/ui/screens/session.rs` to use session-derived scale, show negative values distinctly, and remove the “good” clamp.
  - Deliverable: New heatmap/color mapping with dynamic scale and supporting tests.
  - Checklist: UI BDD tests pass; build succeeds; `cargo test --all` clean; `cargo clippy --all` clean.

## Phase 5: Integration Validation
- Subphase 5.1 (tests first): Delete any conflicting end-to-end tests. Add new BDD end-to-end tests in `tests/session_e2e.rs` (mock capture) for talking vs reference (good), silence vs silence (good), silence vs speech (bad), loudness ramp visible. No coding until these tests exist.
  - Deliverable: E2E BDD tests added; conflicts removed.
  - Checklist: Tests compile; `cargo test --all` clean; `cargo clippy --all` clean.
- Subphase 5.2 (tests first, then validation run): Before validation, add/adjust BDD-style acceptance checks (scripted assertions or doc-based expectations) capturing expected post-change behaviors; remove obsolete validation scripts. Then run full suite; capture updated screenshots for talking/silence with new mapping; note observed scales.
  - Deliverable: Clean test/clippy logs; screenshots and brief notes; updated acceptance checks.
  - Checklist: `cargo test --all` clean; `cargo clippy --all` clean; artifacts saved.
