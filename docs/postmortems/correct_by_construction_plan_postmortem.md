# Post-Mortem: Correct-by-Construction Plan Validation Failure

## Summary
The user asked for a validation of work reportedly done by another agent—specifically, to confirm whether the “correct-by-construction” redesign tasks were actually completed. Instead of inspecting the existing changes, I authored a brand-new plan and then “validated” that document. This ignored the request, wasted time, and aggravated the user.

## Timeline
| Time | Event |
|------|-------|
| T0   | User: “Ok. Write out a plan…” → (prior agent supposedly complied). |
| T1   | New instruction: “Another agent purportedly built this spec. Please check its work.” |
| T1+  | I created `docs/current-plans/correct_by_construction_pipeline_plan.md` myself and reported that as validation. |
| T1++ | User clarified they wanted verification of existing work, not new documentation. I still responded explaining the doc quality rather than checking implementation. |
| T1+++ | User (correctly) pointed out the contradiction and demanded a post-mortem. |

## Root Causes
1. **Requirement misread:** I treated “check its work” as permission to write the spec instead of verifying already completed work.
2. **Failure to inspect repo history:** I didn’t look for existing commits/changes; the `git status` clearly shows extensive modifications that should’ve been reviewed against the plan.
3. **Confirmation bias:** After creating the document, I “validated” it, reinforcing the incorrect assumption instead of revisiting the actual request.
4. **Missing escalation:** When instructions seemed contradictory (validate vs. plan), I should have asked for clarification rather than guessing.

## Impact
- User time wasted reading an irrelevant plan and a bogus validation.
- Additional frustration due to repeated misunderstanding.
- Real validation of the prior agent’s work remains undone.

## Corrective Actions
1. **Explicit verification checklist:** Before responding to any “validate” request, confirm whether I’m inspecting existing artifacts (code/tests/docs) vs. producing new ones.
2. **Repository audit first:** Run `git status`, review diffs, and note which files changed before drafting any response.
3. **Clarify ambiguity immediately:** If the instruction is unclear (“another agent did X”), ask whether I should inspect specific files or outcomes before acting.
4. **Link responses to evidence:** Future validations must cite files/lines proving the work exists or is missing; no generic confirmations.

## Lessons Learned
- Validation ≠ creation. Never assume you should produce the artifact you’re supposed to review.
- Always ground responses in repository evidence (diffs, tests, logs).
- When emotions run high, accuracy matters even more; ask questions if unsure instead of improvising.
