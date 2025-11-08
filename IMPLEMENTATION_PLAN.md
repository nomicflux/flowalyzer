# Flowalyzer Implementation Plan

## Project Overview

Flowalyzer is a Rust audio processing tool that:
- Reads audio files in various formats (MP3, OGG, FLAC, etc.)
- Chunks audio at linguistic boundaries (sentence/word) using speech recognition
- Applies operations (repeat, speed change, silence insertion) to chunks
- Reassembles processed chunks into output audio

## Architecture

Following Amplifier's "bricks & studs" modular design philosophy:
- **Pure functions at core**: All processing takes data input, returns data output, no side effects
- **Side effects at edges**: I/O operations (decode, encode) isolated from processing
- **Ruthless simplicity**: Minimal abstractions, straightforward implementations
- **Vertical slice approach**: Complete end-to-end flows before horizontal expansion

## Learning Objectives

**CRITICAL**: This project is as much about learning proper Amplifier subagent workflows as building flowalyzer itself. DO NOT take shortcuts or skip agents "for efficiency." The learning happens through proper agent usage.

### Why Use Specialized Agents?

1. **Agent expertise**: Each agent embodies specific patterns and methodologies
2. **Proper workflow**: Learn to delegate appropriately rather than doing everything directly
3. **Pattern observation**: See how experts approach problems systematically
4. **Future foundation**: Build skills for working with AI-assisted development

### What Each Agent Teaches

- **bug-hunter**: Systematic debugging with hypothesis-driven investigation
- **modular-builder**: Creating isolated, self-contained modules following "bricks & studs" philosophy
- **integration-specialist**: Wiring modules together while maintaining clean boundaries
- **test-coverage**: Comprehensive testing strategies without over-testing

### Success Metrics

You've succeeded when you can:
- Invoke the right agent for each task type
- Provide sufficient context for agents to work effectively
- Recognize patterns in how each agent approaches problems
- Apply these patterns to future projects

## Current Status

### Working Directory
- Main: `/Users/demouser/Code/flowalyzer/`
- Amplifier symlink working directory: `/Users/demouser/Code/pyenv/amplifier/ai_working/flowalyzer/`

### Completed (8 tasks)
1. ✅ Cargo.toml configured with dependencies:
   - symphonia (audio decoding - all formats)
   - hound (WAV encoding)
   - dasp (DSP primitives)
   - tdpsola (time stretching)
   - clap (CLI)
   - anyhow (error handling)
   - whisper-rs commented out (requires cmake)

2. ✅ Core types defined in `src/types.rs`:
   - `AudioData`: samples (Vec<f32>), sample_rate (u32)
   - `Transcript`: segments (Vec<Segment>)
   - `Segment`: text, start_time, end_time, granularity
   - `Granularity`: Word | Sentence
   - `ChunkBoundary`: start_time, end_time, source_segment_ids
   - `ChunkConfig`: target_duration, min_duration, max_duration
   - `AudioChunk`: samples, sample_rate, start_time, end_time, metadata
   - `Operation`: Repeat(u32) | Speed(f32) | InsertSilence(f64) | Identity
   - `ChunkMetadata`: Optional metadata for chunks

3. ✅ Audio decoder (`src/audio/decoder.rs`):
   - Uses symphonia for multi-format support
   - Converts all sample formats to mono f32 [-1.0, 1.0]
   - Handles: S8, S16, S24, S32, U8, U16, U24, U32, F32, F64
   - Fixed i24/u24 conversion using `.inner()` method

4. ✅ WAV encoder (`src/audio/encoder.rs`):
   - Uses hound (switched from FLAC due to system lib issues)
   - Writes 16-bit mono WAV files
   - Clamps samples to [-1.0, 1.0]

5. ✅ Chunking strategy (`src/chunking/mod.rs`):
   - Pure function: `calculate_chunk_boundaries(transcript, config) -> Vec<ChunkBoundary>`
   - Greedy algorithm combining segments to hit target duration
   - Respects linguistic boundaries (sentence/word)
   - Handles long segments by splitting at target intervals
   - Fixed type inference by adding explicit `: usize` annotations

