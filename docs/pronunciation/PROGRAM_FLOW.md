# Understanding the Pronunciation Binary: A Tutorial

This tutorial walks you through how the pronunciation training application works, from the moment you run the command to how it processes audio in real-time. By the end, you'll understand the complete flow of data and control through the system.

## Starting the Journey: What Happens When You Run the Command

When you type `cargo run --bin pronunciation session --reference example.wav`, you're starting a sophisticated real-time audio processing system. Let's follow the journey from command-line to live feedback.

### The Entry Point

The program begins in `src/bin/pronunciation.rs`. Think of this as the front door to the entire application. The first thing it does is set up logging (so we can see what's happening), then it hands control to `clap`, a command-line parsing library that understands how to turn your arguments into usable configuration.

The CLI structure is defined in `src/pronunciation/cli.rs` (lines 7-72). The design is simple: there's one main command called `session` that launches the interactive pronunciation training. You must provide a reference audio file (the "correct" pronunciation you're trying to match), and you can optionally configure things like which microphone to use, how much latency to tolerate, and what sample rate to target.

Here's what's interesting about the design: the CLI separates concerns cleanly. There's `CaptureArgs` for microphone settings, `PipelineArgs` that brings together the reference file and capture settings, and `SessionArgs` that wraps it all up. This separation means you could potentially add other commands later (like a batch processing mode) without changing the core pipeline configuration.

### Building the Configuration

Once `clap` has parsed your arguments, the program calls `build_session_config()` (lines 22-68 in `src/bin/pronunciation.rs`). This function is doing more than it might appear at first glance.

First, it resolves where the application's assets live. By default, this is the `assets/` directory next to the binary, but you can override it with `--assets-path`. This matters because the application needs to load alignment weights - numerical values that control how much weight to give different audio features when comparing your pronunciation to the reference.

These alignment weights come from `assets/config/alignment_weights.json`. Think of them as the "importance ratings" for different aspects of pronunciation: how much should we care about MFCC coefficients (which capture the shape of your vocal tract) versus pitch (which captures intonation)? These weights are configurable because different languages and learning contexts might prioritize different aspects.

Next, it sets up capture settings. This is where we configure how to read from your microphone. The latency range (default 100-200ms) is particularly important - it controls the buffer size for audio capture. Smaller buffers mean lower latency (faster feedback) but risk audio dropouts if the system can't keep up. Larger buffers are more stable but slower to respond.

Finally, it creates a `SessionConfig` that bundles everything together: the reference file path, assets location, capture settings, and alignment weights. This config object is the complete blueprint for the session that's about to run.

## Launching the Runtime: Threading and Initialization

With configuration in hand, the program calls `run_session(config)` which creates a `SessionRuntime`. This is where things get interesting from an architecture perspective.

### Why a Separate Thread?

The runtime creates a new thread (lines 78-175 in `src/pronunciation/session.rs`) called "session-runtime". Why? Because audio processing has very different timing requirements than UI rendering.

The UI needs to respond to user clicks, repaint windows at 60fps, and stay responsive. Audio processing needs to pull samples from the microphone every 10-20 milliseconds, extract features, perform alignment, and generate feedback - all without dropping samples or introducing audible glitches.

If we tried to do both on the same thread, the UI would freeze during heavy processing, or worse, audio processing would be delayed by UI repaints. By separating them, each can run at its own pace.

