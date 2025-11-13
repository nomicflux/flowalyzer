# Understanding Signal Processing for Pronunciation Analysis: A Tutorial

If you're new to signal processing, the pronunciation training system might seem like magic: you speak into a microphone, and somehow the computer knows if your pronunciation matches a reference. This tutorial demystifies the process by explaining the signal processing pipeline step by step.

We'll build your understanding progressively, starting with basic concepts and working up to sophisticated feature extraction. By the end, you'll understand not just *what* the system does, but *why* it does it that way.

## The Fundamental Challenge: Audio is Just Numbers

When you speak into a microphone, the physical vibrations of air molecules get converted to electrical signals, which get converted to digital samples - just numbers representing amplitude at discrete time points. At 16,000 samples per second (16kHz), one second of audio is 16,000 numbers between -1.0 and 1.0.

But these numbers, by themselves, don't tell us much. Looking at raw samples, you can't easily tell:
- What phonemes were spoken
- Whether pronunciation was accurate
- How pitch changed over time
- Whether articulation was crisp or mumbled

The signal processing pipeline transforms these raw samples into features that capture linguistically meaningful information. Let's follow this transformation step by step.

## Step 1: Getting to a Common Format

**The Problem**: Audio comes in many sample rates. Microphones might capture at 44.1kHz (CD quality), 48kHz (professional audio), or other rates. The reference file might be at yet another rate. To compare them, we need a common format.

**The Solution**: Resample everything to 16kHz.

**Why 16kHz?**

This isn't arbitrary. Human speech has most of its important information below 8kHz. The highest formant (resonance of the vocal tract) for most speech sounds is around 5-6kHz. By the Nyquist theorem, to capture frequencies up to 8kHz, we need a sample rate of at least 16kHz (twice the maximum frequency).

Going higher (like 44.1kHz) would capture ultrasonic information that doesn't exist in speech. It would also multiply our processing time - more samples means more computation. 16kHz is the sweet spot: captures all speech information, processes efficiently.

**How It Works**: Linear interpolation (see `src/audio/resample.rs`, lines 4-23).

Imagine you have samples at 48kHz and want 16kHz - a 3:1 downsampling. For every 3 samples in the input, you want 1 sample in the output. But the positions don't align exactly - you might want a sample at position 2.7 in the input.

Linear interpolation estimates this value by drawing a line between samples 2 and 3:

```
output = (1 - 0.7) * sample[2] + 0.7 * sample[3]
       = 0.3 * sample[2] + 0.7 * sample[3]
```

It's a weighted average where the weights depend on distance. Closer to sample[3]? Weight it more heavily.

This isn't perfect - fancier algorithms (like sinc interpolation) are more accurate but much slower. For speech, linear interpolation is good enough and very fast.

## Step 2: From Time to Frequency - The STFT

**The Problem**: Raw samples show amplitude over time, but pronunciation is really about frequencies. When you say "ah", your vocal tract resonates at certain frequencies (formants). When you say "ee", different frequencies. We need to see the frequency content.

**The Solution**: Short-Time Fourier Transform (STFT).

**Understanding the Fourier Transform**:

The Fourier Transform is based on a profound mathematical insight: any periodic signal can be represented as a sum of sine waves at different frequencies. When you speak, your vocal cords create periodic vibrations, and your vocal tract filters those vibrations, emphasizing some frequencies and suppressing others.

The Fourier Transform finds those frequency components. If you say "ah" and compute the Fourier Transform, you'll see peaks at the formant frequencies (resonances of your vocal tract in that configuration).

**Why "Short-Time"?**

Speech changes over time. If you compute the Fourier Transform of an entire sentence, you get a meaningless average of all the frequencies over the whole duration. Instead, we need to see how frequencies change over time.

The solution: compute Fourier Transforms on short windows (25ms) and slide that window across the audio. This gives us a 2D representation: time on one axis, frequency on the other, amplitude encoded in the value.

**The Implementation** (see `src/pronunciation/features/mel.rs`, lines 44-59):

```rust
let fft_size = 400;  // 25ms at 16kHz
let hop_size = 160;  // 10ms at 16kHz
let stft = spectrum::rstft(&audio, fft_size, hop_size, WindowType::Hanning);
```

**Window Size (25ms / 400 samples)**: Why this duration?

Speech phonemes typically last 50-200ms. To see transitions between phonemes, we need windows shorter than that. But we also need multiple periods of the fundamental frequency (pitch) in each window - otherwise, we can't accurately determine frequency content.

