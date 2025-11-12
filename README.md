# Flowalyzer Pronunciation Tool

The pronunciation binary launches a real-time shadowing session that compares live microphone input against the supplied reference clip. All analysis happens during the session—no prerecorded learner files or offline analyzers are supported.

## Running the Session UI

- Build and run with `cargo run --bin pronunciation session --reference <path/to/reference.wav>`.
- Configure capture latency using `--latency-min` and `--latency-max` if the default 100–200 ms window needs adjustment.
- The UI presents waveform, spectrogram, and pitch contour windows that refresh continuously while you record.

## In-Session Controls

- `Space` toggles recording; capture always uses the live microphone stream.
- `R` restarts the shadowing loop, replaying the reference clip and beginning a fresh comparison.
- The Control Strip displays the current latency budget status with color-coded feedback and offers guidance when the pipeline approaches or exceeds 200 ms.

## Feedback Visualisation

- Waveforms, contour plots, and spectrograms retain the latest four seconds of activity to emphasise near-real-time differences.
- The phoneme timeline highlights segments with timing, articulation, or contour issues and exposes detailed tooltips for accessibility.
- Pitch overlays mark divergences in contour so relative movement mismatches are easy to spot even when absolute pitch differs.

## Testing

- `cargo test` exercises the streaming `SessionEngine` using mock capture sources; a headless runtime no longer exists, so test failures indicate interactive-session regressions that must be fixed before release.
