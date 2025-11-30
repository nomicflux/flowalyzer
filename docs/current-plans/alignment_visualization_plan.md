## Alignment & Visualization Reliability Plan

### Phase 1: Verify Data Coverage (alignment length)
- **Subphase 1.1 – History accumulation (`src/ui/screens/session.rs`)**
  - Delete existing history/window tests in `src/ui/screens/session.rs` test module.
  - Write tests (same file) for `HistoryBuffers::accumulate` and `trim_to_window` expecting ~450 frames for 4.5s @10ms and correct trimming.
  - Implement/adjust `HistoryBuffers::accumulate`/`trim_to_window` to satisfy tests.
  - Deliverable: histories retain full 4.5s frames and trim properly.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 1.2 – Runtime snapshot production (`src/pronunciation/session/runtime.rs`)**
  - Delete frame-count coverage tests in `tests/stateless_runtime.rs` and `tests/session_engine.rs`.
  - Write tests in `tests/stateless_runtime.rs` expecting `AlignmentReport` start/end indices to advance over full 4.5s (~450 frames) with no early cutoff.
  - Implement/adjust snapshot generation in `src/pronunciation/session/runtime.rs` to satisfy tests.
  - Deliverable: runtime emits snapshots spanning full clip without truncation.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 1.3 – UI accumulation loop (`src/ui/screens/session.rs`)**
  - Delete UI accumulation/plot tests in `src/ui/screens/session.rs`.
  - Write tests in `src/ui/screens/session.rs` for `SessionApp::apply_snapshot` to append multi-chunk (>300 frames) sequences and clear on restart.
  - Implement/adjust `SessionApp::apply_snapshot` as needed to satisfy tests.
  - Deliverable: UI accumulates full-length sequences across chunks.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.

### Phase 2: Fix Pitch Extraction/Voicing
- **Subphase 2.1 – Pitch computation core (`src/pronunciation/alignment/mod.rs`)**
  - Delete pitch/voicing tests in `tests/alignment.rs` and `tests/pronunciation_types.rs`.
  - Write tests in `tests/alignment.rs` for voiced+silence+voiced fixture expecting reference/learner pitch near zero in silence and aligned in voiced segments.
  - Implement/adjust core pitch handling (e.g., `compute_contour_band` or helper) in `src/pronunciation/alignment/mod.rs` to satisfy tests.
  - Deliverable: core pitch returns stable voiced-only contours.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 2.2 – Smoothing/voicing gate (`src/pronunciation/alignment/mod.rs`)**
  - Delete unsmoothed pitch expectations in `tests/alignment.rs`.
  - Write tests in `tests/alignment.rs` for small-window smoothing and a voicing gate that zeroes unvoiced frames while preserving voiced shape.
  - Implement/adjust smoothing/voicing helper in `src/pronunciation/alignment/mod.rs` to satisfy tests.
  - Deliverable: smoothing + voicing gate applied; silence near zero, voiced shape preserved.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 2.3 – Session pipeline integration (`src/pronunciation/session/runtime.rs`)**
  - Delete prior pitch behavior tests in `tests/session_engine.rs`.
  - Write integration tests in `tests/session_engine.rs` for 4.5s voiced/silence pattern ensuring finite pitch arrays with correct segmentation.
  - Implement/adjust pitch path in `src/pronunciation/session/runtime.rs` to satisfy tests.
  - Deliverable: runtime emits pitch arrays matching expected segmentation.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.

### Phase 3: Make Similarity Interpretable via Scaling (no clamps)
- **Subphase 3.1 – Scaling logic (`src/pronunciation/alignment/mod.rs`)**
  - Delete similarity tests in `tests/alignment.rs`.
  - Write tests in `tests/alignment.rs` for `compute_similarity` (or helper) expecting stable scaling (finite, ordered: better match → higher score) and documented zero meaning.
  - Implement/adjust similarity scaling in `src/pronunciation/alignment/mod.rs` to satisfy tests.
  - Deliverable: similarity uses stable, interpretable scaling.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 3.2 – Runtime propagation (`src/pronunciation/session/runtime.rs`)**
  - Delete old similarity range tests in `tests/stateless_runtime.rs`.
  - Write tests in `tests/stateless_runtime.rs` for multi-chunk scaled `similarity_band` ensuring finite values across the clip.
  - Implement/adjust propagation in `src/pronunciation/session/runtime.rs` to satisfy tests.
  - Deliverable: scaled similarity flows through reports.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 3.3 – UI display semantics (`src/ui/screens/session.rs`)**
  - Delete UI similarity/contour tests in `src/ui/screens/session.rs`.
  - Write UI-level tests in `src/ui/screens/session.rs` asserting plots/legends display the new scaled range correctly.
  - Implement/adjust rendering in `src/ui/screens/session.rs` to use the scaled values and updated legends.
  - Deliverable: UI conveys interpretable similarity consistent with scaling.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.

### Phase 4: Improve Visualization (shared axes, smoothing, stable heatmap)
- **Subphase 4.1 – Pitch/energy plots (`src/ui/screens/session.rs`)**
  - Delete plot rendering tests in `src/ui/screens/session.rs`.
  - Write tests in `src/ui/screens/session.rs` for `draw_history_plot`/`draw_history_line` expecting shared axes per series and optional light smoothing for similarity/contour lines, handling near-constant data.
  - Implement/adjust plot helpers in `src/ui/screens/session.rs` to satisfy tests.
  - Deliverable: plots readable with minimal jitter and correct scaling.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 4.2 – Heatmap normalization (`src/ui/screens/session.rs`)**
  - Delete heatmap tests in `src/ui/screens/session.rs`.
  - Write tests in `src/ui/screens/session.rs` for `draw_comparison_panel`/`draw_row` to use stable normalization (not per-row min/max), validating color variation and tiny-span stability.
  - Implement/adjust heatmap normalization in `src/ui/screens/session.rs` to satisfy tests.
  - Deliverable: heatmap reflects proximity without clipping artifacts.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
- **Subphase 4.3 – Docs/legends (`src/ui/screens/session.rs`, docs)**
  - Delete doc/legend checks tied to old visuals.
  - Write tests (or doc checks if applicable) ensuring legends/labels describe scaled similarity meaning and pitch voicing behavior; locations: `src/ui/screens/session.rs` labels and relevant docs (`docs/current-plans/...`).
  - Implement/adjust legend strings and docs accordingly.
  - Deliverable: legends/docs match new scaling and pitch handling.
  - Check: confirm deliverable; `cargo test --all`; `cargo clippy --all`.