Most adult male voices have a fundamental frequency around 100-150 Hz (period of 7-10ms). Most adult female voices are around 150-250 Hz (period of 4-7ms). A 25ms window contains 2-6 pitch periods, which is enough for accurate frequency analysis.

Too short (say, 5ms) and we can't resolve low frequencies accurately. Too long (say, 100ms) and we blur together multiple phonemes. 25ms is the empirically determined sweet spot for speech.

**Hop Size (10ms / 160 samples)**: Why not slide by just 1 sample?

We could compute an FFT at every sample position (hop size = 1), but that's wasteful - consecutive windows would overlap by 99.75%, giving us 400 times more data than necessary with minimal additional information.

We could skip windows entirely (hop size = 400), but that gives us only 40 frames per second, which is too coarse for seeing consonant transitions.

10ms (100 frames per second) is smooth enough to see all speech transitions but efficient enough to process in real-time.

**Hanning Window**: Why multiply by a window function?

When we extract a 25ms segment from continuous audio, we're abruptly cutting it off at the edges. This creates artificial discontinuities. In frequency space, discontinuities create "spectral leakage" - energy spreads into neighboring frequency bins where it doesn't belong.

The Hanning window smoothly tapers the signal to zero at the edges, eliminating these discontinuities. It's shaped like a bell curve: full weight in the middle, zero at the edges. This costs us a bit of frequency resolution but gives much cleaner spectral content.

**The Output**:

The STFT returns complex numbers: magnitude (how much of this frequency is present) and phase (where in the cycle). For pronunciation analysis, we mostly care about magnitude - phase is less perceptually relevant for speech.

## Step 3: From Linear Frequency to Perceptual Frequency

**The Problem**: The STFT gives us frequency bins that are evenly spaced in Hz: 0Hz, 80Hz, 160Hz, 240Hz, ... But human hearing isn't linear in frequency.

You can easily hear the difference between 100Hz and 200Hz (a doubling). But you can barely tell the difference between 8000Hz and 8100Hz (also 100Hz difference, but now a tiny fraction). Our hearing is roughly logarithmic - we hear ratios, not absolute differences.

**The Solution**: Convert to mel scale.

**What is Mel Scale?**

The mel scale is a perceptual scale of pitches judged by listeners to be equal in distance from one another. The formula is:

```
mel(f) = 2595 * log10(1 + f/700)
```

At low frequencies, mels and Hz are nearly linear (1000Hz ≈ 1000 mels). At high frequencies, mels compress many Hz into each mel (8000Hz ≈ 2840 mels).

**How We Apply It**: Mel filterbank (see `src/pronunciation/features/mel.rs`, lines 79-103).

We create 80 triangular filters spaced evenly on the mel scale. Each filter covers a range of frequencies:
- Filter 1 might cover 20-80 Hz
- Filter 40 might cover 1000-1200 Hz  
- Filter 80 might cover 6000-8000 Hz

Notice how low filters are narrow (80 Hz wide) and high filters are wide (2000 Hz wide). This matches our perception - we have fine frequency resolution at low frequencies, coarse resolution at high frequencies.

We multiply each filter by the power spectrogram and sum the result. This gives us 80 mel bands representing the perceptually relevant frequency content.

**Why 80 Bands?**

This is informed by the physiology of the cochlea (inner ear). The basilar membrane has roughly 3000 inner hair cells distributed along its length, but they're not evenly spaced - more are dedicated to low frequencies. 80 bands is a practical compression that captures most perceptually important information.

More bands would give finer frequency resolution but make processing slower and alignment harder (more dimensions to compare). Fewer bands would lose important detail. 80 is, once again, an empirically determined sweet spot.

**The Result**:

We now have 80 mel bands × N frames, where N depends on audio duration. Each value represents how much energy is present in that perceptual frequency band at that time. This is the mel spectrogram - the foundation for higher-level features.

## Step 4: Capturing Vocal Tract Shape - MFCCs

**The Problem**: The mel spectrogram has 80 bands per frame. That's a lot of dimensions to compare when aligning reference and learner audio. Also, it's sensitive to overall amplitude (recording volume) and includes pitch information (which varies even for correct pronunciation of the same phoneme).

**The Solution**: Mel-Frequency Cepstral Coefficients (MFCC).

**Understanding Cepstrum**:

The word "cepstrum" is a play on "spectrum" - it's the spectrum of a spectrum. Here's the idea:

When you speak, your vocal cords create a periodic waveform (the source) which gets filtered by your vocal tract (the filter). The vocal tract's shape determines its resonances (formants). In frequency space, this is multiplication: signal = source × filter.

