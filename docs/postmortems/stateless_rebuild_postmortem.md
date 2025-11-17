# Post-Mortem: Stateless Runtime Rebuild False Start

## Summary

We were instructed to rebuild the pronunciation session architecture “from scratch” with ruthless simplicity: no incremental cleanup, no historical timelines in the engine, and history owned entirely by the UI. Instead of honoring that directive, we attempted to salvage the old modules with piecemeal edits (tweaking `SessionEngine`, adding guard tests, patching pitch-contour helpers). The result was a muddled hybrid that neither preserved the original behavior nor delivered the stateless design.

This document records what went wrong so we do not repeat the mistake.

## Timeline of Missteps

| Date (relative) | Misstep | Impact |
|-----------------|---------|--------|
| T0              | Treated the task as an incremental refactor. We tried to “clean up” the existing `session` module rather than deleting it. | Old timeline-oriented types and UI plumbing stayed in place, making statelessness impossible to guarantee. |
| T0 + 1          | Added new integration tests (`stateless_processing.rs`) before the architecture existed. | Tests reinforced the old structure (directly poking `SessionEngine`) and diverted attention from the actual rebuild. |
| T0 + 2          | Tweaked pitch-contour code repeatedly to satisfy legacy tests. | Burned time on legacy behavior instead of executing the teardown. |
| T0 + 3          | Continued modifying UI/state types piecemeal (“cleanup”) rather than performing the documented teardown. | Confusion escalated; we were fighting the old design rather than replacing it. |

## Root Causes

1. **Failure to honor “from scratch”.** We misread “reimplement from scratch” as “refactor heavily”. That led to incremental edits and guard tests designed to protect the old structures.
2. **No upfront documentation.** We dove straight into code without writing the replacement spec. Without a blueprint, we kept discovering legacy dependencies mid-change and tried to patch around them.
3. **Premature testing.** By adding integration tests for a design that didn’t exist, we anchored ourselves to the old API and slowed down the teardown.
4. **Over-reliance on incremental guarantees.** Trying to prove statelessness via tests before the types enforced it was backwards: the compiler should make history storage impossible; the tests should confirm behavior after the fact.

## Corrective Actions

1. **Write the spec first (complete).** `docs/stateless_runtime_spec.md` now captures the clean-room design. All future implementation work must reference that document explicitly.
2. **Teardown before build.** Implementation resumes only after deleting the existing `session` tree and UI history plumbing. No file survives unless it conforms to the new spec.
3. **Delay integration tests.** We will not add new tests until the new runtime exists. When we do, they will exercise the runtime via `SessionHandle`, not internal structs.
4. **Review checkpoints.** Every stage (teardown, engine, runtime, UI histories) will include a short note referencing the spec so reviewers can confirm we’re still aligned.

## Lessons Learned

* “Impossible by construction” is an architectural constraint, not a test harness. Enforce it via types and module boundaries, not just assertions.
* When asked to “rebuild from scratch”, do not carry forward any implementation artifacts unless explicitly approved.
* Documentation is a prerequisite for drastic rewrites; without a spec, code tends to drift back toward the old design.
* Any deliverable that ships without its core functionality is not complete; treating it as done hides unfinished work.

With the spec and post-mortem in place, we now have a clear path to rebuild the runtime properly. Any future deviation from the spec needs to be justified in writing before touching code. 
