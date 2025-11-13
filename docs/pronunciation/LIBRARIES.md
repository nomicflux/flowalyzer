# Understanding Audio and Signal Processing Libraries: A Tutorial

If you're new to Rust audio programming or signal processing, this tutorial will help you understand the libraries used in the pronunciation training application. We'll explore what each library does, why it's needed, and how to use it effectively.

## The Challenge: Reading, Processing, and Playing Audio in Rust

Audio programming has several distinct challenges, and no single library solves them all. Let's understand what we need:

1. **Reading audio files** in many different formats (MP3, WAV, FLAC, OGG)
2. **Capturing audio** from a microphone in real-time
3. **Playing audio** through speakers
4. **Processing audio** mathematically (Fourier transforms, spectrograms, pitch detection)
5. **Storing and manipulating** numerical arrays efficiently

Each of these requires specialized knowledge and platform-specific code. That's where libraries come in - they handle the complexity so we can focus on our application logic.

## Audio Input/Output: The Foundation

### symphonia: The Universal Audio Decoder

**What problem does it solve?**

Audio files come in many formats. MP3 uses lossy compression with complex psychoacoustic models. FLAC uses lossless compression. WAV stores raw PCM samples. OGG Vorbis uses a different lossy algorithm. Each format requires a different decoder.

Writing decoders for all these formats would take years. Worse, audio files can store samples in many different numeric formats: 8-bit integers, 16-bit integers, 24-bit integers, 32-bit integers, unsigned or signed, floating point. They can be mono or stereo or surround sound.

**How symphonia helps:**

Symphonia is a pure Rust audio decoding library that handles all common formats. You give it a file path, and it gives you back samples as f32 numbers between -1.0 and 1.0. It handles format detection, sample conversion, and channel mixing automatically.

**How we use it:**

Look at `src/audio/decoder.rs` (lines 13-94). The `decode_audio()` function is surprisingly short considering what it does:

```rust
let file = std::fs::File::open(path)?;
let mss = MediaSourceStream::new(Box::new(file), Default::default());
let probe_result = symphonia::default::get_probe().format(&hint, mss, ...)?;
```

This probes the file to determine its format. Symphonia looks at the file extension and magic bytes to figure out whether it's MP3, WAV, etc.

```rust
let track = format.tracks().iter()
    .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;
let mut decoder = symphonia::default::get_codecs()
    .make(&track.codec_params, &DecoderOptions::default())?;
```

This creates the appropriate decoder for the detected format. The factory pattern (`get_codecs().make()`) means the same code works for all formats.

```rust
loop {
    let packet = match format.next_packet() { ... };
    let decoded = decoder.decode(&packet)?;
    let mono_samples = convert_to_mono_f32(&decoded);
    all_samples.extend(mono_samples);
}
```

This loops through packets (chunks of compressed audio), decodes them, converts to our standard format (mono f32), and accumulates them.

The `convert_to_mono_f32()` function (lines 96-268) handles all the different sample formats. Notice how it has a match statement with branches for S8, S16, S24, S32, U8, U16, U24, U32, F32, F64. Each branch knows the right conversion formula for that format. For signed 16-bit:

```rust
AudioBufferRef::S16(buf) => {
    let scale = 1.0 / 32768.0;  // Convert to [-1.0, 1.0]
    mono_samples.extend(buf.chan(0).iter().map(|&s| s as f32 * scale));
}
```

Dividing by 32768 converts the range [-32768, 32767] to approximately [-1.0, 1.0].

**Why this matters for pronunciation:**

The reference audio you provide might be in any format. Symphonia means we can accept WAV files from voice recorders, MP3 files from downloads, FLAC files from audio workstations - whatever the user has. They all get normalized to the same internal format.

### hound: Writing WAV Files

**What problem does it solve?**

We need to write audio files too. For example, the main `flowalyzer` application writes processed chunks to disk. We could write raw bytes, but then we'd need to write WAV headers, handle byte order, manage the chunk structure. It's tedious and error-prone.

**How hound helps:**

Hound is a simple WAV encoder. You tell it the sample rate, number of channels, and bit depth, then write samples one at a time. It handles the WAV format internals.

**How we use it:**

Look at `src/audio/encoder.rs` (lines 6-37):

```rust
let spec = hound::WavSpec {
    channels: 1,
    sample_rate: audio.sample_rate,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
};
let mut writer = hound::WavWriter::create(path, spec)?;
```