6. ✅ Audio slicer (`src/audio/slicer.rs`):
   - Pure function: `slice_audio(audio, boundaries) -> Vec<AudioChunk>`
   - Extracts audio samples based on time boundaries
   - Clamps indices to valid ranges

7. ✅ Audio assembler (`src/audio/assembler.rs`):
   - Pure function: `assemble_audio(chunks) -> Option<AudioData>`
   - Concatenates chunks with 10ms crossfade to prevent clicks
   - Verifies matching sample rates
   - **HAS FAILING TEST** - see next section

8. ✅ Build succeeds with warnings (unused imports, dead code - normal for incomplete project)

### Current Blocker - MUST FIX FIRST

**Failing Test: `src/audio/assembler.rs::test_basic_assembly`**

#### Problem Analysis
- **Location**: Line 90 in `src/audio/assembler.rs`
- **Assertion**: `assert!(audio.samples.len() > 100)`
- **Expected**: >100 samples (less than 200 due to crossfade overlap)
- **Actual**: ≤100 samples (indicates crossfade is consuming samples instead of overlapping)
- **Root cause hypothesis**: Crossfade logic bug in lines 29-46 of `assemble_audio` function
- **Context**: Two 100-sample chunks with 10ms crossfade should result in ~190-195 samples total

#### How to Fix: Use bug-hunter Agent

**Invocation command:**
```
Use the bug-hunter subagent to debug and fix the failing test in src/audio/assembler.rs
```

**Context to provide:**
- Test failure: `test_basic_assembly` at line 90
- Expected behavior: Two 100-sample chunks → >100 samples total (with crossfade overlap)
- Actual behavior: Result is ≤100 samples
- Suspect code: Lines 29-46 (crossfade logic in `assemble_audio`)
- Hypothesis: Crossfade might be replacing samples instead of overlapping them

**Expected agent workflow:**
1. Run the test to confirm failure
2. Read the assembler.rs code
3. Analyze the crossfade algorithm (lines 29-46)
4. Form hypotheses about the bug
5. Add debug output or create minimal test case
6. Identify exact issue (likely: not adding remaining samples after crossfade)
7. Fix the bug
8. Verify test passes

**Learning focus:**
- Observe how bug-hunter systematically forms and tests hypotheses
- Note the use of minimal reproductions vs wild guessing
- See how it validates the fix before claiming success

**Success criteria:**
- `cargo test` shows all 7 tests passing (6 already pass + this one)
- No new test failures introduced
- Fix is minimal and doesn't over-complicate the code

### Pending Tasks (9 tasks)

#### Phase 3: Operations Module

**Purpose**: Build three isolated, self-contained operation modules. Each is a pure function "brick" with a clear contract "stud". This phase teaches modular-builder patterns through repetition.

---

**9. Implement repeat operation** (`src/operations/repeat.rs`)

**Invocation command:**
```
Use the modular-builder subagent to create the repeat operation module at src/operations/repeat.rs
```

**Context to provide:**
- **Module purpose**: Repeat an audio chunk N times
- **Contract**: `pub fn repeat_chunk(chunk: &AudioChunk, count: u32) -> Vec<AudioChunk>`
- **Implementation**: Pure function returning N identical copies of the chunk
- **Edge cases**: count=0 (return empty vec), count=1 (return single chunk)
- **Tests needed**: count=0, count=1, count=3 (verify all chunks identical)
- **Dependencies**: Only `crate::types::AudioChunk`

**Expected agent workflow:**
1. Create `src/operations/` directory if needed
2. Create `repeat.rs` with module doc comment explaining purpose
3. Implement pure `repeat_chunk` function
4. Add comprehensive tests covering edge cases
5. Verify tests pass with `cargo test`

**Learning focus:**
- See how modular-builder creates self-contained modules
- Observe "brick & stud" pattern: clear contract, isolated implementation
- Note the emphasis on pure functions (no side effects)
- Watch test-first or test-alongside development

**Success criteria:**
- File `src/operations/repeat.rs` exists
- Function signature matches contract exactly
- All tests pass
- Module is fully self-contained (no external dependencies beyond types)

