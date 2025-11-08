# Flowalyzer Actual State (2025-11-08)

## Summary
- End-to-end CLI pipeline is implemented and wired: decode → transcribe (Whisper) → chunk → slice → language-learning recipe → assemble → encode.
- A Whisper model file is required at runtime; transcription has no stub or fallback.
- Operations module includes repeat, silence, speed (using `ssstretch`), and a recipe dispatcher that produces 12 chunks per source chunk for the language-learning preset.
- `cargo test` currently reports 32 passing tests and 1 ignored test (Whisper integration), with several compiler warnings about unused exports and metadata fields.
- Several planning-time structures (`ProcessingPlan`, `ChunkMetadata`, etc.) remain unused but compiled in.

## Pipeline Entry Point

`src/main.rs` orchestrates the full processing sequence, always applying the language-learning recipe and printing progress updates.

```62:125:src/main.rs
fn main() -> Result<()> {
    let args = Args::parse();
    // ... existing code ...
    let audio = audio::decoder::decode_audio(&args.input_file)
        .context("Failed to decode input audio")?;
    // ... existing code ...
    let transcript = transcription::transcribe_audio(&audio)
        .context("Failed to transcribe audio")?;
    // ... existing code ...
    let chunk_boundaries = chunking::calculate_chunk_boundaries(&transcript, config);
    let chunks = audio::slicer::slice_audio(&audio, &chunk_boundaries);
    // ... existing code ...
    let recipe = types::Recipe::language_learning();
    for (i, chunk) in chunks.iter().enumerate() {
        let processed = operations::recipe::apply_recipe(chunk, &recipe);
        processed_chunks.extend(processed);
        if (i + 1) % 10 == 0 {
            println!("   Processed {}/{} chunks", i + 1, chunks.len());
        }
    }
    // ... existing code ...
    audio::encoder::encode_audio(&assembled, &args.output_file)
        .context("Failed to encode output audio")?;
    Ok(())
}
```

### CLI Contract
- Mandatory positional args: input path, output path.
- `--target-duration` (default `2.0`) controls chunk sizing; no operations flag is exposed.
- Validation enforces existing input file, positive durations, and warns if output extension is not `.wav`.

```15:59:src/main.rs
#[derive(Parser, Debug)]
struct Args {
    #[arg(value_name = "INPUT")]
    input_file: PathBuf,
    #[arg(value_name = "OUTPUT")]
    output_file: PathBuf,
    #[arg(long, default_value_t = 2.0)]
    target_duration: f64,
}
impl Args {
    fn validate(&self) -> Result<()> {
        if !self.input_file.exists() {
            anyhow::bail!("Input file does not exist: {:?}", self.input_file);
        }
        // ... existing code ...
        Ok(())
    }
}
```

## Core Modules

### Audio Decode / Encode / Slice / Assemble
- Decoder accepts multiple formats through Symphonia, downmixing to mono `f32` samples and handling all integer/float PCM variants.
- Encoder writes 16-bit mono WAV files via Hound.
- Slicer converts transcript boundaries to samples, cloning slices and preserving metadata stubs.
- Assembler concatenates chunks with a 2ms minimum crossfade and rejects mismatched sample rates.

```12:84:src/audio/decoder.rs
pub fn decode_audio<P: AsRef<Path>>(path: P) -> Result<AudioData> {
    // ... existing code ...
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;
    // ... existing code ...
    let mono_samples = convert_to_mono_f32(&decoded);
    all_samples.extend(mono_samples);
    // ... existing code ...
}
```

```17:58:src/audio/assembler.rs
let crossfade_samples = ((sample_rate as f64 * 0.002) as usize).max(10);
// ... existing code ...
assembled[fade_start + i] = assembled[fade_start + i] * prev_weight
    + chunk.samples[i] * curr_weight;
// ... existing code ...
```

### Transcription (Whisper)
- Uses `whisper-rs` with greedy sampling and disabled console output.
- Loads model path from `WHISPER_MODEL_PATH` or defaults to `./models/ggml-base.en.bin`; failure to load aborts the run.
- Iterates segments via `state.as_iter()`, converting centiseconds to seconds and tagging granularity as sentence-level.
- Unit test is ignored because it requires the actual model file.