This creates a writer configured for mono, 16-bit integer samples at whatever sample rate the audio has.

```rust
for &sample in &audio.samples {
    let clamped = sample.clamp(-1.0, 1.0);
    let i16_sample = (clamped * 32767.0) as i16;
    writer.write_sample(i16_sample)?;
}
```

This converts our internal f32 format back to 16-bit integers. We clamp to ensure no value exceeds [-1.0, 1.0] (which would cause distortion), then scale by 32767 and cast to i16.

**Why 16-bit?**

16-bit audio has a dynamic range of about 96 dB, which is more than sufficient for speech. Using 16-bit (vs 32-bit float) makes files smaller. And 16-bit integer is universally supported by all audio software.

### cpal: Cross-Platform Audio Capture

**What problem does it solve?**

Capturing audio from a microphone is intensely platform-specific. On Windows, you use WASAPI or DirectSound. On macOS, you use CoreAudio. On Linux, you use ALSA or PulseAudio or JACK or PipeWire. Each has different APIs, different threading models, different sample format support.

**How cpal helps:**

CPAL (Cross-Platform Audio Library) provides a unified Rust API that works on all platforms. Behind the scenes, it uses the platform's native audio API, but you write the same code regardless of platform.

**How we use it:**

Look at `src/audio/capture.rs` (lines 169-212) for the `LiveCapture` implementation:

```rust
let device = select_device(config)?;
```

This finds either the default microphone or the one the user specified by name. On macOS, it queries CoreAudio. On Windows, it queries WASAPI. But the code is identical.

```rust
let supported = device.default_input_config()?;
let stream_config = StreamConfig {
    channels: supported.channels(),
    sample_rate: supported.sample_rate(),
    buffer_size: BufferSize::Default,
};
```

This queries what the device supports. Some microphones only support 48kHz, others only 44.1kHz. Some are mono, others stereo. We ask the device what it can do and configure accordingly.

```rust
let stream = build_input_stream(device, &stream_config, 
                                 supported.sample_format(), 
                                 sender, finished)?;
stream.play()?;
```

This builds and starts the capture stream. From this moment, audio samples are arriving. But here's the tricky part: they arrive on a background thread managed by the audio driver, and we need to safely transfer them to our processing thread.

Look at the callback functions (lines 214-248):

```rust
fn emit_chunk_f32(data: &[f32], channels: usize, 
                  sender: &Arc<SyncSender<Vec<f32>>>, 
                  finished: &Arc<AtomicBool>) {
    if finished.load(Ordering::Relaxed) || channels == 0 {
        return;
    }
    let mut mono = Vec::with_capacity(data.len() / channels);
    for frame in data.chunks(channels) {
        mono.push(mix_to_mono(frame));
    }
    let _ = sender.try_send(mono);
}
```

This callback runs on the audio driver's background thread. It receives interleaved multi-channel audio (L R L R L R for stereo) and:

1. Checks if we're finished (no point processing audio after shutdown)
2. Mixes to mono by averaging channels
3. Sends over a channel (thread-safe queue) to the processing thread

Notice `try_send` instead of `send`. If the processing thread can't keep up, we drop samples rather than blocking. Blocking the audio thread causes audible glitches or crashes.

**Why this matters for pronunciation:**

Real-time feedback requires real-time capture. CPAL gives us low-latency access to microphone audio (typically 100-200ms from when sound enters the microphone to when samples arrive in our code). This makes the "speak and see immediate feedback" experience possible.

### rodio: Audio Playback

**What problem does it solve?**

Playing audio has similar platform challenges to capture. Plus, you often want to decode audio formats on the fly, adjust volume, mix multiple sources, handle playback asynchronously.

**How rodio helps:**

Rodio builds on CPAL to provide high-level playback features. You give it samples (or a decoder that produces samples), and it handles playback on a background thread.

**How we use it:**

Look at `src/audio/playback.rs` (lines 19-36):

```rust
pub fn play_audio(data: &AudioData) -> Result<()> {
    let stereo = duplicate_to_stereo(&data.samples);
    let buffer = SamplesBuffer::new(2, data.sample_rate, stereo);
    play_source(buffer)
}
```

Our internal audio is mono, but most systems work better with stereo (two channels). The `duplicate_to_stereo` function (lines 50-57) simply duplicates each sample:

```rust
pub fn duplicate_to_stereo(samples: &[f32]) -> Vec<f32> {
    let mut output = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        output.push(sample);
        output.push(sample);
    }
    output
}
```

This is efficient - no resampling or processing, just copying samples.

```rust
fn play_source<S>(source: S) -> Result<()> {
    let (_stream, handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&handle)?;
    sink.append(unified);
    sink.sleep_until_end();
    Ok(())
}
```

This creates an output stream (connection to speakers), creates a "sink" (playback manager), appends our audio to it, and waits until playback finishes.

For the pronunciation session, there's a more sophisticated usage in `src/pronunciation/session.rs` (lines 901-939). The `ReferencePlayer` keeps the stream and sink alive so playback can be started and stopped on demand without recreating everything.

**Why this matters for pronunciation:**

When you start a recording session, the reference audio plays through your speakers simultaneously. This lets you hear what you're trying to match while you speak. Rodio handles this playback asynchronously - it doesn't block the capture or processing threads.

## Signal Processing: Extracting Meaning from Audio

Now we have audio samples flowing in and out. But raw samples are just numbers - we need to extract meaningful features. This is where signal processing libraries come in.

### aus: The Signal Processing Toolkit

**What problem does it solve?**

Signal processing requires sophisticated mathematical operations:
- Fourier transforms (converting time domain to frequency domain)
- Mel-scale filterbanks (converting linear frequency to perceptual scales)
- MFCC computation (extracting spectral envelope)
- Pitch estimation (finding fundamental frequency)

Implementing these from scratch requires deep DSP knowledge and careful optimization (FFT algorithms, for instance, need special care to be fast).

**How aus helps:**

AUS (Audio Signal) is a Rust library providing signal analysis primitives. It wraps well-tested algorithms in an ergonomic Rust API.

**How we use it for spectrograms:**

Look at `src/pronunciation/features/mel.rs` (lines 44-59):

```rust
let fft_size = ((TARGET_SAMPLE_RATE as usize * WINDOW_MS) / 1000).max(1);
let hop_size = ((TARGET_SAMPLE_RATE as usize * HOP_MS) / 1000).max(1);
let stft = spectrum::rstft(&audio_f64, fft_size, hop_size, WindowType::Hanning);
```

This computes the Short-Time Fourier Transform. Let's break down why each parameter matters:

**FFT Size (400 samples = 25ms at 16kHz)**: This is the window size. Larger windows give better frequency resolution but worse time resolution. For speech, 25ms is a sweet spot - long enough to capture several pitch periods (important for pitch detection) but short enough to see consonants and transitions.

**Hop Size (160 samples = 10ms)**: This is how far we slide the window each frame. Smaller hops give smoother temporal resolution. 10ms means 100 frames per second, which is plenty for speech analysis.

**Hanning Window**: This is a window function applied before FFT. It tapers the window edges to zero, reducing spectral leakage (artificial frequencies that appear due to discontinuities at window boundaries).

The STFT returns a 2D array: [frames x frequency_bins]. Each cell is a complex number representing magnitude and phase at that time and frequency.

```rust
let (magnitude, _) = spectrum::complex_to_polar_rstft(&stft);
let power = analysis::make_power_spectrogram(&magnitude);
```

We convert to magnitude (|STFT|) and power (|STFT|²). Power emphasizes differences, which is useful for mel spectrogram computation.

**How we use it for mel spectrograms:**

```rust
let freqs = spectrum::rfftfreq(fft_size, TARGET_SAMPLE_RATE);
let filterbank = MelFilterbank::new(MIN_FREQ, 
                                    (TARGET_SAMPLE_RATE as f64) / 2.0,
                                    MEL_BANDS, &freqs, true);
let mel = analysis::mel::make_mel_spectrogram(&power, &filterbank);
```

The mel filterbank converts linear frequency (Hz) to mel scale. Why? Human hearing is not linear. We're better at distinguishing 100Hz from 200Hz (a difference of 100Hz) than distinguishing 5000Hz from 5100Hz (also 100Hz difference). Mel scale accounts for this.

The filterbank creates triangular filters at mel-spaced frequencies. Each filter sums the power in a frequency band. This reduces dimensionality (201 frequency bins → 80 mel bands) while retaining perceptually important information.

**How we use it for MFCC:**

Look at `src/pronunciation/features/statistics.rs` (line 27):

```rust
let mfcc_raw = analysis::mel::mfcc_spectrogram(mel_spectrogram, MFCC_COUNT, None);
```

