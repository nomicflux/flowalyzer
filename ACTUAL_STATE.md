# Flowalyzer Actual State (2025-11-08)

## Summary
- CLI now requires a JSON recipe (`--recipe-json` or `--recipe-file`) and writes each processed chunk to its own subdirectory, enabling per-chunk review or export.
- Optional `--start` / `--end` trim arguments allow focusing on a subsection of the input before transcription.
- Whisper transcription remains mandatory at runtime; segment metadata is logged (sentence/word mix and preview text) to make transcript-derived fields actively used.
- Operations stack (`repeat`, `silence`, `change_speed`) is driven exclusively through the recipe system; each step declares whether it emits audio or silence (`silent: bool`), and speed changes now compensate for Signalsmith latency to preserve fastplay tails.
- `cargo clippy --all-targets --all-features` is clean; `cargo test` reports 34 passing + 1 ignored (Whisper) with no warnings.

## Pipeline Entry Point

`src/main.rs` has been refactored into small helpers. `run(args)` orchestrates validation, recipe loading, decode/trim, transcription logging, chunk planning, slicing, and per-chunk output writing.

```128:215:src/main.rs
fn run(args: Args) -> Result<()> {
    args.validate()
        .context("Failed to validate command-line arguments")?;
    print_banner(&args);
    let recipe = load_recipe(&args)?;
    log_recipe(&recipe);
    let trim = args.trim_range()?;
    log_trim_request(trim);
    let audio = decode_and_trim(&args, trim)?;
    let transcript = transcribe_with_logging(&audio)?;
    let boundaries = plan_chunks(&transcript, args.target_duration);
    let chunks = slice_chunks(&audio, &boundaries);
    write_chunks(&chunks, &boundaries, &recipe, &args.output_dir)?;
    println!("\n✓ Processing complete!");
    Ok(())
}
```

### CLI Contract
- Positional args: `INPUT` audio path, `OUTPUT_DIR` root directory for chunk outputs.
- Required recipe: exactly one of `--recipe-json` (inline string) or `--recipe-file` (path to JSON).
- Optional trimming: `--start`, `--end` accept seconds or `HH:MM:SS(.mmm)` and must satisfy `0 <= start < end`.
- `--target-duration` (default `2.0`) configures target chunk length (min/max derived automatically).
- Output directory is created if missing; each chunk is written to `chunk_{NNNN}/processed.wav` with progress logs every 10 chunks.

Example JSON recipe (`language-learning` equivalent):
```json
{
  "name": "language-learning",
  "steps": [
    {"repeat_count": 3, "speed_factor": 0.5, "silent": false},
    {"repeat_count": 1, "speed_factor": 0.5, "silent": true},
    {"repeat_count": 3, "speed_factor": 1.0, "silent": false},
    {"repeat_count": 1, "speed_factor": 1.0, "silent": true},
    {"repeat_count": 3, "speed_factor": 1.5, "silent": false},
    {"repeat_count": 1, "speed_factor": 1.5, "silent": true}
  ]
}
```

## Core Modules

### Audio Decode / Encode / Slice / Assemble
- `audio/decoder.rs`: Symphonia-based multi-format decode, converts everything to mono `f32`.
- `audio/encoder.rs`: Hound-based 16-bit mono WAV writer.
- `audio/slicer.rs`: Pure slice by time boundaries; metadata field removed.
- `audio/assembler.rs`: Crossfades between chunks (~2 ms minimum), validates sample-rate consistency.

### Transcription (Whisper)
- Uses `whisper-rs` greedy inference; requires `WHISPER_MODEL_PATH` or defaults to `./models/ggml-base.en.bin`.
- Segments are tagged as `Granularity::Word` when duration < 1 s, otherwise `Sentence`; CLI logs sentence/word counts plus a preview of the first three segments.
- Ignored test documents API usage; model download remains manual.

