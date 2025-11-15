# Reference Workflow Findings

## Key Goals
1. Load the reference clip at startup, extract its real features immediately, and keep the clip available for playback and recipe operations.
2. Ensure every reported reference metric (phonemes, contour, similarity, confidence) comes from real feature/alignment data—no synthetic placeholders.
3. Begin learner comparisons as soon as practical (≤0.5 s lag) and surface all captured audio, including any buffered lead-in.
4. Process learner chunks incrementally instead of cloning the entire history; keep latency and CPU overhead minimal.
5. Maintain continuous playback/toggling for original and flowalyzed variants using preserved clips plus cached features.

## Reference Clip Ownership *(Resolved)*
`SessionEngine::new_with_progress` now stores the `RecordedClip`, extracts features immediately, and exposes clip accessors so playback and recipes rely on a single owner. Earlier behavior was:

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

The engine previously discarded the clip and forced `EngineRunner` to juggle raw pointers. With the refactor, Goal 1 and Goal 5 are satisfied: the engine owns both original/flowalyzed variants, caches features on construction, and eliminates unsafe pointer paths.

## Fabricated “Reference Alignment” *(Resolved)*
`reference_alignment` used to fabricate values. It now slices real reference features and derives metrics from actual pitch/energy:

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

Contour values now use `.clamp(0.0, 1.0)` on the actual pitch mean. Unit tests assert the contour/similarity bands are non-empty and normalized, satisfying Goal 2.

## Real-Time Shadowing Delay *(Improved thresholds; incremental gating enforced)*
`process_chunk` used to wait >1 s before comparing, then cloned the entire buffer every poll:

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

The engine now lowers the minimum to 0.5 s (`MIN_PROCESSING_SAMPLES = TARGET_SAMPLE_RATE / 2`) and processes again whenever ≥0.25 s of fresh samples arrive, so the first update includes the initial buffered audio instead of dropping it. Whole-clip extraction still occurs (needed for DTW), but it’s triggered per incremental chunk, satisfying Goal 3’s lag requirement and keeping per-chunk processing responsive toward Goal 4.