---

**10. Implement silence insertion** (`src/operations/silence.rs`)

**Invocation command:**
```
Use the modular-builder subagent to create the silence insertion module at src/operations/silence.rs
```

**Context to provide:**
- **Module purpose**: Generate silent audio chunks
- **Contract**: `pub fn insert_silence(duration: f64, sample_rate: u32) -> AudioChunk`
- **Implementation**: Create AudioChunk with Vec of zeros, length = duration * sample_rate
- **Edge cases**: duration=0.0 (empty samples), very short durations
- **Tests needed**: 0.0s, 0.1s, 1.0s durations; verify all samples are 0.0
- **Dependencies**: Only `crate::types::AudioChunk`

**Expected agent workflow:**
1. Create `silence.rs` in `src/operations/`
2. Implement pure `insert_silence` function
3. Calculate sample count correctly: `(duration * sample_rate as f64) as usize`
4. Set appropriate time bounds (start_time=0.0, end_time=duration)
5. Add tests with various durations
6. Verify all tests pass

**Learning focus:**
- Second iteration of modular-builder pattern (reinforcement)
- Compare to repeat.rs: same pattern, different logic
- Note consistent module structure across operations

**Success criteria:**
- File `src/operations/silence.rs` exists
- Function creates correct number of zero samples
- All tests pass
- Consistent style with repeat.rs

---

**11. Implement speed change** (`src/operations/speed.rs`)

**Invocation command:**
```
Use the modular-builder subagent to create the speed change module at src/operations/speed.rs
```

**Context to provide:**
- **Module purpose**: Time-stretch audio without pitch change
- **Contract**: `pub fn change_speed(chunk: &AudioChunk, speed_factor: f32) -> AudioChunk`
- **Implementation**: Use `tdpsola` crate for time-domain pitch-synchronous overlap-add
- **Dependency**: Add `use tdpsola::tdpsola;` (already in Cargo.toml)
- **Edge cases**: speed_factor=1.0 (no change), speed_factor near 0, very large factors
- **Tests needed**: speed_factor = 0.5 (slower), 1.0 (identity), 2.0 (faster)
- **Research**: Agent may need to check tdpsola crate docs for API usage
- **Dependencies**: `crate::types::AudioChunk`, `tdpsola`

**Expected agent workflow:**
1. Research tdpsola API (likely needs sample_rate and speed_factor params)
2. Create `speed.rs` in `src/operations/`
3. Implement using `tdpsola::tdpsola()` function
4. Handle sample_rate correctly for time calculations
5. Add tests with multiple speed factors
6. Verify output sample counts match expected time-stretch ratios

**Learning focus:**
- Third iteration: modular-builder with external library integration
- Compare to repeat.rs and silence.rs: same pattern, more complex dependency
- Observe how agent researches unfamiliar APIs
- Note how external libraries integrate into pure function architecture

**Success criteria:**
- File `src/operations/speed.rs` exists
- Correct integration with tdpsola crate
- Output AudioChunk has correct sample count for speed_factor
- All tests pass
- Module remains self-contained despite external dependency

---

**12. Wire operations module** (`src/operations/mod.rs`)

**Invocation command:**
```
Use the integration-specialist subagent to create the operations module wiring at src/operations/mod.rs
```

**Context to provide:**
- **Module purpose**: Wire together the three operation modules into a unified interface
- **Contract**: `pub fn apply_operation(chunk: &AudioChunk, op: &Operation) -> Vec<AudioChunk>`
- **Sub-modules to integrate**: `repeat`, `silence`, `speed` (created in tasks 9-11)
- **Operation enum variants** (from `types.rs`):
  - `Operation::Identity` → return vec![chunk.clone()]
  - `Operation::Repeat(n)` → call `repeat::repeat_chunk(chunk, n)`
  - `Operation::Speed(factor)` → call `speed::change_speed(chunk, factor)`, wrap in vec
  - `Operation::InsertSilence(duration)` → call `silence::insert_silence(duration, chunk.sample_rate)`, wrap in vec
- **Exports needed**: Re-export all three operation functions as public
- **Pattern**: Simple dispatcher based on enum matching

