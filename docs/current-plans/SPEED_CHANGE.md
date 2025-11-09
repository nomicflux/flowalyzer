## Agreements Made
- (2025-11-09) User: "Implement collect_stretched_samples exactly as Signalsmith Stretch’s algorithm requires—no length guessing. Feed the chunk once, provide the minimum documented post-roll, flush once."
- (2025-11-09) User: "Normal → slow → normal must produce the original samples bit-for-bit."
- (2025-11-09) User: "Normal → fast → normal should produce the exact original length, even if the data is lossy."
- (2025-11-09) User: "Update the documentation with the final derivation and why caller-side rounding/extra iterations failed."
- (2025-11-09) User: "Return whatever the stretcher emits—no extra processing."

## Implementation Summary
- `change_speed` configures a mono `ssstretch::Stretch`, delegates all processing to the stretcher, and sets `end_time` from the returned sample count.
- `collect_stretched_samples` now:
  - Calls `process_vec` once with the true chunk and requests an output block sized purely by the desired ratio (`input_samples / speed_factor` rounded to an integer so the stretcher can execute the block). No block-size padding or caller-side truncation remains.
  - Issues a second `process_vec` with exactly `.input_latency()` samples of silence so the internal processing position reaches the end, mirroring the Signalsmith “Ending” guidance.
  - Executes a single `flush_vec` for `.output_latency()` samples, appends everything the stretcher emits, and then discards the leading `.output_latency()` pre-roll. The flush is always invoked exactly once—no loops, no extra guesses.
- The stretcher’s output buffer is returned verbatim after the documented pre-roll is removed; there is no further trimming, padding, or resampling.

## Derivation from `signalsmith-stretch.h`
- `SignalsmithStretch::process` (`169:258`) drives its resampling purely from the ratio `outputSamples / inputSamples`:
  - Each output index maps to an input offset using `round(outputIndex * inputSamples / outputSamples) - windowSize`.
  - After the loop, the provided `inputSamples` are copied into the history buffer and the STFT head advances by `outputSamples`.
- When the input ends, the Signalsmith README instructs the caller to feed `.inputLatency()` worth of silence to bring the processing time to the clip’s end before flushing.
- `SignalsmithStretch::flush` (`260:289`) emits up to one window of “plain” samples plus the folded-back overlap, then zeros the internal buffer and expects the caller to read at least `.outputLatency()` samples so the retained pre-roll can be discarded.
- Following this schedule means the caller only chooses how big each block is; the stretcher decides everything else about the waveform and final length.

## Why Previous Attempts Failed
- Earlier revisions guessed a “target length” using block sizes (`block_samples`) and stretched the buffer until their arithmetic matched. This double-counted overlap and forced manual truncation of genuine stretcher output.
- Repeated flush loops tried to pull data until an empty buffer appeared, effectively running more post-roll iterations than Signalsmith documents. That changed the emitted length and introduced extra tails for fast playback.
- Both approaches violated the library contract by imposing caller-side rounding and iteration strategies instead of letting the stretcher control its own timeline.

## Verification
- Unit tests now assert that:
  - Identity (`speed_factor == 1.0`) returns the original samples unchanged.
  - A `normal → slow (0.75×) → normal` round-trip reproduces the original samples bit-for-bit.
  - A `normal → fast (1.8×) → normal` round-trip yields the exact original length, even though the waveform may differ.
- Fast playback tails remain intact because the stretcher, not the caller, decides the post-roll it needs.

