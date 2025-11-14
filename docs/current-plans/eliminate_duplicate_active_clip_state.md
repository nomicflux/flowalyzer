# Eliminate Duplicate Active Clip State

## Problem
`EngineRunner` maintains duplicate state for "which variant is active":
- `EngineRunner.active_clip: ActiveClip` encodes both storage (clip data) and state (which is active)
- `SessionSnapshot.active_clip_variant: ClipVariant` is the UI's source of truth
- Code overwrites snapshot with engine state, causing sync issues (e.g., replay switches back to Flowalyzed after user selects Original)

## Solution
Make `SessionSnapshot.active_clip_variant` the single source of truth. `EngineRunner` should only store clip data, not track which variant is active.

## Implementation

### Phase 1: Refactor EngineRunner Storage

**Files to modify:**
- `src/pronunciation/session.rs`

**Code style checklist:**
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, separate modules for separate functionality)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Implementation steps:**

1. Change `EngineRunner.active_clip: ActiveClip` to `flowalyzed_clip: Option<RecordedClip>`:
   - Update struct definition at line 1145
   - Change from `active_clip: ActiveClip` to `flowalyzed_clip: Option<RecordedClip>`

2. Update `EngineRunner::build()` to initialize `flowalyzed_clip: None`:
   - At line 1222, change `active_clip: ActiveClip::Original` to `flowalyzed_clip: None`

3. Remove `active_variant()` method entirely (lines 1235-1239):
   - This method is dead code after refactor

4. Update `active_clip()` method (lines 1228-1233) to take `variant: ClipVariant` parameter:
   ```rust
   fn active_clip(&self, variant: ClipVariant) -> Result<&RecordedClip> {
       match variant {
           ClipVariant::Original => Ok(&self.original_reference),
           ClipVariant::Flowalyzed => {
               self.flowalyzed_clip.as_ref()
                   .ok_or_else(|| PronunciationError::new("flowalyzed clip not available"))
           }
       }
   }
   ```

5. Update `can_toggle_to_flowalyzed()` (line 1505) to check `self.flowalyzed_clip.is_some()`:
   ```rust
   fn can_toggle_to_flowalyzed(&self) -> bool {
       self.flowalyzed_clip.is_some()
   }
   ```

6. Update `complete_recipe_application()` (lines 1421-1432) to store clip in `flowalyzed_clip`:
   ```rust
   fn complete_recipe_application(
       &mut self,
       new_clip: ActiveClip,
       updates: &Sender<SessionSnapshot>,
       snapshot: &mut SessionSnapshot,
   ) {
       let ActiveClip::Flowalyzed(clip) = new_clip else {
           unreachable!("generate_flowalyzed_clip always returns Flowalyzed variant");
       };
       self.flowalyzed_clip = Some(clip);
       snapshot.active_clip_variant = ClipVariant::Flowalyzed;
       snapshot.recipe_applying = false;
       info!("flowalyzed clip generated and activated successfully");
       let _ = updates.send(snapshot.clone());
   }
   ```

7. Update `activate_flowalyzed_clip()` (lines 1447-1456) to extract clip from `ActiveClip`:
   - The method already extracts `ActiveClip::Flowalyzed(clip)`, so it can pass `clip` directly to `cache_and_activate_flowalyzed`

**Deliverables:**
- `EngineRunner` uses `flowalyzed_clip: Option<RecordedClip>` instead of `active_clip: ActiveClip`
- `active_variant()` method removed
- `active_clip()` updated to take variant parameter and return `Result`
- `can_toggle_to_flowalyzed()` updated
- `complete_recipe_application()` stores clip separately
- Code compiles (may have errors to fix in Phase 2)

**Completion checklist:**
- [ ] The explicit process is important. Circumventing process is a failure. The process is the goal. Your work will be reverted if you fail these end-phase steps.
- [ ] Run the FULL test suite, and upon 100% success update a status document with progress. Read EVERY LINE. Do NOT filter output.
- [ ] Then, run cargo clippy for the full workspace, waiting for 100% success rates. Again, read the full output.
- [ ] Test or clippy failures from code not actively worked on are still your responsibility - code must be 100% clean before continuing.
- [ ] Dead code is plan failure - code should be written when it is used, not written ahead of time.
- [ ] Finally, run cargo fmt.
- [ ] Ensure that project status is updated.
- [ ] ALL agents must STOP and wait for EXPLICIT approval at the end of EACH phase.