We want to separate source from filter. Multiplication in frequency space becomes addition in log space:

```
log(signal) = log(source) + log(filter)
```

Then we apply another Fourier-like transform (the Discrete Cosine Transform, DCT) to separate rapidly varying components (source/pitch) from slowly varying components (filter/vocal tract shape).

The slowly varying components (low DCT coefficients) represent the spectral envelope - the overall shape determined by your vocal tract. This is what we care about for pronunciation - is your tongue in the right position? Is your mouth open enough?

**The Implementation** (see `src/pronunciation/features/statistics.rs`, line 27):

```rust
let mfcc_raw = analysis::mel::mfcc_spectrogram(mel_spectrogram, 13, None);
```

This computes:
1. log(mel_spectrogram) - compresses dynamic range, makes multiplication into addition
2. DCT(log_mel) - separates spectral envelope from fine structure
3. Keep first 13 coefficients - these capture the envelope

**Why 13 Coefficients?**

Coefficient 0 represents overall log energy (average loudness). Coefficients 1-12 represent the spectral envelope shape. Higher coefficients represent finer detail in the envelope, but they're less stable and more sensitive to noise.

13 is a standard choice in speech recognition - enough to capture all important articulatory information, not so many that we include noise. You could use 12 or 16 without much difference, but 13 has historical momentum.

**What Each Coefficient Means**:

- **MFCC[0]**: Average log energy (how loud)
- **MFCC[1]**: Spectral tilt (ratio of low to high frequencies)
- **MFCC[2]**: First formant vicinity (tongue height)
- **MFCC[3]**: Second formant vicinity (tongue position front/back)
- **MFCC[4-12]**: Higher-order spectral shape details

When you say "ah", you get one pattern of MFCCs. When you say "ee", you get a different pattern. These patterns are relatively stable across different speakers and recording conditions - they capture articulation, not voice quality.

## Step 5: Capturing Temporal Dynamics - Deltas

**The Problem**: MFCCs capture what the vocal tract shape is at each instant, but pronunciation is also about how it moves. The difference between "bad" and "bat" is tiny in steady-state articulation but huge in how quickly the vocal tract closes.

**The Solution**: Delta and delta-delta features.

**Understanding Deltas**:

Delta features measure how MFCCs change over time - they're derivatives. If MFCCs are position, deltas are velocity.

**The Math** (see `src/pronunciation/features/statistics.rs`, lines 90-119):

```rust
delta[t] = Σ n * (mfcc[t+n] - mfcc[t-n]) / (2 * Σ n²)
```

This is a weighted linear regression over ±2 frames (±20ms). It asks: "which direction and how fast are the MFCCs changing?"

For example, during the transition from "b" to "ah" in "bad", you'd see:
- MFCCs shifting from closed vocal tract (stop consonant) to open vocal tract (vowel)
- Large positive deltas (rapid opening)
- Delta-deltas near zero (constant velocity)

During the transition from "ah" to "d" in "bad", you'd see:
- MFCCs shifting from open to closed
- Large negative deltas (rapid closing)
- Delta-deltas near zero

For "bat" vs "bad", the steady state "a" vowel is similar (similar MFCCs), but:
- "bat" ends with a voiceless stop (abrupt closure, very large negative deltas)
- "bad" ends with a voiced stop (gentler closure, smaller negative deltas)

Deltas capture these dynamic differences.

**Delta-Delta (Acceleration)**:

Delta-delta features are deltas of deltas - second derivatives, or acceleration. They capture how quickly the rate of change is changing.

This matters for distinguishing sounds with different trajectories. A sound that starts slowly and speeds up (positive delta-delta) sounds different from one that moves at constant speed (zero delta-delta), even if they have the same average velocity.

**Why This Works for Alignment**:

When comparing reference and learner audio, we want to know: did you make the right movements at the right speed? Deltas and delta-deltas capture this. If your deltas match the reference, you're moving your articulators at the right speed. If your delta-deltas match, you're accelerating and decelerating at the right times.

## Step 6: Detecting Articulation Changes - Spectral Flux

**The Problem**: Consonants often involve rapid changes in spectral content. A "t" or "k" is a brief burst of noise across all frequencies. We want to detect these articulation events.

**The Solution**: Spectral flux.

**Understanding Spectral Flux**:

Spectral flux measures how much the spectrum changed from one frame to the next. The implementation (see `src/pronunciation/features/statistics.rs`, lines 62-79):

