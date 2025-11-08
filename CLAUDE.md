# CLAUDE.md - Flowalyzer Audio Processing Tool

This file provides guidance to Claude Code when working with this repository.

## ⚠️ CRITICAL: Read IMPLEMENTATION_PLAN.md FIRST

**Before doing ANYTHING, read `/Users/demouser/Code/flowalyzer/IMPLEMENTATION_PLAN.md`**

That document contains:
- Complete project status and history
- Detailed pending tasks with specific agents to use
- Current blocker (failing test that MUST be fixed first)
- Full architecture explanation
- How to use Amplifier agents (DON'T ASSUME YOU KNOW HOW)

## Project Overview

**Flowalyzer** is a Rust audio processing tool that chunks audio files at linguistic boundaries and applies operations (repeat, speed change, silence insertion) before reassembling.

**Current State**: ✅ **MVP COMPLETE** - All modules implemented, CLI functional, 26 tests passing. Ready for end-to-end testing and production use.

## Critical Instructions to Avoid Previous Failures

### 1. DO NOT ASSUME YOU KNOW HOW TO USE AMPLIFIER AGENTS

Previous Claude instances wasted 30+ minutes trying wrong methods to invoke Amplifier agents. **ASK THE USER** for the correct invocation method if you're unsure. Do not try:
- Task tool with subagent_type (this doesn't work for custom agents)
- Just mentioning agent name in text (user said this is wrong too)
- Random guessing

### 2. READ WHAT YOU'RE TOLD

When the user gives you information:
- **Actually process it**, don't just retrieve and ignore
- **Remember it** for the rest of the session
- **Connect it** to similar information you've seen before
- If you read documentation, APPLY what it says

### 3. PRESERVE EXISTING WORK

When asked to "re-plan" or "update plan":
- **DON'T** create a new plan from scratch
- **DO** annotate/update the existing plan
- The existing plan has more detail - don't destroy it

### 4. THINK ABOUT TOOL SELECTION

When selecting which agent to use for a task:
- **Don't** default to "general-purpose" for everything
- **Do** analyze which specialized agent fits the task
- bug-hunter for debugging
- modular-builder for creating isolated modules
- integration-specialist for wiring things together
- test-coverage for comprehensive testing

### 5. BUILD ON YOUR OWN WORK

If you did something successfully earlier in the conversation:
- **Remember** how you did it
- **Reuse** that approach for similar tasks
- **Don't** forget your own successful patterns

### 6. USE AGENTS AS DEFAULT, NOT OPTIMIZATION

Amplifier philosophy: Use specialized agents for EVERYTHING by default. Direct tool use is the exception, not the rule. Multi-step work should always go to an agent.

## Project Architecture

**Design Philosophy**: Amplifier's "bricks & studs" modular design
- Pure functions at core (data in, data out, no side effects)
- Side effects only at edges (I/O operations)
- Ruthless simplicity (minimal abstractions)
- Vertical slice approach (end-to-end first, then expand)

### Completed Modules

1. **types.rs** - Core data structures (AudioData, Transcript, Segment, ChunkBoundary, Operation, etc.)
2. **audio/decoder.rs** - Multi-format audio decoding via symphonia, converts to mono f32
3. **audio/encoder.rs** - WAV encoding via hound (16-bit mono)
4. **audio/slicer.rs** - Pure function to extract audio samples by time boundaries
5. **audio/assembler.rs** - Concatenates chunks with crossfade (✅ FIXED - all tests passing)
6. **chunking/mod.rs** - Pure function to calculate chunk boundaries from transcript
7. **operations/repeat.rs** - ✅ Repeat chunk N times (pure function)
8. **operations/silence.rs** - ✅ Generate silence of specified duration (pure function)
9. **operations/speed.rs** - ✅ Time-stretch using ssstretch (Signalsmith Stretch) - pitch-preserving
10. **operations/mod.rs** - ✅ Dispatcher and operation wiring complete
11. **main.rs** - ✅ CLI and full pipeline integration complete

### Architecture Decisions

**Speed Operation Implementation:**
- **Chose ssstretch over tdpsola** - tdpsola requires pitch detection (unsuitable for arbitrary audio)
- ssstretch works on all audio types (music, speech, noise) without pitch knowledge
- Requires C++14 compiler (available on macOS via XCode command line tools)
- API: `Stretch::new()` + `preset_default()` + `process_vec()`
- ~20 lines of code vs ~60+ lines with tdpsola + pitch detection

**MVP Chunking Strategy:**
- Time-based regular intervals (deferred transcription/linguistic boundaries)
- whisper-rs commented out in Cargo.toml (requires cmake - deferred to v2)
- Can be upgraded to linguistic boundaries later without breaking API

## Build and Development Commands

### Building
```bash
cargo build          # Build the project
cargo build --release # Build with optimizations
cargo check          # Fast compile check
```

### Testing
```bash
cargo test           # Run all tests (26 passing)
cargo test -- --nocapture # See test output
cargo test operations::speed # Run specific module tests
```

### Linting
```bash
cargo fmt            # Format code
cargo clippy         # Linter checks
```

## Amplifier Integration

This project uses Amplifier's specialized agents and tools via symlink:
- `.claude/agents/` → `/Users/demouser/Code/pyenv/amplifier/.claude/agents/`

### Amplifier Documentation Locations

**READ THESE BEFORE ATTEMPTING TO USE AMPLIFIER TOOLS:**

#### Core Amplifier Docs
- **Main guide**: `/Users/demouser/Code/pyenv/amplifier/AGENTS.md`
  - Sub-agent optimization strategy
  - Incremental processing patterns
  - Decision tracking system
  - Configuration management
  - Zero-BS principle (no placeholders/stubs)
  - Implementation philosophy (ruthless simplicity)
  - Modular design philosophy ("bricks & studs")

#### Claude Code Integration Docs
- **Subagents reference**: `/Users/demouser/Code/pyenv/amplifier/ai_context/claude_code/CLAUDE_CODE_SUBAGENTS.md`
  - How subagents work in Claude Code
  - Configuration format
  - File locations and priority
  - Model selection
  - Tool permissions
- **Slash commands**: `/Users/demouser/Code/pyenv/amplifier/ai_context/claude_code/CLAUDE_CODE_SLASH_COMMANDS.md`
  - Built-in commands
  - Custom command creation
  - MCP slash commands
  - SlashCommand tool
- **Common workflows**: `/Users/demouser/Code/pyenv/amplifier/ai_context/claude_code/CLAUDE_CODE_COMMON_WORKFLOWS.md`
- **Hooks reference**: `/Users/demouser/Code/pyenv/amplifier/ai_context/claude_code/CLAUDE_CODE_HOOKS.md`

#### Philosophy & Design Docs
- **Implementation philosophy**: `/Users/demouser/Code/pyenv/amplifier/ai_context/IMPLEMENTATION_PHILOSOPHY.md`
- **Modular design**: `/Users/demouser/Code/pyenv/amplifier/ai_context/MODULAR_DESIGN_PHILOSOPHY.md`
- **Amplifier vision**: `/Users/demouser/Code/pyenv/amplifier/AMPLIFIER_VISION.md`

#### Available Slash Commands
In `/Users/demouser/Code/pyenv/amplifier/.claude/commands/`:
- **/commit** - Create git commits
- **/create-plan** - Create implementation plans
- **/execute-plan** - Execute planned tasks
- **/modular-build** - Build modular components
- **/review-changes** - Review code changes
- **/review-code-at-path** - Review specific paths
- **/ultrathink-task** - Deep analysis mode

### Available Specialized Agents

In `/Users/demouser/Code/pyenv/amplifier/.claude/agents/` (23 agents):

**Development Agents:**
- **bug-hunter** - Debug failing tests and errors
- **modular-builder** - Build isolated modules with clear contracts
- **integration-specialist** - Wire modules together, implement CLIs
- **test-coverage** - Create comprehensive test suites
- **zen-architect** - High-level architectural design (already used for this project)
- **api-contract-designer** - Design API contracts
- **database-architect** - Database schema design
- **performance-optimizer** - Optimize performance
- **security-guardian** - Security analysis
- **contract-spec-author** - Write module contracts

**Knowledge & Analysis Agents:**
- **analysis-engine** - Deep analysis tasks
- **concept-extractor** - Extract concepts from content
- **content-researcher** - Research content
- **insight-synthesizer** - Synthesize insights
- **knowledge-archaeologist** - Dig through historical context
- **pattern-emergence** - Identify emerging patterns
- **visualization-architect** - Create visualizations

**Meta & Support Agents:**
- **ambiguity-guardian** - Identify and resolve ambiguities
- **amplifier-cli-architect** - Design Amplifier CLI tools
- **graph-builder** - Build knowledge graphs
- **module-intent-architect** - Design module intent
- **post-task-cleanup** - Clean up after tasks
- **subagent-architect** - Create new specialized agents

### How to Invoke Agents

**STILL UNKNOWN** - Multiple Claude instances have failed to invoke Amplifier agents correctly.

#### Failed Attempts Log (Session 2025-10-17):

**Attempt 1:** `Task(subagent_type="bug-hunter", description="...", prompt="detailed multi-paragraph prompt")`
- **Result**: Agent invoked but did NOTHING (0 tool uses, 0 tokens)
- **Error**: None - it just ran and returned empty
- **Context**: First attempt after exiting plan mode

**Attempts 2-N:** Same Task tool syntax with various prompt lengths/styles
- **Result**: `API Error 400: "This credential is only authorized for use with Claude Code and cannot be used for other API requests"`
- **Error**: Consistent 400 Bad Request errors
- **Observation**: The error changed from "invokes but does nothing" to "400 error" - unclear why

**Attempt with natural language:** "I'll use the bug-hunter agent to..."
- **Result**: Absolutely nothing - no invocation, no error, text ignored

#### Key Observations:

1. **Task tool IS the correct mechanism** (it successfully invoked once, even if it did nothing)
2. **400 errors suggest credential/API issues**, not syntax errors
3. **bug-hunter.md has `model: opus`** - may require credentials this session doesn't have
4. **First invocation worked differently** than subsequent ones - unknown cause
5. **Natural language does NOT invoke agents** - despite what docs seemed to suggest

#### Theories to Test:

1. The `model: opus` setting requires different credentials than available
2. First invocation worked because of some initial state that was lost
3. There's a specific prompt format/structure required that we haven't found
4. The Task tool parameters need something we're not providing

#### What to Try Next:

1. Check if changing `model: opus` to `model: inherit` in bug-hunter.md helps
2. Look for actual working examples in Amplifier codebase (not just docs)
3. Try the simpler built-in agents like "Explore" to see if they work
4. Ask user for working session logs or examples

**CRITICAL**: Do not assume you know how to invoke agents. Test incrementally and verify each step actually works.

## Todo List Status

Run `/help` to see todo tracking commands. Current todos track:
- ✅ 7 completed tasks (core I/O, types, chunking, slicer, assembler)
- 🔴 1 in-progress (fix assembler test - BLOCKER)
- ⏸️ 9 pending (operations, CLI, integration tests, docs)

## Key Dependencies

- **symphonia** - Multi-format audio decoding (MP3, OGG, FLAC, AAC)
- **hound** - WAV encoding (switched from FLAC - no system deps)
- **dasp** - Digital audio signal processing
- **ssstretch** - Time-stretching via Signalsmith Stretch (requires C++14 compiler)
- **clap** - CLI argument parsing
- **anyhow** - Error handling
- **whisper-rs** - COMMENTED OUT (requires cmake, deferred to v2)

## Important Constraints

1. **No compilation errors allowed** - If code doesn't compile, you have nothing. Fix immediately.
2. **All tests must pass** - 100% pass rate required before moving forward
3. **Pure functions everywhere** - Processing logic must be side-effect free
4. **Vertical slices** - Get end-to-end working before expanding horizontally
5. **Systematic debugging** - Don't randomly change code. Analyze first, then fix.

## Development Workflow

1. Read IMPLEMENTATION_PLAN.md for current status
2. Fix the failing assembler test (current blocker)
3. Implement operations module (3 pure functions + wiring)
4. Build CLI and wire full pipeline
5. Add integration tests
6. Update this file with usage examples

## File Structure

```
flowalyzer/
├── Cargo.toml              # ✅ Dependencies configured (ssstretch, symphonia, hound, clap, anyhow)
├── CLAUDE.md               # ✅ This file - project documentation
├── IMPLEMENTATION_PLAN.md  # ✅ Detailed status and implementation history
└── src/
    ├── main.rs             # ✅ CLI and full pipeline integration
    ├── types.rs            # ✅ Core data structures
    ├── audio/
    │   ├── mod.rs          # ✅ Module exports
    │   ├── decoder.rs      # ✅ Multi-format audio decoding (symphonia)
    │   ├── encoder.rs      # ✅ WAV encoding (hound)
    │   ├── slicer.rs       # ✅ Time-based audio extraction
    │   └── assembler.rs    # ✅ Chunk assembly with crossfade
    ├── chunking/
    │   └── mod.rs          # ✅ Chunk boundary calculation
    └── operations/
        ├── mod.rs          # ✅ Operation dispatcher
        ├── repeat.rs       # ✅ Repeat operation (3 tests)
        ├── silence.rs      # ✅ Silence insertion (5 tests)
        └── speed.rs        # ✅ Time-stretch via ssstretch (5 tests)
```

## Usage Examples

### Basic Usage - Identity Processing

Process audio without modifications (decode → chunk → reassemble → encode):

```bash
cargo run -- input.mp3 output.wav
```

This will:
1. Decode `input.mp3` (supports MP3, OGG, FLAC, WAV, AAC)
2. Chunk into 2-second segments (default)
3. Reassemble with crossfade
4. Encode to `output.wav`

### Chunking Options

Specify custom chunk duration:

```bash
cargo run -- input.mp3 output.wav --target-duration 1.5
```

Creates 1.5-second chunks instead of default 2 seconds.

### Operations

#### Repeat Operation

Repeat each chunk N times:

```bash
cargo run -- input.mp3 output.wav --operations repeat:3
```

Example: 10-second input with 2-second chunks → 5 chunks × 3 repeats = 30 seconds output

#### Speed Operation

Change speed with pitch preservation:

```bash
# 50% slower (pitch preserved)
cargo run -- speech.mp3 slow.wav --operations speed:0.5

# 2x faster (pitch preserved)
cargo run -- speech.mp3 fast.wav --operations speed:2.0

# 25% slower
cargo run -- music.mp3 stretched.wav --operations speed:0.75
```

Speed factors:
- `< 1.0` = slower (longer duration)
- `1.0` = no change
- `> 1.0` = faster (shorter duration)

#### Silence Insertion

Insert silence between chunks:

```bash
# Insert 0.5 seconds of silence between each chunk
cargo run -- input.mp3 output.wav --operations silence:0.5
```

### Combined Example

```bash
# Create 1-second chunks, repeat each 2 times, then slow down by 25%
cargo run -- podcast.mp3 processed.wav \
  --target-duration 1.0 \
  --operations repeat:2

# Then apply speed separately if needed
cargo run -- processed.wav final.wav --operations speed:0.75
```

**Note**: Currently only one operation type per run. Chain multiple runs for complex workflows.

### Typical Workflows

#### Language Learning Tool

Slow down speech for comprehension practice:

```bash
# Chunk into sentences (2s), repeat 3x, slow to 60% speed
cargo run -- lesson.mp3 practice.wav \
  --target-duration 2.0 \
  --operations repeat:3

cargo run -- practice.wav practice-slow.wav --operations speed:0.6
```

#### Music Practice

Create practice loops:

```bash
# 4-second chunks (musical phrases), repeat 10x
cargo run -- song.mp3 practice-loop.wav \
  --target-duration 4.0 \
  --operations repeat:10
```

#### Audio Book Speed Adjustment

Speed up audiobooks while preserving voice quality:

```bash
cargo run -- audiobook.mp3 fast-audiobook.wav --operations speed:1.5
```

### Build and Run

```bash
# Development build (faster compilation)
cargo build
./target/debug/flowalyzer input.mp3 output.wav --operations speed:0.5

# Release build (optimized performance)
cargo build --release
./target/release/flowalyzer input.mp3 output.wav --operations speed:0.5
```

### Testing

```bash
# Run all tests (26 tests)
cargo test

# Run specific module tests
cargo test operations::speed
cargo test audio::assembler

# See test output
cargo test -- --nocapture
```

### Troubleshooting

**Error: "Input file does not exist"**
- Check file path is correct
- Use absolute path if relative path fails

**Error: "Failed to decode input audio"**
- Verify file is a valid audio file
- Supported formats: MP3, OGG, FLAC, WAV, AAC, M4A

**Warning: "Output file should have .wav extension"**
- Output format is always WAV (16-bit mono)
- Rename output file to end with `.wav`

**Build error with ssstretch**
- Requires C++14 compiler
- macOS: Install XCode command line tools (`xcode-select --install`)
- Linux: Install clang (`apt install clang` or `dnf install clang`)

## Error History (Learn From These)

Previous Claude instances made these mistakes:
1. Forgot how to invoke agents despite doing it successfully earlier
2. Created new plans instead of updating existing ones when asked to "re-plan"
3. Selected "general-purpose" for all tasks instead of thinking about specialized agents
4. Ignored documentation they had just read
5. Tried random fixes instead of systematic debugging
6. **Made lazy assumptions about APIs instead of reading actual source code**
7. **Gave up on research too quickly instead of using curl/bash to get real documentation**

**Don't repeat these patterns.**
