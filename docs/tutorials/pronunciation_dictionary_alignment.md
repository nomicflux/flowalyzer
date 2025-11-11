# Pronunciation Dictionary & Alignment Tutorial

This walkthrough explains how Flowalyzer’s pronunciation pipeline loads a CMU-style lexicon, interprets ARPABET phoneme symbols, and uses them during the alignment phase.

## CMU-Style Pronunciation Dictionary

- **File**: `assets/phonemes/lexicon.txt`
- **Format**: Each line contains a token followed by its phoneme sequence.
  - Example: `ALIGNMENT  AH0 L AY1 N M AH0 N T`
  - Words are uppercase; parenthetical suffixes (e.g., `(1)`) designate alternate pronunciations and are removed by the loader.
- **Storage**: The file is baked into the binary with Rust’s `include_str!` and is tracked by `build.rs` to trigger rebuilds when it changes.

## ARPABET Refresher

- ARPABET is a phonetic transcription system used by CMUdict.
- Symbols represent phonemes (e.g., `P`, `R`, `AY1`).
- Vowels carry stress markers:
  - `0` – no stress
  - `1` – primary stress
  - `2` – secondary stress
- Consonants have no stress digits (e.g., `S`, `CH`, `ZH`).
- Flowalyzer keeps the symbols as-is; downstream code may optionally strip stress suffixes if a model requires it.

## Dictionary Loader

- **Module**: `src/pronunciation/alignment/dictionary.rs`
- Uses a global `DEFAULT_DICTIONARY` (`once_cell::sync::Lazy`) backed by the bundled lexicon.
- Key steps:
  1. Normalize tokens (uppercase, strip punctuation other than apostrophes).
  2. Parse each lexicon line into phoneme sequences.
  3. Store variants in `HashMap<String, Vec<Box<[&'static str]>>>`.
- Primary APIs:
  - `PronunciationDictionary::shared()` – access the singleton.
  - `lookup(token)` – get phoneme variants for one token.
  - `map_tokens(tokens)` – map an entire transcript.
- Tests: `tests/alignment/dictionary.rs` verifies normalization, lookups, and error handling.

## Template Generation

- **Module**: `src/pronunciation/alignment/templates.rs`
- Function `build_templates(reference_features, phonemes)`:
  - Evenly partitions reference frames by phoneme count.
  - Computes MFCC centroids and average energy per segment.
  - Returns `PhonemeTemplate` structs consumed by the DTW core.

## Dynamic Time Warping Alignment

- **Module**: `src/pronunciation/alignment/dtw.rs`
- Function `align_templates(templates, learner_features)`:
  1. Runs monotonic DTW over phoneme templates and learner frames.
  2. Accumulates costs combining MFCC distance and energy penalty.
  3. Backtracks to produce ordered `AlignmentSegment` entries.
  4. Converts frame indices to milliseconds via `frames_to_ms`.
- Tests: `tests/alignment/dtw.rs` plus fixtures in `tests/fixtures/alignment/` validate segment boundaries and cost aggregation.

## How It Fits Together

1. **Transcript Mapping**: The CLI or UI collects a transcript; `PronunciationDictionary::map_tokens` converts each token to ARPABET sequences.
2. **Template Synthesis**: Reference audio features go through `build_templates` using one chosen variant per token (Phase 4 defaults to the first variant).
3. **Learner Alignment**: `align_templates` compares learner features against templates and returns timing data and similarity scores for each phoneme.
4. **Reporting**: `PhonemeAligner::align` assembles these results into an `AlignmentReport`, which later phases will feed into scoring and visualization.

## Extending or Customizing

- **Adding Pronunciations**: Append lines to `assets/phonemes/lexicon.txt`. Run `cargo fmt` + the full test suite to ensure the dictionary still loads.
- **Alternative Stress Handling**: Strip digits in `normalize_token` or before template generation if a stressless comparison is required.
- **Multiple Pronunciation Variants**: In future phases, iterate through variants (or use language models) to select the best match before generating templates.

## Next Steps

For deeper integration details, follow Phase 4 of `docs/current-plans/RECORDING_AND_ANALYSIS.md` and inspect `src/pronunciation/mod.rs`, where alignment integrates with metrics and the UI pipeline.