**Expected agent workflow:**
1. Create `src/operations/mod.rs`
2. Add module declarations: `mod repeat;`, `mod silence;`, `mod speed;`
3. Re-export operation functions: `pub use repeat::repeat_chunk;`, etc.
4. Implement `apply_operation` dispatcher with match on `op`
5. Add basic tests that verify correct dispatch
6. Verify `cargo test` passes

**Learning focus:**
- See how integration-specialist wires isolated modules together
- Observe clean boundaries: mod.rs doesn't implement logic, just connects
- Note the "stud" pattern: apply_operation is the connection point for other code
- Compare to modular-builder: different responsibility (wiring vs building)

**Success criteria:**
- File `src/operations/mod.rs` exists
- All three sub-modules properly declared and exported
- `apply_operation` correctly dispatches to each operation
- Tests verify all Operation enum variants work
- Module maintains clean separation: no business logic in mod.rs

---

**13. Test Phase 3: Operations integration tests**

**Invocation command:**
```
Use the test-coverage subagent to create comprehensive integration tests for the operations module
```

**Context to provide:**
- **Purpose**: Verify operations module integration and dispatcher work correctly
- **Test location**: Add to `src/operations/mod.rs` or create `src/operations/tests.rs`
- **What to test**:
  - Each Operation enum variant dispatches correctly
  - Operation::Identity returns unchanged chunk
  - Operation::Repeat creates correct number of copies
  - Operation::Speed produces time-stretched output
  - Operation::InsertSilence generates silent chunk
- **Test strategy**: Create test AudioChunks with known properties, apply operations, verify results
- **Edge cases**: Empty chunks, extreme parameters, chaining operations

**Expected agent workflow:**
1. Analyze operations module structure
2. Identify critical paths requiring tests
3. Create comprehensive test suite
4. Test each operation variant
5. Add edge case tests
6. Verify all tests pass with `cargo test`

**Learning focus:**
- See test-coverage agent's systematic testing approach
- Observe comprehensive vs over-testing balance
- Note focus on behavior verification, not implementation details
- Watch identification of critical test cases

**Success criteria:**
- All Operation enum variants have tests
- Integration between mod.rs dispatcher and sub-modules verified
- Edge cases covered appropriately
- All tests pass
- Test names clearly describe what they verify

#### Phase 4: CLI & Pipeline

**Purpose**: Build the complete end-to-end application. Integration-specialist wires all the "bricks" together through their "studs" (contracts) into a working CLI tool.

---

**14. Implement CLI argument parsing** (`main.rs`)

**Invocation command:**
```
Use the integration-specialist subagent to implement the CLI argument parsing in main.rs
```

**Context to provide:**
- **Purpose**: Parse command-line arguments for the flowalyzer tool
- **Framework**: Use `clap` with derive macros (already in Cargo.toml)
- **Required arguments**:
  - `input_file: PathBuf` - Input audio file path
  - `output_file: PathBuf` - Output audio file path
- **Optional arguments**:
  - `--target-duration <SECONDS>` - Target chunk duration (default: 2.0)
  - `--operations <SPEC>` - Operations to apply (format: "repeat:2,speed:1.5" or similar)
- **Validation**: Check input file exists and is readable
- **Help text**: Add descriptive help for each argument

**Expected agent workflow:**
1. Read existing `main.rs` (currently placeholder)
2. Add clap imports and derive macros
3. Create `Args` struct with derive(Parser)
4. Implement validation in Args (e.g., custom validator for file existence)
5. Add minimal tests or examples showing CLI usage
6. Verify `cargo build` succeeds

**Learning focus:**
- See integration-specialist working with external framework (clap)
- Observe input validation at system boundaries
- Note clean separation: CLI parsing separate from business logic

**Success criteria:**
- `Args` struct properly defined with clap macros
- `--help` output is clear and helpful
- Input file validation works
- Code compiles successfully

---

**15. Wire full audio processing pipeline** (`main.rs`)

**Invocation command:**
```
Use the integration-specialist subagent to implement the complete audio processing pipeline in main.rs
```