MFCC (Mel-Frequency Cepstral Coefficients) is computed by:
1. Taking the logarithm of the mel spectrogram (compresses dynamic range)
2. Applying Discrete Cosine Transform (DCT)
3. Keeping the first 13 coefficients

Why? MFCC captures the spectral envelope - the overall shape of the spectrum. This shape is determined by your vocal tract (shape of mouth, tongue position, throat). It's less sensitive to pitch (which varies even for the same sound) and more sensitive to articulation (which is what we want to measure).

**How we use it for pitch estimation:**

Look at `src/pronunciation/features/contour.rs` (lines 57-63):

```rust
let (_timestamps, pitches, voiced_flags, _confidence) = analysis::pyin_pitch_estimator(
    &audio,
    TARGET_SAMPLE_RATE,
    FREQ_MIN,
    FREQ_MAX,
    frame_len,
);
```

PYIN (Probabilistic YIN) is a sophisticated pitch estimation algorithm. It:
1. Computes autocorrelation (how similar is the signal to a shifted version of itself?)
2. Uses cumulative mean normalized difference (a trick to make autocorrelation more robust)
3. Applies probabilistic interpretation (estimates confidence for each pitch candidate)
4. Selects the most likely pitch

The result is pitch values in Hz, voiced flags (is this frame voiced speech or unvoiced/silent?), and confidence scores.

**Why this matters for pronunciation:**

These features capture different aspects of pronunciation:
- **Mel spectrogram**: Overall spectral content
- **MFCC**: Vocal tract shape (articulation)
- **Pitch**: Fundamental frequency (intonation)

By computing these features from both reference and learner audio, we can compare them quantitatively. The alignment algorithm (covered in another tutorial) uses these features to determine "how similar is your pronunciation to the reference?"

**Performance note:**

AUS's pitch estimation can be slow. That's why in `Cargo.toml` (lines 34-38), we optimize the `aus` and `pyin` packages even in debug builds:

```toml
[profile.dev.package.aus]
opt-level = 3

[profile.dev.package.pyin]
opt-level = 3
```

This makes development iteration faster (most code is unoptimized and compiles quickly) while keeping audio processing fast (these hot functions are optimized).

### ndarray: Numerical Arrays for Features

**What problem does it solve?**

We're working with multi-dimensional numerical data:
- MFCC: 13 coefficients × N frames
- Mel spectrogram: 80 bands × N frames
- Energy: N values (one per frame)

We could use `Vec<Vec<f32>>`, but that's inefficient (pointer indirection, cache misses) and awkward (no built-in linear algebra operations).

**How ndarray helps:**

Ndarray provides NumPy-like arrays for Rust: efficient contiguous storage, zero-cost views, mathematical operations, and multidimensional indexing.

**How we use it:**

Look at `src/pronunciation/features/statistics.rs` (lines 49-60):

```rust
fn array_from_vec2(data: &[Vec<f64>]) -> Array2<f32> {
    let rows = data.len();
    let cols = data[0].len();
    let mut flat = Vec::with_capacity(rows * cols);
    for row in data {
        flat.extend(row.iter().map(|v| *v as f32));
    }
    Array2::from_shape_vec((rows, cols), flat).expect("valid mel dimensions")
}
```

This converts the `Vec<Vec<f64>>` returned by AUS into an `Array2<f32>` (2D array of f32). All data is stored contiguously in memory, which is cache-friendly and enables SIMD operations.

For alignment (see `src/pronunciation/alignment/mod.rs` line 136-146), we can efficiently access rows:

```rust
let frame_mfcc = reference.mfcc.row(row);
let learner_mfcc = learner.mfcc.row(col);
```

This creates views (no copying) into the array. We can then compute distances:

```rust
fn mean_abs(lhs: ArrayView1<'_, f32>, rhs: ArrayView1<'_, f32>) -> f32 {
    lhs.iter()
        .zip(rhs.iter())
        .map(|(a, b)| (a - b).abs())
        .sum::<f32>()
        / lhs.len() as f32
}
```

This computes mean absolute difference efficiently. The compiler can vectorize this loop (process multiple elements per CPU instruction) because ndarray guarantees contiguous storage.

**Why this matters for pronunciation:**

Feature matrices can be large (seconds of audio × many coefficients). Efficient storage and computation is crucial for meeting the 200ms latency budget.

## Specialized Libraries

