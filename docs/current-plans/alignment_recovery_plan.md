# Alignment Recovery & Verification Plan

Key lessons (must be honored in every phase):
1. Treat the five key goals as hard acceptance criteria for every change.
2. Preserve downstream consumers (UI visualizations) whenever refactoring engine code.
3. Validate user-visible outputs, not just scalar fields, through automated and manual checks.
4. Pair performance/latency improvements with quality guards so we never ship empty or saturated metrics.
5. Add regression coverage for every data path that the UI relies on before shipping incremental processing changes.

## Key Goals (hard requirements)
1. Reference clip loads at startup, real features are extracted immediately, and clips stay available for playback and recipes.
2. Every reported reference metric (phonemes, contour, similarity, confidence) is derived from real alignment data.
3. Learner comparisons begin within ≤0.5 s and surface all captured audio, including buffered lead-in.
4. Learner chunks are processed incrementally without cloning entire history, keeping latency and CPU minimal.
5. Playback/toggling between original and flowalyzed clips remains seamless using preserved clips and cached features.

## Phase 1 – Restore Reliable Alignment Data
**Deliverable:** `AlignmentReport` once again carries non-empty, normalized `similarity_band`, `contour_band`, and pitch vectors that match real learner audio while honoring offsets.

Actionable subtasks (each checked against all key goals):
- Audit current learner-buffer mutations and revert any destructive updates that empty visualization vectors, while keeping reference initialization untouched (Goals 1,2,5).
- Reintroduce a non-destructive trimming path: build a temporary slice for feature extraction, apply silence trimming to that slice only, and leave `learner_buffer` intact for visualization (Goals 2–4).
- Ensure `pending_offset_ms`, `applied_offset_ms`, and spectrogram data remain in sync without dropping lead-in samples (Goals 2 & 3).
- Add regression tests that assert `similarity_band` and `contour_band` are non-empty after processing a silence+speech capture and that their first columns align with `learner_offset_ms` (Goals 2 & 3).
- Step: rewrite any function in touched files that now exceeds 20 lines to keep maintenance manageable (Goals 2–4 by clarity).
- Step: run `cargo test --all` repeatedly until it yields exactly zero failures (protects all goals by ensuring no regressions).
- Step: run `cargo clippy --all` with `-D warnings` repeatedly until it yields exactly zero failures (enforces correctness and style supporting every goal).

## Phase 2 – Balanced Gain & Incremental Processing
**Deliverable:** Adaptive gain and incremental chunk handling keep pitch contour within realistic ranges while preserving ≤0.5 s comparisons and low CPU usage.

Actionable subtasks:
- Replace the hard clamp with a soft-limiter or RMS-driven gain curve that keeps samples within ±1 without flattening pitch dynamics (Goals 2 & 3).
- Validate adaptive gain via unit tests covering quiet, nominal, and loud inputs to guarantee no saturation while maintaining incremental processing (Goals 2 & 4).
- Confirm incremental helpers (ingest, windowing) leave enough buffered context for both alignment and playback, documenting invariants about untouched clip caches (Goals 1,4,5).
- Re-run the mock-capture silence+speech test to confirm both latency (<0.5 s) and spectrogram data remain correct (Goals 3 & 4).
- Step: ensure all modified functions remain under 20 lines or are rewritten into helpers (clarity ensures Goals 2–5 stay maintainable).
- Step: run `cargo test --all` until zero failures.
- Step: run `cargo clippy --all` until zero failures.

## Phase 3 – UI Data Verification & Manual Checklist
**Deliverable:** Documented verification that UI receives real-time spectrogram/pitch data aligned with learner offsets while playback/toggling stays seamless.

Actionable subtasks:
- Extend UI-side tests around `build_spectrogram_window` to cover cases with large offsets so the projected timeline stays aligned with reference frames (Goals 2 & 3).
- Add integration coverage that toggles clip variants after recording to confirm cached clips continue providing valid features (Goals 1 & 5).
- Write a concise manual checklist (to be executed by the designated UI runner) describing how to verify mic warmup messaging, pitch contour ranges, and spectrogram rendering after silence lead-ins (all goals, with emphasis on 2,3,5).
- Step: verify all functions touched in this phase stay under 20 lines or are refactored to be so.
- Step: run `cargo test --all` until zero failures.
- Step: run `cargo clippy --all` until zero failures.

**Completion requires:** every phase’s deliverable met, all subtasks satisfied without violating any key goal, every touched function ≤20 lines, and both `cargo test --all` plus `cargo clippy --all` passing cleanly. No exceptions.
