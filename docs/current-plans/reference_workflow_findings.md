# Reference Workflow Findings

## Key Goals
1. Load the reference clip at startup, extract its real features immediately, and keep the clip available for playback and recipe operations.
2. Ensure every reported reference metric (phonemes, contour, similarity, confidence) comes from real feature/alignment data—no synthetic placeholders.
3. Begin learner comparisons as soon as practical (≤0.5 s lag) and surface all captured audio, including any buffered lead-in.
4. Process learner chunks incrementally instead of cloning the entire history; keep latency and CPU overhead minimal.
5. Maintain continuous playback/toggling for original and flowalyzed variants using preserved clips plus cached features.

## Reference Clip Ownership
`SessionEngine::new_with_progress` takes a `RecordedClip` but renames it `_reference` and discards it immediately, leaving the clip owned solely by `EngineRunner`. The constructor just writes an empty cache:

```rust
pub fn new_with_progress(
    _reference: RecordedClip,
    alignment: AlignmentWeights,
    latency_budget_ms: u32,
    capture: C,
    _defer_feature_extraction: bool,
    _feature_progress: Option<&dyn Fn(FeatureExtractionEvent)>,
) -> Result<Self> {
    Ok(Self {
        capture,
        extractor: FeatureExtractor::new(),
        aligner: AudioAligner::new(alignment),
        metrics: MetricCalculator::new(),
        reference_features_cache: HashMap::new(),
        active_clip: ClipVariant::Original,
        learner_buffer: Vec::new(),
        latency_budget_ms,
        chunk_count: 0,
        last_processed_buffer_size: None,
    })
}
```

Because the engine throws away the clip, every later call must pass raw pointers back from `EngineRunner`, and the cache has to be warmed manually through `ensure_features`.

**Key goal impact:** Violates Goal 1 and complicates Goal 5 because the engine no longer owns the clip it supposedly initializes with.

## Fabricated “Reference Alignment”
`reference_alignment` never calls the aligner. It slices the reference features into fixed windows, invents phoneme labels (`"R0"`, `"R1"`, …), and derives “similarity” purely from reference energy variance:

```rust
let similarity_band: Vec<f32> = phonemes.iter()
    .enumerate()
    .map(|(i, _)| {
        let start_frame = (i * SEGMENT_FRAMES).min(ref_features.energy.len());
        let end_frame = ((i + 1) * SEGMENT_FRAMES).min(ref_features.energy.len());
        if start_frame < end_frame {
            let segment_energy = &ref_features.energy[start_frame..end_frame];
            let mean = segment_energy.iter().sum::<f32>() / segment_energy.len() as f32;
            let variance = segment_energy.iter()
                .map(|&e| (e - mean).powi(2))
                .sum::<f32>() / segment_energy.len() as f32;
            (variance.sqrt() * 10.0).min(1.0)
        } else {
            0.0
        }
    })
    .collect();
```

Contour values are likewise placeholders `(mean_pitch / 500.0).min(1.0)`. No learner data or alignment path is involved, so the emitted `AlignmentReport` contains fabricated phonemes, zero timing deltas, and confidence `1.0`—violating the “no fake values” constraint for reference analysis.

**Key goal impact:** Directly violates Goal 2 (metrics must be real) and undermines Goal 1’s “extract real features immediately” intent.

## Real-Time Shadowing Delay
`process_chunk` buffers amplified audio but refuses to extract learner features until at least a full second of samples exist and half a second of new audio has accrued:

```rust
#[cfg(not(test))]
const MIN_PROCESSING_SAMPLES: usize = TARGET_SAMPLE_RATE as usize; // 1s
const PROCESS_INTERVAL_SAMPLES: usize = TARGET_SAMPLE_RATE as usize / 2; // 0.5s
let should_process = if buffer_size_after >= MIN_PROCESSING_SAMPLES {
    let last_processed = self.last_processed_buffer_size.unwrap_or(0);
    let has_enough_new_audio = {
        let new_audio_since_last = buffer_size_after.saturating_sub(last_processed);
        new_audio_since_last >= PROCESS_INTERVAL_SAMPLES
    };
    if has_enough_new_audio {
        self.last_processed_buffer_size = Some(buffer_size_after);
        true
    } else {
        false
    }
} else {
    false
};
```

When the condition fails, the function exits immediately and nothing is analysed:

```rust
if !should_process {
    return Ok(None);
}
```

This throttling means the “real-time” comparison only starts after 1–1.5 s of speech, not “as the recorded audio happens.” Each pass then clones the entire buffer and re-extracts features, compounding latency.

### Desired Behavior
- Preprocess as soon as any meaningful chunk arrives. We can gate on the minimum duration the pitch tracker needs (e.g., ~200 ms) but not force an entire second before the first comparison.
- If the system must run a 0.5 s lag for stability, the initial 0.5 s of speech still needs to be analysed and surfaced once enough context exists. Currently, those early samples are skipped altogether because `process_chunk` returns `Ok(None)` until the thresholds are hit.

Potential alternatives:

- Lower `MIN_PROCESSING_SAMPLES` to the minimum frame window the feature extractor can handle (e.g., `TARGET_SAMPLE_RATE / 5` for 200 ms) and keep a rolling window so the first comparison happens almost immediately.
- Keep the existing buffer but, when `should_process` is false, record the pending samples and mark how much unprocessed audio remains. Once processing finally runs, replay the accumulated data so the first `AlignmentReport` includes the entire learner clip so far (initial ~0.5 s plus the newest chunk). No data should be thrown away simply because the first-pass gate wasn’t open.
- Process chunks incrementally instead of cloning the whole buffer. Right now we do:

```rust
let buffer_copy = self.learner_buffer.clone();
let clip = RecordedClip::from_samples(buffer_copy, TARGET_SAMPLE_RATE);
let features = self.extractor.extract(&clip)?;
```

Every poll clones all buffered samples and re-extracts features over the entire history. The incremental approach should:

1. Maintain a sliding analysis window (e.g., 0.5 s) and only append new samples into a feature extractor that can process overlapping frames.
2. Extract features for just the new chunk (with enough overlap to maintain continuity) and feed those into the aligner against the already cached reference features.
3. Store any leftover samples that were insufficient for a full frame hop and prepend them to the next chunk.

Deliverable: “Process chunks incrementally” means eliminating the full-buffer clone and instead running feature extraction per chunk with overlap, keeping the history only as long as needed for the aligner. That keeps latency bounded and ensures every part of the learner audio, including the first fraction of a second, is analysed and compared promptly.

**Key goal impact:** Violates Goal 3 (comparisons delayed >0.5 s) and Goal 4 (full-buffer cloning). Addressing the incremental processing plan satisfies both goals without contradicting playback requirements.