The threads communicate via channels - thread-safe message queues that Rust provides. Commands flow from the UI thread to the runtime thread (start recording, stop, replay reference), and snapshots flow back from the runtime to the UI (here's the latest alignment, here are the scores).

### The Initialization Dance

When the runtime thread starts, it doesn't immediately start recording. Instead, it goes through a careful initialization sequence:

1. **Send Initial Snapshot**: The first thing it does is send a snapshot to the UI saying "I'm initializing". This is crucial for user experience - the UI can show immediately, displaying "Loading..." rather than appearing frozen.

2. **Load Reference Audio**: Using `symphonia` (a Rust audio decoding library), it loads the reference WAV file and converts it to mono f32 samples. This happens in `load_clip()` in `src/pronunciation/mod.rs` (lines 301-313). The function is careful about error handling - if the file doesn't exist or can't be decoded, it returns a clear error that gets displayed in the UI.

3. **Resample if Needed**: The system works at 16kHz internally (16,000 samples per second). This is a sweet spot for speech processing - high enough to capture all the important frequencies in human speech (which mostly lives below 8kHz due to the Nyquist limit), but low enough to process efficiently. If your reference file is at a different sample rate (say, 44.1kHz or 48kHz), it gets resampled using linear interpolation.

4. **Extract Reference Features**: This is the expensive part. The system computes a mel spectrogram (more on this in the signal processing tutorial), MFCC coefficients, pitch contour, energy, and spectral flux from the reference audio. This can take several seconds for a long reference file. The good news is it only happens once - these reference features are computed at startup and then reused for every comparison.

5. **Setup Capture Source**: The system creates a `LiveCaptureSource` that will later open the microphone. It's not opened yet (that happens when you click "Start"), but it's ready to go.

6. **Compute Reference Alignment**: Before showing the UI, it creates a baseline visualization showing the reference audio's energy and pitch contours. This gives you something to look at before you start recording.

7. **Send Ready Snapshot**: Finally, it sends a snapshot marked as no longer initializing, and the UI switches from "Loading..." to "Ready".

## The Main Event Loop: Processing Commands and Audio

Now the runtime thread enters its main loop (lines 683-851). This loop is the heart of the system.

### Understanding the Loop Structure

The loop processes two types of events: commands from the UI and audio chunks from the microphone. Here's the clever part: it does this in a non-blocking way.

When processing commands, it uses `try_recv()` which checks if there's a command waiting but doesn't block if there isn't. When polling for audio, it uses `recv_timeout()` with a 20ms timeout. This means every 20ms, the loop wakes up, checks for commands, checks for audio, processes what's available, and goes back to waiting.

Why 20ms? It's a balance. Shorter timeouts would waste CPU constantly waking up when there's nothing to do. Longer timeouts would make the system less responsive to commands. 20ms means the system responds to button clicks within 20ms (imperceptibly fast) while spending most of its time sleeping when idle.

### The Start Command: Kicking Off Recording

When you click "Start" in the UI, it sends a `Start` command to the runtime thread. The handler (lines 761-795) does three things:

1. **Start Capture**: Calls `engine.start()` which opens the microphone stream. The `cpal` library (cross-platform audio library) creates a background thread that continuously pulls samples from the audio driver and sends them over a channel. From this moment on, audio chunks are arriving every few milliseconds.

2. **Start Playback**: Creates a `ReferencePlayer` and tells it to play the reference audio through the speakers. This is implemented using `rodio`, which creates its own playback thread. The playback happens independently - the system isn't waiting for it to finish.

3. **Enter Drive Loop**: Transitions to the drive loop, which is where real-time processing happens.

### The Drive Loop: Real-Time Processing

The drive loop (lines 797-851) is simpler than you might expect. Every iteration does two things:

1. **Check for commands**: Maybe the user clicked "Stop" or "Replay". Handle that immediately.

2. **Poll for audio**: Ask the capture source "do you have any audio chunks for me?"

If there's audio, it calls `engine.poll()` which does the real work.

### Processing Audio Chunks

When `engine.poll()` receives an audio chunk (lines 403-432), here's what happens:

**Resampling**: The chunk arrives at whatever sample rate the microphone provides (often 44.1kHz or 48kHz). It gets resampled to 16kHz using linear interpolation. This is fast - just a few microseconds.

**Buffer Management**: The resampled chunk is appended to the learner buffer. This buffer is a sliding window that holds the last few seconds of audio. Why not just process each chunk independently? Because features need context. A single 20ms chunk doesn't tell you much about pronunciation - you need to see how phonemes transition, how pitch changes over time.

The buffer has a maximum size (reference length + 0.5 seconds). Once it exceeds this, the oldest samples are dropped. This keeps memory usage bounded and focuses on recent audio.

**Minimum Sample Check**: Before doing expensive feature extraction, it checks if there are at least 1600 samples (0.1 seconds). This threshold ensures there's enough audio to extract meaningful features. Below this, you'd be trying to compute pitch from a snippet too short to contain even one pitch period.

**Feature Extraction**: Once there's enough audio, it extracts the same features computed from the reference: mel spectrogram, MFCC, pitch contour, energy, and spectral flux. This is the expensive part - typically 50-100ms for 1-2 seconds of audio. The signal processing tutorial explains what each of these features captures.

**Alignment**: The extracted features are compared to the reference features using dynamic time warping (DTW). Think of DTW as finding the best way to "stretch" or "compress" your audio to match the reference timing. It finds which frame of your audio corresponds to which frame of the reference, even if you spoke faster or slower.

**Scoring**: From the alignment, it calculates pronunciation scores. How well did your timing match? How similar was your articulation? How close was your intonation? These scores (0 to 1, with 1 being perfect) are computed by the metrics calculator.

**Latency Measurement**: The system measures how long processing took. If it exceeds the budget (default 200ms), a warning is logged. This is important for maintaining real-time feedback - if processing is too slow, the feedback arrives too late to be useful.

**Snapshot Creation**: All this information (alignment, scores, latency) is packaged into a `SessionSnapshot` and sent to the UI. The UI receives this snapshot and updates the display - waveforms, pitch contours, phoneme timeline, scores.

### The Beautiful Part: Continuous Updates

Here's what makes this feel real-time: this entire process happens continuously while you're speaking. Audio chunks arrive every 20ms. Every 100-200ms, there's enough new audio to trigger feature extraction and alignment. Snapshots flow to the UI at 5-10 Hz, making the visualizations feel live and responsive.

The user sees their waveform building up, pitch contour tracing out, phoneme segments appearing and turning green/yellow/red based on quality. It feels like instant feedback, even though there's actually a 100-200ms processing delay.

## State Management: Keeping Everything Synchronized

The system maintains state through snapshots. Each snapshot is immutable (it never changes after creation) and contains everything the UI needs to render:

- **Alignment Data**: Which frames matched up, timing deltas, similarity scores
- **Pitch Contours**: Both reference and learner pitch over time
- **Energy Envelopes**: Both reference and learner energy over time  
- **Phoneme Segments**: Time ranges with quality scores
- **Overall Scores**: Timing, articulation, intonation, overall
- **Status Flags**: Recording or not, reference playing or not
- **Latency Info**: Current processing latency
- **Error Messages**: If anything went wrong

The UI never modifies this data. It just renders it. When new audio is processed, a new snapshot is created and sent. The UI receives it, drops the old snapshot, and re-renders with the new one.

This design (called "unidirectional data flow") makes the system much easier to reason about. There's no shared mutable state between threads, no locks or mutexes needed. Just immutable snapshots flowing from runtime to UI.

## Error Handling: Graceful Degradation

Audio systems are notorious for platform-specific issues. Microphones can disconnect, sample rates might not be supported, files might be corrupt. The system handles errors at multiple levels:

**Initialization Errors**: If loading the reference file fails, or the alignment weights file is missing, or feature extraction fails, the error is captured and sent in a snapshot. The UI displays it clearly and doesn't proceed to the session.

**Runtime Errors**: If the microphone stream fails (maybe the microphone was unplugged), or feature extraction throws an error (maybe there's weird audio), the error is captured in a snapshot. Recording stops automatically, the error is displayed, and the user can try starting again.

**Latency Warnings**: If processing is too slow, a warning appears in the UI but recording continues. This isn't a fatal error - feedback will just be delayed slightly. The user can decide whether to continue or adjust settings.

**Graceful Shutdown**: When the user closes the UI, a `Shutdown` command is sent. The runtime stops recording, closes the microphone, waits for all pending processing to complete, and then exits cleanly. No hanging threads, no resource leaks.

## Data Flow Summary: Following the Audio

Let's trace one piece of audio through the entire system:

1. You speak into the microphone
2. Audio driver captures samples (background thread)
3. `cpal` sends chunk over channel (20ms of audio)
4. Runtime thread receives chunk in `poll()`
5. Chunk is resampled to 16kHz
6. Chunk is appended to learner buffer
7. When buffer is big enough, features are extracted
8. Features are aligned with reference features via DTW
9. Scores are calculated from alignment
10. Snapshot is created with all this info
11. Snapshot is sent over channel to UI thread
12. UI receives snapshot and re-renders
13. You see updated waveforms, pitch curves, scores

From sound waves hitting your microphone to pixels on your screen: roughly 150ms total latency. Fast enough to feel real-time.

## Performance Considerations: Why It's Fast Enough

Real-time audio processing is challenging. Here's how the system stays fast:

**Efficient Libraries**: The `aus` library (used for signal processing) is optimized. The pitch estimation algorithm (PYIN) is compiled with optimization level 3 even in debug builds. This makes development faster without sacrificing performance.

**Smart Buffering**: By requiring a minimum of 0.1 seconds before processing, and processing 1-2 seconds at a time, the system amortizes the cost of feature extraction over multiple frames. It's more efficient to process 2 seconds once than 0.1 seconds twenty times.

**Sliding Window**: The learner buffer is bounded. It doesn't grow forever, so memory usage stays constant regardless of how long you record.

**Thread Separation**: Audio capture runs in its own thread (managed by `cpal`). Feature extraction runs in the runtime thread. UI rendering runs in the main thread. Playback runs in its own thread (managed by `rodio`). No thread blocks another.

**Low-Latency Channels**: Rust's `mpsc` channels are lock-free and very fast. Sending a snapshot takes microseconds, not milliseconds.

The result: on a modern CPU, the system can process audio faster than real-time. A 1-second reference typically takes 70ms to process (0.07x real-time). This leaves plenty of headroom for the 200ms latency budget.

## Conclusion: A Well-Architected System

The pronunciation binary is a good example of separating concerns in a real-time system:

- **CLI parsing** is separate from **session logic**
- **Configuration** is separate from **execution**
- **Audio capture** is separate from **processing**
- **Processing** is separate from **UI rendering**
- **Commands** flow one direction, **snapshots** flow the other
- **Errors** are captured and displayed, not crashed
- **Latency** is measured and managed

This architecture makes the system maintainable, testable, and extensible. Want to add a new feature? Extract it in the feature extraction step. Want a new score? Calculate it in the metrics step. Want a new visualization? Subscribe to snapshots in the UI.

Understanding this flow is the foundation for understanding how signal processing (covered in another tutorial) and library usage (covered in another tutorial) fit together to create a complete pronunciation training system.