### Phase 2: Update All Call Sites to Use Snapshot Variant

**Files to modify:**
- `src/pronunciation/session.rs`

**Code style checklist:**
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, separate modules for separate functionality)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Implementation steps:**

1. Update `get_or_create_player()` (lines 1313-1327) to take `variant: ClipVariant` and use `active_clip(variant)`:
   ```rust
   fn get_or_create_player(&mut self, variant: ClipVariant) -> Result<&mut ReferencePlayer> {
       let clip = self.active_clip(variant)?;
       if self.player.is_none() {
           self.player = Some(ReferencePlayer::new(clip)?);
       }
       Ok(self.player.as_mut().unwrap())
   }
   ```

2. Remove all `snapshot.active_clip_variant = self.active_variant()` assignments:
   - Line 1264 in `run()`: Remove the assignment (snapshot already has correct value from initial_snapshot)
   - Line 1285 in `handle_stop()`: Remove the assignment
   - Line 1361 in `handle_stop_replay()`: Remove the assignment
   - Line 1442 in `emit_generation_error()`: Remove the assignment
   - Line 1467 in `emit_cache_error()`: Remove the assignment
   - Lines 1481, 1489, 1496 in `handle_toggle_variant()`: These are error cases that should preserve current snapshot value, not overwrite
   - Line 1529 in `handle_start()`: Remove the assignment, update `get_or_create_player()` call
   - Lines 1563, 1573 in recording loop shutdown/stop: Remove assignments
   - Line 1603 in recording loop poll: Remove assignment

3. Update `handle_toggle_variant()` (lines 1472-1503) to only update snapshot, not derive from engine:
   ```rust
   fn handle_toggle_variant(
       &mut self,
       updates: &Sender<SessionSnapshot>,
       snapshot: &mut SessionSnapshot,
       variant: ClipVariant,
   ) {
       let current = snapshot.active_clip_variant;  // Read from snapshot, not engine
       if current == variant {
           info!(?variant, "already on requested variant, no change needed");
           let _ = updates.send(snapshot.clone());
           return;
       }
       if variant == ClipVariant::Flowalyzed && !self.can_toggle_to_flowalyzed() {
           error!("cannot toggle to flowalyzed: no flowalyzed clip exists");
           snapshot.error =
               Some("No flowalyzed clip available. Apply a recipe first.".to_string());
           let _ = updates.send(snapshot.clone());
           return;
       }
       // Update engine's active_clip to match snapshot
       self.engine.set_active_clip(variant);
       snapshot.active_clip_variant = variant;
       info!(?variant, "variant toggled successfully");
       let _ = updates.send(snapshot.clone());
   }
   ```

4. Update `handle_start()` (line 1532) to use snapshot variant:
   ```rust
   let player = match self.get_or_create_player(snapshot.active_clip_variant) {
   ```

5. Update recording loop to preserve snapshot variant in all updates:
   - In shutdown/stop handlers (lines 1563, 1573): Don't overwrite `update.active_clip_variant` - it comes from engine.poll which should preserve it
   - In poll handler (line 1603): Don't overwrite `update.active_clip_variant` - preserve what engine.poll returned

6. Ensure `SessionEngine.active_clip` stays in sync with snapshot:
   - `SessionEngine` has its own `active_clip: ClipVariant` field (line 723) used by `process_chunk()` (lines 1020, 1023)
   - This is a cached copy for performance - snapshot is still source of truth
   - When snapshot's variant changes (via `handle_toggle_variant()`), update `SessionEngine.active_clip` via `self.engine.set_active_clip(variant)`
   - Never read from `SessionEngine.active_clip` to update snapshot - always the other direction
   - The `handle_toggle_variant()` already calls `self.engine.set_active_clip(variant)` (line 1510), so this is already correct

