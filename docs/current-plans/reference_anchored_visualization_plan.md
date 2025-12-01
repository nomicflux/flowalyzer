## Reference-Anchored Visualization Plan

### Phase 1: Energy Scaling to Reference
- **Subphase 1.1 – Reference range computation (`src/ui/screens/session.rs`: `draw_history_plot`, `shared_range`; tests in `mod tests`)**
  - Delete existing energy plot tests in `src/ui/screens/session.rs`.
  - Write new BDD tests expecting reference-derived min/max (or percentile) to be computed once and shared across reference/learner energy.
  - Implement reference range computation inside `draw_history_plot` using the reference series only; return/share that range.
  - Deliverable: reference energy range is computed and available for plotting both series.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.
- **Subphase 1.2 – Energy plot rendering with reference scale (`src/ui/screens/session.rs`: `draw_history_plot`, `draw_history_line`; tests in `mod tests`)**
  - Delete prior rendering assertions tied to auto-scaling.
  - Write new BDD tests that plot with a fixed reference range, asserting learner near-zero stays at baseline and reference fills the range.
  - Implement energy plot rendering to use the reference-derived range for both reference and learner series; adjust legend text.
  - Deliverable: energy plot uses reference-anchored scale; silent learner appears flat.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.

### Phase 2: Pitch/Contour Scaling to Reference Range
- **Subphase 2.1 – Pitch/contour range derivation (`src/ui/screens/session.rs`: pitch path in `draw_history_plot` reuse; tests in `mod tests`)**
  - Delete existing pitch/contour plot tests in `src/ui/screens/session.rs`.
  - Write new BDD tests that compute the pitch/contour range from the reference+learner series (no voiced-only filtering).
  - Implement range derivation for pitch/contour plots using shared ranges.
  - Deliverable: reference-derived range is computed for pitch/contour scaling.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.
- **Subphase 2.2 – Pitch/contour rendering with shared reference range (`src/ui/screens/session.rs`: `draw_history_plot`, `draw_history_line`; tests in `mod tests`)**
  - Delete prior pitch/contour rendering assertions tied to auto-scaling.
  - Write new BDD tests asserting plots use the shared reference-derived range and render learner near-zero when absent.
  - Implement rendering to use the shared reference range for pitch and contour plots; update legends.
  - Deliverable: pitch/contour plots anchored to the reference-derived range without voiced-only filtering.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.

### Phase 3: Heatmap Fixed Normalization
- **Subphase 3.1 – Fixed scale selection (`src/ui/screens/session.rs`: `draw_comparison_panel`, `data_range`; tests in `mod tests`)**
  - Delete existing heatmap tests in `src/ui/screens/session.rs`.
  - Write new BDD tests expecting fixed scales for similarity (0–1) and contour (small symmetric band around 0) selection.
  - Implement scale selection logic in `draw_comparison_panel` to pass fixed ranges into rows.
  - Deliverable: fixed scales chosen and applied for similarity/contour heatmaps.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.
- **Subphase 3.2 – Heatmap rendering with fixed scales (`src/ui/screens/session.rs`: `draw_row`, `similarity_color`, `contour_color`; tests in `mod tests`)**
  - Delete prior heatmap rendering assertions tied to per-row min/max.
  - Write new BDD tests asserting flat colors when variation is tiny and proper gradient when values span the fixed range.
  - Implement `draw_row` normalization using provided fixed ranges and keep palettes; adjust if needed.
  - Deliverable: heatmap reflects fixed scales; silent/low-variation data appears flat/uniform.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.

### Phase 4: Range Hints and Legends
- **Subphase 4.1 – UI legend/label updates (`src/ui/screens/session.rs`: labels in `draw_comparison_panel`, history plot titles; tests in `mod tests`)**
  - Delete any doc/legend checks tied to old scaling.
  - Write new BDD tests ensuring labels describe reference-based scaling and fixed heatmap ranges.
  - Implement legend/label updates in `draw_comparison_panel` and history plot titles.
  - Deliverable: UI legends match reference-anchored scaling semantics.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.
- **Subphase 4.2 – Documentation alignment (`docs/current-plans/` and inline comments if needed)**
  - Delete stale doc references to old scaling behavior.
  - Write doc/check updates describing reference-based scaling and fixed heatmap normalization.
  - Implement doc updates accordingly.
  - Deliverable: docs align with new scaling semantics.
  - Check: confirm deliverable; run `cargo test --all` until clean; run `cargo clippy --all` until clean.

---

## Status / Notes
- Energy: reference-only range; learner energy rendered on that scale. Label updated to “Waveform energy (reference scale)”.
- Pitch/contour: shared reference+learner range (no voiced filtering) drives rendering; title notes shared range.
- Heatmaps: similarity fixed to 0–1, contour fixed to ±0.5; normalization uses these fixed spans and legends call them out.
- Tests/clippy: `cargo test --all`, `cargo clippy --all -- -D warnings` passing after these changes.