```rust
for i in 1..magnitude.len() {
    let previous = &magnitude[i - 1];
    let current = &magnitude[i];
    let mut sum = 0.0;
    for (curr, prev) in current.iter().zip(previous.iter()) {
        let diff = (curr - prev).max(0.0);
        sum += diff * diff;
    }
    flux.push(sum.sqrt());
}
```

This computes, for each frequency bin:
- How much did the magnitude increase? (ignoring decreases)
- Square it (emphasize large changes)
- Sum across all bins
- Take square root (convert to magnitude-like units)

**Why Only Increases?**

The `.max(0.0)` is called half-wave rectification. We only count increases, not decreases. Why?

Onsets (the start of new sounds) are perceptually more important than offsets (the end of sounds). When you say "cat", you notice the onset of "c" (sudden increase in high-frequency noise) more than the offset of the previous silence. Half-wave rectification emphasizes onsets.

**What High Flux Means**:

- **High flux**: Rapid spectral change (consonant onset, plosive burst, voice onset)
- **Low flux**: Stable spectrum (sustained vowel, continuant)

For example, in "stop":
- "s": High flux at onset (silence to noise), then low (stable hissing)
- "t": Very high flux (silence to burst to voice)
- "o": Moderate flux (transition to vowel), then low (stable vowel)
- "p": High flux (voice to silence)

Spectral flux helps align audio at phoneme boundaries. When both reference and learner have high flux at similar relative times, they're probably hitting the same consonants.

## Step 7: Measuring Loudness - Energy

**The Problem**: Pronunciation includes stress patterns. "REcord" (noun) vs "reCORD" (verb) differ in which syllable is emphasized. Emphasis correlates with loudness.

**The Solution**: Frame-by-frame energy.

**The Math** (see `src/pronunciation/features/statistics.rs`, lines 81-88):

```rust
for frame in power {
    let sum: f64 = frame.iter().sum();
    energies.push(sum.sqrt());
}
```

This computes RMS (root mean square) energy: square root of the sum of power across all frequencies.

**Why RMS?**

Power is proportional to amplitude squared. Summing power gives total energy. Taking the square root converts back to amplitude-like units (which are easier to think about and visualize).

**What Energy Captures**:

- **Voiced sounds** (vowels, nasals, approximants): High energy (periodic waveforms have lots of power)
- **Voiceless fricatives** (s, f, sh): Medium energy (noise has less power than voiced sounds)
- **Stops** (p, t, k, b, d, g): Low energy (closure followed by brief burst)
- **Silence**: Very low energy

Comparing energy envelopes helps verify:
- Are you stressing the right syllables?
- Are your vowels appropriately loud compared to consonants?
- Are you maintaining consistent volume?

## Step 8: Capturing Intonation - Pitch Contour

**The Problem**: Pronunciation isn't just articulation - it's also prosody (pitch, rhythm, stress). Questions end with rising pitch. Statements end with falling pitch. Stress involves pitch changes as well as loudness.

**The Solution**: Pitch estimation via PYIN algorithm.

**Understanding Pitch**:

Pitch (perceived frequency) corresponds to the fundamental frequency (F0) of vocal cord vibration. When you speak, your vocal cords open and close periodically. The rate of this vibration is F0, typically:
- Adult males: 100-150 Hz
- Adult females: 150-250 Hz
- Children: 250-400 Hz

Pitch gives speech its melodic quality. Different languages use pitch differently (lexical tone in Mandarin, phrasal intonation in English), but all languages use it.

**The Challenge of Pitch Estimation**:

Unlike finding frequency components in music (where instruments play clear pitches), speech is messy. The signal contains:
- Fundamental frequency (F0)
- Harmonics (2×F0, 3×F0, 4×F0, ...)
- Formants (resonances that emphasize certain harmonics)
- Noise (consonants, breathing, room ambiance)

Simple algorithms (like autocorrelation) can be confused by strong harmonics or noise. PYIN (Probabilistic YIN) is a sophisticated algorithm that handles these challenges.

**How PYIN Works** (conceptually):

1. **Autocorrelation**: Compute how similar the signal is to a shifted version of itself. If there's a periodic component at frequency F, the signal will correlate well with itself shifted by 1/F seconds.

2. **Normalized Difference**: Instead of raw correlation, use cumulative mean normalized difference (CMND). This is a trick that makes the algorithm more robust to amplitude variations.

3. **Peak Detection**: Find local minima in CMND (corresponding to possible pitch periods).

