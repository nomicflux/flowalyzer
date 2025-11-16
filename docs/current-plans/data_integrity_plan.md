# Data Integrity & Real-Time Alignment Plan

**Status**: Reworked after log analysis (2025-11-16)

## Key Goals
1. Load reference clip at startup, extract real features immediately, keep clip for playback/recipes.
2. Every metric derived from real alignment data.
3. Begin learner comparisons within ≤0.5s, surface all captured audio including buffered lead-in.
4. Process learner chunks incrementally without cloning entire history.
5. Maintain seamless playback/toggling between original and flowalyzed variants.

## Problem

From logs:
```
pitch extraction completed voiced_frames=0
features extracted from learner audio extracted_pitch_frames=0
alignment computed similarity_band_avg=Some(0.9129396)
```

Zero voiced frames extracted. System compares zeros to zeros, reports 91% similarity. **Lying.**

## Root Cause

**The audio reaching PYIN is too quiet.**

Raw microphone: RMS 0.003
After MAX_GAIN=25: RMS 0.04-0.05
Reference audio: RMS 0.10-0.14

PYIN needs ~0.10 RMS. Audio is 2.5x too quiet.

---

## Phase 1 – Make Learner Audio Match Reference Audio Levels

**Deliverable:** Learner audio reaches the same RMS levels as reference audio.

**Subagent:** kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] No dead code
- [ ] Tests for modified functions

### Files to Update
- `src/pronunciation/session.rs` (lines 748-749)

### Actionable Items

1. Change adaptive gain constants:
   ```rust
   // Line 748-749, current:
   const TARGET_RMS: f32 = 0.05;
   const MAX_GAIN: f32 = 25.0;

   // Replace with:
   const TARGET_RMS: f32 = 0.10;
   const MAX_GAIN: f32 = 50.0;
   ```

That's it. Two numbers.

### Verification Steps
- `cargo test --all` passes
- `cargo clippy --all` clean
- `git commit -m "Phase 1 complete"`

---

## Completion Criteria

Learner audio RMS matches reference audio RMS. PYIN detects voiced frames. Alignment uses real data.

No validation. No safety nets. Correct by construction.