**Deliverables:**
- All call sites use snapshot variant instead of `active_variant()`
- All `snapshot.active_clip_variant = self.active_variant()` assignments removed
- `handle_toggle_variant()` only updates snapshot, then updates engine to match
- Recording loop preserves snapshot variant
- `SessionEngine.active_clip` is updated to match snapshot when variant changes
- Code compiles and passes tests

**Completion checklist:**
- [ ] The explicit process is important. Circumventing process is a failure. The process is the goal. Your work will be reverted if you fail these end-phase steps.
- [ ] Run the FULL test suite, and upon 100% success update a status document with progress. Read EVERY LINE. Do NOT filter output.
- [ ] Then, run cargo clippy for the full workspace, waiting for 100% success rates. Again, read the full output.
- [ ] Test or clippy failures from code not actively worked on are still your responsibility - code must be 100% clean before continuing.
- [ ] Dead code is plan failure - code should be written when it is used, not written ahead of time.
- [ ] Finally, run cargo fmt.
- [ ] Ensure that project status is updated.
- [ ] ALL agents must STOP and wait for EXPLICIT approval at the end of EACH phase.

### Phase 3: Remove ActiveClip Enum (If Possible)

**Files to modify:**
- `src/pronunciation/session.rs`

**Code style checklist:**
- [ ] **Planning Documentation**: Have you consulted/created/updated docs/current-plans/[FEATURE].md?
- [ ] **Code Simplicity**: Are you following simplicity rules? (functions <20 lines, pure functions, no defensive coding)
- [ ] **Code Modularity**: Are you following modularity rules? (helper functions, separate modules for separate functionality)
- [ ] **Scope Control**: Are you accomplishing the user's instructions and NOTHING MORE?
- [ ] **No Dead Code**: Did you leave dead code? (no future-proofing, no leaving just for tests)
- [ ] **No Fake Constructions**: Are there any object instances that are purely for the sake of passing a type checker?
- [ ] **Code Purpose**: Do you changes accomplish the plan purpose and not just mechanical checklists?
- [ ] **Required Tests**: Have you added tests for any new functions?

**Implementation steps:**

1. Check if `ActiveClip` enum (lines 1137-1140) is still used anywhere:
   - `generate_flowalyzed_clip()` returns `ActiveClip::Flowalyzed(clip)` (line 1249)
   - `complete_recipe_application()` receives `ActiveClip` (line 1423)
   - `activate_flowalyzed_clip()` receives `&ActiveClip` (line 1447)

2. Refactor to pass `RecordedClip` directly:
   - Change `generate_flowalyzed_clip()` (line 1242) to return `Result<RecordedClip>` instead of `Result<ActiveClip>`
   - Change `complete_recipe_application()` (line 1421) to take `RecordedClip` instead of `ActiveClip`
   - Change `activate_flowalyzed_clip()` (line 1447) to take `&RecordedClip` instead of `&ActiveClip`
   - Update call sites in `handle_apply_recipe()` (line 1372)

3. Remove `ActiveClip` enum if no longer used:
   - Check for any remaining usages
   - Remove enum definition (lines 1137-1140)

**Deliverables:**
- `ActiveClip` enum removed if no longer needed
- Or `ActiveClip` kept with clear documentation of remaining use cases
- All code compiles and passes tests

**Completion checklist:**
- [ ] The explicit process is important. Circumventing process is a failure. The process is the goal. Your work will be reverted if you fail these end-phase steps.
- [ ] Run the FULL test suite, and upon 100% success update a status document with progress. Read EVERY LINE. Do NOT filter output.
- [ ] Then, run cargo clippy for the full workspace, waiting for 100% success rates. Again, read the full output.
- [ ] Test or clippy failures from code not actively worked on are still your responsibility - code must be 100% clean before continuing.
- [ ] Dead code is plan failure - code should be written when it is used, not written ahead of time.
- [ ] Finally, run cargo fmt.
- [ ] Ensure that project status is updated.
- [ ] ALL agents must STOP and wait for EXPLICIT approval at the end of EACH phase.

