# Lessons Learned (for future agents)

## Summary of What Went Wrong
- Changed code without confirming with the user first (e.g., removing/re-adding chunk accumulation, tweaking contour logic, UI “legibility” tweaks). This caused churn and mistrust.
- Introduced hacks and placeholders that violated constraints (fake durations, zero-handling that wasn’t in the spec, normalization/legibility tweaks), instead of preserving raw data fidelity.
- Missed the real data-path/root-cause: counting device-rate samples toward a target defined at the configured rate meant resampled chunks were too short for PYIN, driving pitch/contour to extremes (solid red/flat).
- Communication failures: speculated about causes without evidence, explained after editing instead of aligning before.
- Kept helper code solely to satisfy tests/headless, conflicting with “no code just for tests.”

## Root Causes
- Assumed “fix forward” rather than pausing to confirm requirements.
- Prioritized visual tweaks over correctness and spec fidelity (silence-as-match, no fake values).
- Didn’t validate sampling/unit assumptions early (device rate vs. target rate).
- Reactive edits created rollback thrash.

## Actions to Prevent Repeat
- Align on plan/changes before editing; no speculative fixes.
- Start with the data path: validate sample-rate/length assumptions; fix accumulation/resample math first.
- Preserve raw data: no fake values, no legibility hacks; follow silence-as-match and other explicit constraints.
- Remove test-only helpers; keep code driven by real requirements.
- Communicate with evidence: analyze first, propose with rationale, then edit.***

## New Lessons
- When the plan seems contradictory (e.g., hop-phase start vs. tail contribution), stop and ask the user for clarification before coding—never guess or “fix” around the spec. I’ll defer to the plan and ask next time rather than guessing.
- When tests and the plan diverge, update the tests to the plan; do not bend the implementation to satisfy legacy expectations.
- Guardrails include silent filters (dropping negative starts, clamping); removing asserts isn’t enough—avoid any pre-checks that change behavior for invalid inputs.
- Preserve tail contribution and hop phase as written; do not reinterpret examples or adjust offsets without confirmation.***