4. **Probabilistic Selection**: Instead of just picking the lowest minimum, compute probabilities for each candidate pitch based on how clear the minimum is and how consistent it is with neighbors.

5. **Path Selection**: Use dynamic programming to find a smooth pitch trajectory over time (real pitch changes smoothly, so we prefer smooth paths).

**The Implementation** (see `src/pronunciation/features/contour.rs`, lines 57-63):

```rust
let (timestamps, pitches, voiced_flags, confidence) = 
    analysis::pyin_pitch_estimator(&audio, 16000, 55.0, 1200.0, 400);
```

- **Frequency range [55Hz, 1200Hz]**: Covers human speech (lowest bass males to highest soprano females) plus margin
- **Frame length 400 samples**: Matches our STFT window (25ms) so pitch frames align with spectral frames
- **Outputs**: Pitch in Hz, voiced/unvoiced flags (is this speech or silence?), confidence scores

**Post-Processing Pitch** (see `src/pronunciation/features/contour.rs`, lines 124-240):

Raw pitch values are noisy and have missing frames (unvoiced segments). We do several things:

**1. Normalize to Semitones** (lines 124-141):

Instead of raw Hz, we convert to semitones relative to median pitch:

```rust
semitone = 12 * log2(pitch / median_pitch)
```

This makes comparison speaker-independent. Your absolute pitch might be different from the reference (male vs female voice), but the relative pitch changes (rising at questions, falling at statements) should be similar.

**2. Fill Missing Values** (lines 161-197):

During unvoiced segments (consonants like "s", "t"), pitch is undefined. We use forward-fill (copy last valid pitch forward) and backward-fill (copy first valid pitch backward) to create a continuous contour. This lets us visualize pitch over the entire utterance, not just voiced segments.

**3. Smooth** (lines 199-213):

We apply a 5-frame (50ms) moving average. This removes jitter (rapid frame-to-frame variations) while preserving actual pitch movements (which happen over 100-200ms).

**4. Align to Spectral Frames** (lines 215-240):

PYIN produces pitch frames at its own rate. We interpolate to match our STFT frame rate (100 Hz) so all features align temporally.

**Why Pitch Matters**:

Comparing pitch contours reveals:
- Are you using rising intonation for questions?
- Are you using falling intonation for statements?
- Are you stressing the right syllables (stress involves pitch rise)?
- Is your overall melody matching the reference?

## Step 9: Making Features Comparable - Normalization

**The Problem**: Different recordings have different characteristics. One recording might be loud (high energy), another quiet. One speaker might speak close to the microphone (emphasizing low frequencies), another farther away. We want to compare pronunciation, not recording quality.

**The Solution**: Z-score normalization.

**The Math** (see `src/pronunciation/features/statistics.rs`, lines 121-140):

```rust
normalized = (value - mean) / std_dev
```

This:
- Centers the data at zero (subtracting mean)
- Scales to unit variance (dividing by standard deviation)

After normalization:
- Mean ≈ 0 (no overall offset)
- Standard deviation ≈ 1 (no overall scaling)

**Why This Works**:

Imagine comparing two recordings: one where you spoke loudly into a sensitive microphone, one where you spoke quietly. Without normalization, energies would be very different even if pronunciation was identical. Normalization removes these recording-condition differences, leaving only the pronunciation-relevant patterns.

**Applied to All Features**:

We normalize:
- Mel spectrogram (each band independently)
- MFCC coefficients (each coefficient independently)
- Deltas and delta-deltas
- Energy envelope
- Spectral flux

Pitch is already normalized (to semitones relative to median), so it doesn't need z-score normalization.

## Step 10: Bringing It All Together - Feature Matrices

Now we have, for each frame (10ms of audio):
- 80 mel bands (perceptual frequency content)
- 13 MFCC coefficients (vocal tract shape)
- 13 delta coefficients (velocity of vocal tract movement)
- 13 delta-delta coefficients (acceleration of vocal tract movement)
- 1 spectral flux value (articulation sharpness)
- 1 energy value (loudness)
- 1 pitch value (intonation)

Total: 142 numbers per frame.

These are stored in efficient `ndarray` matrices (see `src/pronunciation/mod.rs`, lines 63-74):

```rust
PronunciationFeatures {
    frame_count: usize,
    mel_bands: usize,
    mel_spectrogram: Array2<f32>,     // 80 × frames
    spectral_flux: Array1<f32>,       // frames
    energy: Array1<f32>,              // frames
    mfcc: Array2<f32>,                // 13 × frames
    deltas: Array2<f32>,              // 13 × frames
    delta_deltas: Array2<f32>,        // 13 × frames
    pitch_contour: Array1<f32>,       // frames
}
```