```17:66:src/transcription/mod.rs
pub fn transcribe_audio(audio: &AudioData) -> Result<Transcript> {
    let model_path = std::env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "./models/ggml-base.en.bin".to_string());
    let ctx = WhisperContext::new_with_params(
        &model_path,
        WhisperContextParameters::default(),
    ).context("Failed to load Whisper model. Download with: wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin -P ./models/")?;
    // ... existing code ...
    for segment in state.as_iter() {
        let text = segment.to_str().context("Failed to get segment text")?.to_string();
        let start_time = segment.start_timestamp() as f64 / 100.0;
        let end_time = segment.end_timestamp() as f64 / 100.0;
        segments.push(Segment {
            text,
            start_time,
            end_time,
            granularity: Granularity::Sentence,
        });
    }
    Ok(Transcript { segments })
}
```

### Linguistic Chunking
- Builds greedy chunks constrained by `ChunkConfig` (target ±50%/±150%).
- Splits oversized segments into target-sized slices before resuming normal aggregation.

```5:92:src/chunking/mod.rs
pub fn calculate_chunk_boundaries(
    transcript: &Transcript,
    config: ChunkConfig,
) -> Vec<ChunkBoundary> {
    // ... existing code ...
    if segment_duration > config.max_duration {
        // split long segment into multiple chunks
        // ... existing code ...
    }
    // ... existing code ...
}
```

### Operations and Recipe
- `repeat_chunk`: clones the input chunk `count` times (returns empty for zero).
- `insert_silence`: generates zero-valued samples for the given duration/sample rate.
- `change_speed`: wraps `ssstretch::Stretch` with mono configuration; adjusts output length by factor and preserves sample rate.
- `apply_recipe`: for each `RecipeStep`, applies speed change, repeats the adjusted chunk, and conditionally appends silence matching the adjusted duration.
- Dispatcher `apply_operation` remains available but the CLI path uses `apply_recipe` directly.

```18:65:src/operations/repeat.rs
pub fn repeat_chunk(chunk: &AudioChunk, count: u32) -> Vec<AudioChunk> {
    if count == 0 {
        return Vec::new();
    }
    (0..count).map(|_| chunk.clone()).collect()
}
```

```38:78:src/operations/speed.rs
pub fn change_speed(chunk: &AudioChunk, speed_factor: f32) -> AudioChunk {
    if (speed_factor - 1.0).abs() < 1e-6 {
        return chunk.clone();
    }
    let mut stretch = Stretch::new();
    stretch.preset_default(1, chunk.sample_rate as f32);
    // ... existing code ...
    let output_len = (chunk.samples.len() as f32 / speed_factor).round() as usize;
    stretch.process_vec(&inputs, chunk.samples.len() as i32, &mut outputs, output_len as i32);
    // ... existing code ...
}
```

```45:200:src/operations/recipe.rs
pub fn apply_recipe(chunk: &AudioChunk, recipe: &Recipe) -> Vec<AudioChunk> {
    let mut results = Vec::new();
    for step in &recipe.steps {
        let speed_adjusted = change_speed(chunk, step.speed_factor);
        let repeated = repeat_chunk(&speed_adjusted, step.repeat_count);
        results.extend(repeated);
        if step.add_silence_after {
            let silence_duration = speed_adjusted.end_time - speed_adjusted.start_time;
            let silence = insert_silence(silence_duration, speed_adjusted.sample_rate);
            results.push(silence);
        }
    }
    results
}
```

## Tests and Warnings
- `cargo test` (2025-11-08) → 32 passing, 1 ignored (Whisper requires model), 0 failures. Run time ~0.27s.
- Compiler warnings highlight unused re-exports in `src/audio/mod.rs` and `src/operations/mod.rs`, plus unused fields/types in `src/types.rs`.
- No automated integration test ensures the CLI pipeline completes successfully with an actual model; manual runs rely on downloading `ggml-base.en.bin`.

## Dependencies and Runtime Requirements
- `symphonia` (audio decoding), `hound` (WAV encoding), `ssstretch` (time-stretch), `whisper-rs` (speech-to-text), `clap` (CLI), `anyhow` (error handling), `dasp` (DSP utilities, not exercised directly in current code paths).
- Requires `cmake` (transitively for Whisper) and a Whisper GGML model file. Default lookup path `./models/ggml-base.en.bin` exists in the repository workspace.

## Known Gaps vs Planning Docs
- Planning documents mention optional CLI operations parsing; current CLI hardcodes the language-learning recipe and omits a user-selectable operations flag.
- `ProcessingPlan`, `ProcessingRule`, `ChunkSelector`, and `ChunkMetadata` structures exist but are unused in production flow, matching neither the plan nor runtime log messages.
- Numerous `println!` progress statements are present; no structured logging or quiet mode exists.