**Context to provide:**
- **Purpose**: Connect all modules into end-to-end workflow
- **Available modules** (all implemented in previous tasks):
  - `audio::decoder::decode_audio(path)` → AudioData
  - `chunking::calculate_chunk_boundaries(transcript, config)` → Vec<ChunkBoundary>
  - `audio::slicer::slice_audio(audio, boundaries)` → Vec<AudioChunk>
  - `operations::apply_operation(chunk, op)` → Vec<AudioChunk>
  - `audio::assembler::assemble_audio(chunks)` → Option<AudioData>
  - `audio::encoder::encode_audio(audio, path)` → Result
- **Pipeline flow** (vertical slice):
  1. Parse CLI args (from task 14)
  2. Decode input audio file
  3. Create time-based chunks (MVP: simple time intervals, skip whisper transcription)
  4. Slice audio into chunks
  5. Apply operations to each chunk based on CLI args
  6. Assemble processed chunks
  7. Encode to output file
- **Error handling**: Use `anyhow::Result` throughout, propagate with `.context()`
- **Main signature**: `fn main() -> anyhow::Result<()>`
- **Chunking MVP**: For now, create boundaries at regular time intervals (target_duration) without whisper

**Expected agent workflow:**
1. Implement `fn main() -> anyhow::Result<()>`
2. Parse args with clap
3. Call each module function in sequence
4. Add error context at each step (e.g., `.context("Failed to decode audio")`)
5. Handle intermediate data types correctly
6. Test end-to-end with a sample audio file
7. Verify complete pipeline works

**Learning focus:**
- See integration-specialist connecting multiple modules
- Observe error handling across module boundaries
- Note how "studs" (contracts) make integration straightforward
- Watch vertical slice approach: end-to-end before horizontal expansion
- Compare to task 12: similar wiring pattern, but at application level

**Success criteria:**
- Complete pipeline from input file to output file works
- Error messages are clear and actionable
- All module integrations work correctly
- Can run: `cargo run -- input.wav output.wav --target-duration 2.0`
- Output file is created and playable

---

**16. Test Phase 4: End-to-end integration tests**

**Invocation command:**
```
Use the test-coverage subagent to create end-to-end integration tests for the complete flowalyzer pipeline
```

**Context to provide:**
- **Purpose**: Verify complete application works from CLI to output file
- **Test approach**: Shell script or Rust integration tests
- **Test setup**:
  - Generate test WAV file (use `ffmpeg` or `sox` if available, or create simple WAV in Rust)
  - Build the binary: `cargo build --release`
- **Test cases**:
  - Basic run: input → output with default settings
  - Custom duration: `--target-duration 1.0`
  - With operations: `--operations "repeat:2"`
  - Error cases: non-existent input file, invalid arguments
- **Validation**:
  - Output file exists
  - Output file is valid audio (can decode it back)
  - Output has expected duration/properties

**Expected agent workflow:**
1. Determine best test approach (Bash script vs Rust tests)
2. Create test fixture (sample audio file)
3. Write tests that invoke the binary
4. Verify output file properties
5. Add error case testing
6. Document how to run integration tests

**Learning focus:**
- See test-coverage approach to end-to-end testing
- Observe fixture management strategies
- Note validation beyond "it doesn't crash"
- Compare to unit tests: different concerns, different verification

**Success criteria:**
- Integration test suite can be run with single command
- Tests verify actual file I/O and complete pipeline
- Error cases properly tested
- Tests are repeatable and clean up after themselves
- Documentation explains how to run tests

17. **Update CLAUDE.md**
    - **Tool**: Edit (direct)
    - **Sections to add**:
      - Build: `cargo build --release`
      - Test: `cargo test`
      - Run: `cargo run -- input.wav output.wav --target-duration 2.0 --operations "repeat:2"`
      - Note: Whisper transcription postponed (requires cmake)
      - Current architecture state

## File Structure