## Step 11: Using Features for Alignment

Finally, we use these features to compare reference and learner audio via Dynamic Time Warping (see `src/pronunciation/alignment/mod.rs`).

**The Alignment Cost Function** (lines 129-156):

For each pair of frames (one from reference, one from learner), we compute a distance:

```rust
cost = mfcc_distance * 0.30
     + delta_distance * 0.15
     + delta_delta_distance * 0.05
     + mel_distance * 0.10
     + energy_distance * 0.15
     + flux_distance * 0.05
     + pitch_distance * 0.20
```

**Why These Weights?**

- **MFCC (30%)**: Highest weight because it directly captures articulation (vocal tract shape)
- **Pitch (20%)**: Second highest because intonation is crucial for natural speech
- **Delta (15%)**: Articulation dynamics matter
- **Energy (15%)**: Stress and emphasis patterns matter
- **Mel (10%)**: Redundant with MFCC but provides additional spectral detail
- **Delta-delta (5%)**: Fine temporal dynamics, less critical than velocity
- **Flux (5%)**: Onset detection, specialized role

These weights are configurable (see `assets/config/alignment_weights.json`) because different languages and learning contexts might prioritize different aspects.

**What Alignment Achieves**:

DTW finds the optimal way to stretch/compress the learner audio to match reference timing. Even if you spoke faster or slower, DTW finds which of your frames corresponds to which reference frames.

Once aligned, we can compute:
- **Timing scores**: How much did your timing deviate?
- **Articulation scores**: How similar were your MFCCs and deltas?
- **Intonation scores**: How similar were your pitch contours?

## The Complete Picture: From Sound Waves to Pronunciation Feedback

Let's trace the complete journey:

1. **Sound waves** → Microphone
2. **Analog signal** → Audio driver (analog-to-digital conversion)
3. **Digital samples** (various formats/rates) → Resampling → **Mono f32 @ 16kHz**
4. **Time-domain samples** → STFT → **Frequency-domain frames**
5. **Linear frequency bins** → Mel filterbank → **Perceptual frequency bands**
6. **Mel spectrogram** → DCT → **MFCC coefficients**
7. **MFCCs** → Differentiation → **Deltas and delta-deltas**
8. **Magnitude spectrogram** → Frame differences → **Spectral flux**
9. **Power spectrogram** → Sum and square root → **Energy envelope**
10. **Time-domain samples** → PYIN → **Pitch contour**
11. **All features** → Z-score normalization → **Normalized features**
12. **Reference & learner features** → DTW alignment → **Frame correspondence**
13. **Alignment** → Distance computation → **Pronunciation scores**
14. **Scores** → UI → **Visual feedback**

Each step transforms the data into a representation that's more useful for analyzing pronunciation. Raw samples → frequencies → perceptual frequencies → vocal tract shape → temporal dynamics → alignment → scores.

## Performance: Meeting Real-Time Requirements

The entire pipeline, from 1 second of audio to features, takes about 70ms on a modern CPU:
- STFT: ~10ms
- Mel spectrogram: ~5ms
- MFCC: ~2ms
- Deltas: ~1ms
- Spectral flux: ~1ms
- Energy: ~1ms
- Pitch (PYIN): ~50ms (the bottleneck, which is why we optimize it)

This leaves 130ms in our 200ms budget for alignment (20ms) and overhead (110ms), making real-time feedback possible.

## Conclusion: Signal Processing as Translation

Signal processing translates audio from its raw form (numbers representing air pressure over time) into a form that captures linguistic meaning (vocal tract configurations, temporal dynamics, intonation patterns).

Each step in the pipeline:
- Removes irrelevant variation (recording volume, microphone characteristics)
- Emphasizes linguistically relevant information (articulation, timing, prosody)
- Transforms to a representation suitable for comparison

The result is a system that can judge pronunciation quality with reasonable accuracy, providing learners with immediate, detailed feedback about their speech production.

When you explore the codebase, you'll see these concepts implemented in three main locations:
- Feature extraction: `src/pronunciation/features/` (mel.rs, statistics.rs, contour.rs)
- Alignment: `src/pronunciation/alignment/mod.rs`
- Scoring: `src/pronunciation/metrics/mod.rs`

Understanding the signal processing pipeline is key to understanding why the code is structured the way it is, and how the system achieves real-time pronunciation analysis.
