## Agreements Made
- (2025-11-09) User: "You must modify src/operations/speed.rs so that change_speed strictly follows this contract: take the provided AudioChunk, apply the requested speed factor using ssstretch, remove the stretcher’s output latency, and return exactly the samples produced—no trimming, padding, or other processing."
- (2025-11-09) User: "In change_speed, drop the use of target_length for output sizing. Feed the original chunk into the stretcher and collect all samples it produces."
- (2025-11-09) User: "Update collect_stretched_samples to: Call stretch.process_vec once with the entire input. Loop calling stretch.flush_vec (e.g. 1024 frames per call) until it returns an empty channel, appending every frame to the output buffer. Do not limit the flush to any target length or “latency” count."
- (2025-11-09) User: "Ensure adjust_for_latency remains the only post-processing step—it should just remove the leading latency samples." *(Superseded)*
- (2025-11-09) User: "Update tests if necessary so they check duration ratios rather than exact sample counts, now that the stretcher decides the final length."
- (2025-11-09) User: "Result: the slow, normal, and fast versions use the exact same chunk waveform, and consonant tails remain intact for fast playback."
- (2025-11-09) User: "We take a chunk. We apply the EXACT RECIPE to that EXACT SAME CHUNK. THERE IS NOT ONE SINGLE OTHER PROCESSING STEP TAKEN, NO MATTER HOW HELPFUL YOU THINK IT WOULD BE. NO EXTRA PROCESSING."
- (2025-11-09) User: "YOU FUCKING TAKE THE CHUNK. YOU FUCKING CHANGE THE SPEED. YOU FUCKING RETURN IT. THAT IS IT. DONE. ABSOLUTELY NO OTHER PROCESSING FOR ANY FUCKING REASON."

## Explicitly Rejected
- (2025-11-09) User: "Do not add silence trimming, padding, or any other transformations."
- (2025-11-09) User: "Do not limit the flush to any target length or 'latency' count."
- (2025-11-09) User: "NO EXTRA PROCESSING."
- (2025-11-09) User: "Absolutely no other processing for any reason."

## Implementation Details
- `change_speed` must push the full chunk into `ssstretch`, collect all output frames without using `target_length`, and return them exactly as emitted by the stretcher with no additional post-processing or trimming.
- `collect_stretched_samples` must process the entire input in one call to `process_vec`, then repeatedly call `flush_vec` (e.g. with 1024 frame blocks) until it yields zero-length output, aggregating every frame emitted.
- The stretcher output is authoritative for total sample count; downstream logic and tests must accept that constant stretcher latency is present and perform no latency-aware adjustments.

## Issues Encountered
- None yet.