```
flowalyzer/
├── Cargo.toml
├── CLAUDE.md (needs update)
├── IMPLEMENTATION_PLAN.md (this file)
└── src/
    ├── main.rs (placeholder - needs CLI + pipeline)
    ├── types.rs (complete)
    ├── audio/
    │   ├── mod.rs (exports)
    │   ├── decoder.rs (complete)
    │   ├── encoder.rs (complete)
    │   ├── slicer.rs (complete)
    │   └── assembler.rs (complete but has failing test)
    ├── chunking/
    │   └── mod.rs (complete)
    └── operations/ (DOES NOT EXIST YET)
        ├── mod.rs (pending)
        ├── repeat.rs (pending)
        ├── silence.rs (pending)
        └── speed.rs (pending)
```

## How to Use Amplifier Agents

### Available Agents (symlinked from `/Users/demouser/Code/pyenv/amplifier/.claude/agents/`)

This project uses specialized Amplifier agents for different types of work:

- **bug-hunter**: Systematic debugging with hypothesis-driven investigation
- **modular-builder**: Build self-contained modules following "bricks & studs" philosophy
- **integration-specialist**: Wire modules together, implement pipelines and CLIs
- **test-coverage**: Create comprehensive test suites with strategic coverage
- **zen-architect**: High-level architectural design (already used for initial architecture)

### How to Invoke Custom Amplifier Agents

**CRITICAL DISTINCTION**:
- **Built-in agents** (general-purpose, Explore, bug-hunter, etc.): Use the `Task` tool with `subagent_type` parameter
- **Custom Amplifier agents**: Explicitly mention them in your request

**Correct invocation method for custom agents:**

```
Use the [agent-name] subagent to [detailed task description with context]
```

**Examples:**

```
Use the bug-hunter subagent to debug and fix the failing test in src/audio/assembler.rs
```

```
Use the modular-builder subagent to create the repeat operation module at src/operations/repeat.rs
```

```
Use the integration-specialist subagent to implement the CLI argument parsing in main.rs
```

```
Use the test-coverage subagent to create comprehensive integration tests for the operations module
```

### Providing Context to Agents

Agents work best with comprehensive context. Don't just say "fix the test" or "implement this module". Provide:

1. **Purpose**: What the module/fix accomplishes
2. **Contract**: Function signatures, inputs, outputs
3. **Constraints**: Edge cases, dependencies, requirements
4. **Success criteria**: How to know it's complete

**Example of good context:**

```
Use the modular-builder subagent to create the silence insertion module at src/operations/silence.rs

Context:
- Module purpose: Generate silent audio chunks
- Contract: pub fn insert_silence(duration: f64, sample_rate: u32) -> AudioChunk
- Implementation: Create AudioChunk with Vec of zeros, length = duration * sample_rate
- Edge cases: duration=0.0 (empty samples), very short durations
- Tests needed: 0.0s, 0.1s, 1.0s durations; verify all samples are 0.0
- Dependencies: Only crate::types::AudioChunk
```

This level of detail enables the agent to work effectively without back-and-forth clarification.

## Test Status

Run `cargo test` to verify:
- **Expected**: 7 tests
- **Current**: 6 passed, 1 failed
- **Failing test**: `audio::assembler::tests::test_basic_assembly`

## Key Decisions

1. **Output format**: WAV (via hound) instead of FLAC - simpler, no system dependencies
2. **Input formats**: Flexible via symphonia (MP3, OGG, FLAC, AAC, etc.)
3. **MVP transcription**: Time-based chunking instead of whisper (requires cmake installation)
4. **Error handling**: anyhow crate for ergonomic error propagation
5. **Design philosophy**: Amplifier's ruthless simplicity and "bricks & studs" modularity

## Agent Learning Path

This project follows a deliberate progression to teach different agent patterns:

### Phase 1: Systematic Debugging (Task 8)
**Agent**: bug-hunter

**What you'll learn**:
- Hypothesis-driven debugging methodology
- Minimal reproductions vs code exploration
- How to validate fixes properly
- When to add debug output vs reading code

**Pattern to observe**: The agent doesn't randomly change code. It forms hypotheses, tests them, and fixes the root cause.

### Phase 2: Isolated Module Building (Tasks 9-11)
**Agent**: modular-builder (3x repetition)

