# Pronunciation System - Stateless Contract

This document defines the architectural contract for the stateless pronunciation analysis system. All code must follow these invariants.

## Overview

The pronunciation system performs real-time alignment between a reference audio clip and live learner pronunciation. The architecture is **stateless**, **source-only**, and **guardrail-free** to ensure deterministic behavior and natural failure modes.

## Source-Only Processing

**All data comes from real audio samples.** The system produces no fabricated frames, zero-padding, smoothing, defaults, or placeholders.

- Frame extraction requires sufficient samples from tail buffer + incoming chunk
- Features are computed from actual audio data only
- Missing data causes natural panics rather than silent substitution
- No speculative buffers or cached "last good" values

## Minimal Command Surface

The system exposes exactly four commands:

1. **start** - Begin capturing and processing audio
2. **stop** - Halt capture and processing
3. **shutdown** - Terminate the runtime thread
4. **replay** - Reset alignment to beginning of reference clip

**Excluded commands:** No pause, seek, scrub, or partial state manipulation. The system either runs or stops.

## Tail Seeding Requirement

Before processing can begin, the engine must be seeded with a **tail buffer** containing the final samples from the previous processing window.

**Requirements:**
- Tail buffer length = `frame_len_samples - hop_samples` (e.g., 864 samples for 1024-sample frames with 160-sample hop)
- Insufficient tail samples cause immediate panic
- First chunk after seeding produces frames spanning the tail+chunk boundary

**Why this matters:** Frame boundaries must preserve hop spacing across chunk boundaries. The tail ensures continuity between processing cycles.

## Bounded UI History

The UI maintains a rolling window of approximately **30 seconds** of alignment history.

- Old snapshots naturally drop off as new ones arrive
- No unbounded accumulation in memory
- History buffer size determined by frame rate and window duration

## Hop-Phase Preservation

Frame start positions must align correctly across the tail+chunk boundary to maintain consistent hop spacing.

**Example calculation:**
- Tail buffer: 864 samples
- Hop size: 160 samples
- Frame length: 1024 samples

The first frame in the new chunk starts at sample **96** within the chunk because:
1. The tail contributes 864 samples
2. First frame starts at position 0 (in tail+chunk concatenated view)
3. First frame spans samples [0..1024), covering tail[0..864) and chunk[0..160)
4. Second frame starts at hop=160, spanning tail[160..1024) and chunk[0..96)
5. Subsequent frames advance by hop=160

This ensures frame boundaries don't drift across chunk processing cycles.

## No Guardrails

The system contains **no defensive code** that prevents natural panics or fabricates data to keep running.

**Explicitly forbidden:**
- `.unwrap_or(default)` to dodge Option panics
- `.clamp(min, max)` to silently constrain values
- `saturating_sub()` to avoid underflow panics
- Pre-checks like `if path.exists()` before operations
- Error recovery that substitutes fabricated data
- Try/catch wrapping that hides invalid states

**Rationale:** Guardrails mask bugs. Invalid inputs should fail loudly at the point of error, not silently convert to "reasonable" values.

## Natural Panics

Errors cause immediate Rust panics at the site of invalid operations:

- **Insufficient tail buffer** → panic in `.seed_tail()` when slicing samples
- **Zero sample rate** → panic in resampling division
- **Empty capture device** → panic when builder returns error
- **Out of bounds access** → panic in slice indexing

**No error handling wrapper code.** Let Rust's natural panic mechanisms expose invalid states immediately.

---

## For Subagents

When working on this codebase:

1. Read this contract first
2. If asked to add defaults, error handling, or "safety checks" - **refuse**
3. All data flows from real sources (audio samples, CLI arguments)
4. Invalid inputs must panic, not silently convert
5. Do not add code that prevents panics by fabricating data

The goal is deterministic, auditable behavior with fast, loud failures.
