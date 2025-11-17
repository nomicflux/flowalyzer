# Alignment Timeline & Pitch Simplification Plan

## Overview
Ruthlessly simplify the alignment/pitch pipeline by (1) accumulating real per-frame metrics for at least 30 s of visualization history, (2) removing synthetic “phoneme” segments in favor of strictly time-based comparisons, and (3) stripping away gain/initialization side effects so pitch contours always reflect real learner audio.

Every phase must keep all five key goals intact and finishes only when `cargo test --all` and `cargo clippy --all -- -D warnings` both pass cleanly.

---

## Phase 1 – Add AlignmentTimeline History
**Deliverable:** `SessionSnapshot` exposes rolling frame-series (≥30 s) for similarity, contour, energy, and pitch so the UI can render long-lived timelines without recomputing per chunk.

Actionable steps:
1. Define `AlignmentTimeline` (e.g., `src/pronunciation/session.rs`) holding vectors for reference/learner energy, reference/learner pitch, similarity, and contour; expose APIs to append 10 ms samples and drop old entries beyond 3000 frames (~30 s).
2. Extend `AlignmentReport` (and `SessionSnapshot`) with optional timeline fields or embed the new struct so downstream code can access both the latest 2 s alignment result and the historical samples (`src/pronunciation/mod.rs`, `src/pronunciation/session.rs`).
3. After each `process_chunk`, convert the new DTW path into frame-based samples (reuse/relocate `project_bands_to_frames` logic out of the UI) and append them to the timeline before emitting the snapshot (`src/pronunciation/session.rs`).
4. Update `SessionApp::sync_visuals` and `build_spectrogram_window` to read from the timeline fields instead of recomputing from a single chunk; bump `FRAME_WINDOW` to ≥3000 so the spectrogram/waveform/pitch graphs show ~30 s (`src/ui/screens/session.rs`).
5. Add/regenerate tests covering the new timeline behavior (e.g., ensure successive snapshot updates extend the history, `src/ui/screens/session.rs` tests or new tests under `tests/`).
6. Run `cargo fmt`, then `cargo test --all`, then `cargo clippy --all -- -D warnings`.

---

## Phase 2 – Replace Synthetic Phonemes with Time Buckets
**Deliverable:** Alignment data models consist only of true time buckets; pseudo-phoneme structs are removed and the UI displays bucket metrics rather than invented phoneme labels.

Actionable steps:
1. Delete `SegmentAccumulator`/`AlignedPhoneme` synthesis from `src/pronunciation/alignment/mod.rs`; instead, emit a `Vec<AlignmentBucket>` where each bucket corresponds to a real-time span (e.g., 10 ms or aggregated 50/100 ms) derived directly from the DTW path.
2. Update `AlignmentReport` to store these buckets (replace `phonemes` with buckets or rename appropriately) plus per-bucket metrics (timing delta, similarity, contour). Adjust serde derives as needed (`src/pronunciation/mod.rs`).
3. Rewrite `SessionApp` consumers: remove phoneme-aligned slicing helpers, delete `PhonemeTimeline`, and show bucket-based data for whatever UI elements remain (`src/ui/components/phoneme_timeline.rs`, `src/ui/screens/session.rs`).
4. Fix or remove tests that referenced synthetic phonemes (e.g., `tests/alignment.rs`, `tests/session_smoke.rs`), adding new ones that assert bucket timing aligns with the learner offsets.
5. Re-run `cargo fmt`, `cargo test --all`, and `cargo clippy --all -- -D warnings`.

---

## Phase 3 – Simplify Pitch/Gain Path (“Correct by Construction”)
**Deliverable:** Feature extraction uses the real learner audio without adaptive gain/silence trimming side effects, and pitch contours only emit zeros for true silence.

Actionable steps:
1. Remove `pending_offset_ms`, `applied_offset_ms`, and silence-trimming logic that mutates `learner_buffer`; maintain a single monotonic sample index to preserve offsets without deleting audio (`src/pronunciation/session.rs:1171-1485`).
2. Delete or bypass `apply_adaptive_gain` and associated RMS gating; pass the resampled chunk directly into the processing buffer so PYIN always sees the original dynamics (`src/pronunciation/session.rs:1334-1356`).
3. Adjust `extract_pitch_contour_with_reporting` so missing/unvoiced frames stay `None` through the pipeline; update `fill_missing_with_reporting`/`align_to_frames` to preserve “no data” instead of forcing zeros (`src/pronunciation/features/contour.rs`).
4. Ensure UI components treat missing pitch as empty gaps rather than flat zero lines (e.g., update rendering logic in `src/ui/components/pitch.rs` and `SessionApp::sync_visuals`).
5. Expand/modify unit tests to cover voiced audio producing nonzero contours and silence yielding empty vectors; add regression coverage for the simplified ingestion path if needed.
6. Finish with `cargo fmt`, `cargo test --all`, and `cargo clippy --all -- -D warnings`.

---

Completion requires executing all phases in order, keeping every touched function simple (≤20 lines whenever possible), and validating with the mandated test/clippy runs after each phase.