**What you'll learn**:
- "Bricks & studs" modular design philosophy
- Pure function architecture
- Self-contained module creation
- Contract-first development
- Test-alongside-implementation approach

**Pattern to observe**: Each module is built in isolation with clear boundaries. The agent creates modules that can be understood and tested independently.

**Why 3 iterations?**: Repetition reinforces the pattern. Notice similarities across repeat.rs, silence.rs, and speed.rs despite different logic.

### Phase 3: Module Integration (Tasks 12, 14-15)
**Agent**: integration-specialist (3x usage)

**What you'll learn**:
- Wiring isolated modules together
- Maintaining clean boundaries during integration
- Dispatcher pattern (task 12)
- CLI integration with external frameworks (task 14)
- End-to-end pipeline construction (task 15)

**Pattern to observe**: Integration code doesn't implement business logic. It connects modules through their contracts ("studs"). Compare task 12 (internal wiring) with task 15 (application-level wiring).

### Phase 4: Comprehensive Testing (Tasks 13, 16)
**Agent**: test-coverage (2x usage)

**What you'll learn**:
- Strategic test case identification
- Integration vs unit testing
- Comprehensive coverage without over-testing
- Fixture management for end-to-end tests

**Pattern to observe**: The agent focuses on behavior verification, not implementation details. Tests verify contracts, not internals.

## Anti-Patterns to Avoid

**CRITICAL**: These are common mistakes that defeat the learning purpose of this project.

### ❌ Anti-Pattern 1: Skipping Agents "For Efficiency"
```
Bad: "This is simple, I'll just implement repeat.rs directly without modular-builder"
Why it's wrong: You miss learning how modular-builder approaches isolation and contracts
```

### ❌ Anti-Pattern 2: Providing Insufficient Context
```
Bad: "Use modular-builder to create repeat.rs"
Why it's wrong: Agent lacks necessary context about contracts, edge cases, testing requirements
Good: Provide full context from the task description (contract, edge cases, tests, etc.)
```

### ❌ Anti-Pattern 3: Using Agents as Simple Command Executors
```
Bad: "Use bug-hunter to change line 42 to X"
Why it's wrong: You're dictating the solution instead of letting the agent demonstrate its methodology
Good: Describe the problem, let the agent investigate and fix
```

### ❌ Anti-Pattern 4: Doing Work Directly That Agents Should Handle
```
Bad: Reading assembler.rs, identifying the bug, then asking bug-hunter to apply your fix
Why it's wrong: You rob yourself of seeing the agent's systematic debugging approach
Good: Provide problem description and let bug-hunter investigate
```

### ❌ Anti-Pattern 5: Not Observing Agent Patterns
```
Bad: Run the agent, check if code works, move on without reading agent's approach
Why it's wrong: You miss the learning opportunity
Good: Read the agent's messages, understand its methodology, identify patterns to reuse
```

### ✅ Success Pattern: Proper Agent Usage
```
1. Select the right agent for the task
2. Provide comprehensive context from the task description
3. Let the agent work using its methodology
4. Observe and learn from the agent's approach
5. Identify patterns you can apply to future work
```

## Next Steps for New AI

1. **First priority**: Use bug-hunter agent to fix the assembler test failure
2. **Then**: Use modular-builder agent to implement the 3 operation modules (repeat, silence, speed)
3. **Then**: Use integration-specialist to wire operations module (task 12)
4. **Then**: Use test-coverage for operations integration tests (task 13)
5. **Then**: Use integration-specialist for CLI and pipeline (tasks 14-15)
6. **Then**: Use test-coverage for end-to-end tests (task 16)
7. **Finally**: Update CLAUDE.md with commands (direct edit)

## Build & Test Commands

```bash
# Build
cargo build

# Run tests
cargo test

# Build release
cargo build --release

# Run (once CLI implemented)
cargo run -- input.wav output.wav --target-duration 2.0
```

## Important Notes

- Code compiles with warnings (unused imports) - this is normal for incomplete project
- Whisper integration postponed - requires cmake installation
- All processing functions are pure (data in, data out, no side effects)
- I/O isolated to decoder/encoder at edges
- Follow vertical slice: get end-to-end working before adding features