### ssstretch: Pitch-Preserving Time Stretching

**What problem does it solve?**

The main flowalyzer application needs to slow down or speed up audio without changing pitch. Naive approaches (resampling) change both speed and pitch. Sophisticated approaches (phase vocoder) can separate the two, but are complex to implement correctly.

**How ssstretch helps:**

SSStretch (Signalsmith Stretch) is a high-quality time-stretching algorithm. It uses a phase vocoder with phase locking, producing natural-sounding results.

**How we use it:**

Look at `src/operations/speed.rs` (lines 37-52):

```rust
pub fn change_speed(chunk: &AudioChunk, speed_factor: f32) -> AudioChunk {
    if is_identity_speed(speed_factor) {
        return chunk.clone();
    }
    let mut stretch = configured_stretch(chunk.sample_rate);
    let samples = collect_stretched_samples(&mut stretch, &chunk.samples, speed_factor);
    ...
}
```

The usage is straightforward: create a stretcher, configure it for mono at the appropriate sample rate, set the tempo factor, and process the audio.

```rust
fn configured_stretch(sample_rate: u32) -> Stretch {
    let mut stretch = Stretch::new();
    stretch.preset_default(1, sample_rate as f32);
    stretch
}
```

The stretcher maintains internal state (overlap-add buffers, phase information) which is why it's created as a mutable object.

**Note**: This library requires a C++ compiler because it wraps the Signalsmith Stretch C++ library. This is fine - the quality is worth the extra build complexity.

### whisper-rs: Speech Recognition

**What problem does it solve?**

The main flowalyzer application needs to transcribe audio to text with word-level timing. This enables linguistic boundary detection (splitting audio at natural phrase boundaries rather than arbitrary time intervals).

**How whisper-rs helps:**

Whisper is a state-of-the-art speech recognition model from OpenAI. Whisper-rs provides Rust bindings to the C++ implementation, letting us use it from Rust.

**How we use it:**

Look at `src/transcription/mod.rs`. The basic flow:

```rust
let ctx = WhisperContext::new(&model_path, WhisperContextParameters::default())?;
let mut state = ctx.create_state()?;
state.full_transcribe(&params, &audio_samples)?;
for segment in state.as_iter() {
    let text = segment.to_str();
    let start = segment.start_timestamp() / 100.0;
    let end = segment.end_timestamp() / 100.0;
}
```

1. Load the model (a large file, typically 100-500MB)
2. Create a state object (holds intermediate computations)
3. Transcribe the audio
4. Iterate over segments, extracting text and timestamps

**Note**: This requires a GGML model file and cmake for building. The pronunciation tool doesn't use transcription (it works with pre-segmented reference audio), but the main flowalyzer application uses it for chunking.

## Understanding the Library Ecosystem

Here's how these libraries fit together:

**For the pronunciation tool:**
1. `symphonia` decodes the reference WAV file
2. `cpal` captures live microphone audio
3. `aus` extracts features from both
4. `ndarray` stores features efficiently
5. `rodio` plays the reference audio
6. Custom alignment code compares features
7. UI displays results

**For the main flowalyzer tool:**
1. `symphonia` decodes input audio
2. `whisper-rs` transcribes it
3. Custom chunking logic finds linguistic boundaries
4. `ssstretch` time-stretches chunks
5. `hound` writes output WAV files

## Performance and Optimization

Understanding library performance characteristics:

**Fast operations** (microseconds):
- Resampling (linear interpolation)
- Sample format conversion
- Channel mixing

**Medium operations** (milliseconds):
- STFT computation
- Mel spectrogram
- MFCC extraction
- Delta computation

**Slow operations** (tens of milliseconds):
- Pitch estimation (PYIN)
- Whisper transcription

This is why we optimize `aus` and `pyin` in development, and why we only extract features when we have enough audio buffered (amortizing cost over multiple frames).

## Conclusion: Standing on the Shoulders of Giants

These libraries represent decades of signal processing research and thousands of hours of engineering. By using them:

- We get robust, tested implementations
- We avoid platform-specific code
- We achieve good performance without low-level optimization
- We can focus on our application logic (pronunciation training) rather than audio infrastructure

The key is understanding what each library does and how they compose. Audio capture → feature extraction → comparison → scoring → playback. Each step uses the right tool for the job.

When you explore the pronunciation codebase, you'll see these libraries working together seamlessly, turning raw audio into meaningful pronunciation feedback.