```47:118:src/transcription/mod.rs
for segment in state.as_iter() {
    let text = segment
        .to_str()
        .context("Failed to get segment text")?
        .to_string();
    let start_time = segment.start_timestamp() as f64 / 100.0;
    let end_time = segment.end_timestamp() as f64 / 100.0;
    let duration = end_time - start_time;
    let granularity = if duration < 1.0 {
        Granularity::Word
    } else {
        Granularity::Sentence
    };
    segments.push(Segment {
        text,
        start_time,
        end_time,
        granularity,
    });
}
```

### Linguistic Chunking
- `chunking::calculate_chunk_boundaries` greedily groups transcript segments to hit the target duration while respecting min/max bounds (50–150 % of target) and now tolerates a small overshoot (+30 %) before forcing a cut so phrases can end naturally.
- Segments that still exceed target + overshoot (e.g., background music) are mechanically split at the target interval.
- `ChunkBoundary::source_segment_ids` is used for logging (average segments per chunk and per-chunk counts).

### Operations and Recipes
- `repeat_chunk`, `insert_silence`, and `change_speed` remain pure functions.
- `change_speed` now pads for Signalsmith output latency, flushes remaining samples, drops the latency window, and normalizes length to avoid truncation of fast variants.
- `apply_recipe` consumes the original chunk for each recipe step; regression tests ensure recipes reuse source data and produce expected fast-play tails.
- Static dispatcher/processing-plan scaffolding has been removed; JSON recipes drive all behavior.

```38:152:src/operations/speed.rs
pub fn change_speed(chunk: &AudioChunk, speed_factor: f32) -> AudioChunk {
    if is_identity_speed(speed_factor) {
        return chunk.clone();
    }
    let mut stretch = configured_stretch(chunk.sample_rate);
    let target_len = target_length(chunk.samples.len(), speed_factor);
    let latency = stretch.output_latency().max(0) as usize;
    let mut samples =
        collect_stretched_samples(&mut stretch, &chunk.samples, target_len + latency, latency);
    adjust_for_latency(&mut samples, latency, target_len);
    let new_duration = samples.len() as f64 / chunk.sample_rate as f64;
    AudioChunk {
        samples,
        sample_rate: chunk.sample_rate,
        start_time: chunk.start_time,
        end_time: chunk.start_time + new_duration,
    }
}
```

```93:220:src/operations/recipe.rs
pub fn apply_recipe(chunk: &AudioChunk, recipe: &Recipe) -> Vec<AudioChunk> {
    let mut results = Vec::new();
    for step in &recipe.steps {
        let speed_adjusted = change_speed(chunk, step.speed_factor);
        if step.silent {
            let silence_duration = speed_adjusted.end_time - speed_adjusted.start_time;
            for _ in 0..step.repeat_count {
                results.push(insert_silence(
                    silence_duration,
                    speed_adjusted.sample_rate,
                ));
            }
        } else {
            results.extend(repeat_chunk(&speed_adjusted, step.repeat_count));
        }
    }
    results
}
```

## Tests and Tooling
- `cargo test` (2025-11-08): 34 passed, 1 ignored (Whisper integration), 0 failed.
- `cargo clippy --all-targets --all-features`: clean.
- Latency regression tests (`test_speed_fast_preserves_tail_energy`) and recipe reuse tests guard against prior truncation bugs.

## Dependencies and Runtime Requirements
- Key crates: `symphonia`, `hound`, `ssstretch`, `whisper-rs`, `clap`, `serde/serde_json`, `anyhow`, `dasp`.
- Build requirements: C++14 toolchain (Signalsmith Stretch), `cmake` (whisper-rs). Model download step unchanged.
- JSON recipes express repeat/speed/silence sequences; CLI no longer supports the legacy `--operations` flag.

## Known Gaps
- Whisper transcription is still required; no offline or mock mode exists.
- CLI produces verbose stdout; there is no quiet/log-level toggle.
- Integration tests rely on unit coverage; an end-to-end fixture that drives the binary remains a future enhancement.


