use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::audio::capture::{CaptureConfig, LiveCapture};
use crate::audio::playback::duplicate_to_stereo;
use crate::audio::resample::linear_resample;
use crate::pronunciation::session::{
    AlignmentReport, ClipVariant, PronunciationScores, RecipeApplicationProgress,
    RecipeApplicationStage, SessionConfig, SessionEngine, SessionSnapshot,
};
use crate::pronunciation::{apply_recipe_to_range, PronunciationError, RecordedClip, Result};
use crate::types::Recipe;
use rodio::{buffer::SamplesBuffer, OutputStream, Sink};

thread_local! {
    static PLAYBACK_STREAM: RefCell<Option<OutputStream>> = const { RefCell::new(None) };
}

pub enum SessionCommand {
    Start,
    Stop,
    ReplayReference,
    StopReplay,
    ApplyRecipe {
        start_sec: f64,
        end_sec: f64,
        recipe: Recipe,
    },
    ToggleClipVariant(ClipVariant),
    Shutdown,
}

pub struct SessionRuntime {
    reference_clip: Arc<RecordedClip>,
    flowalyzed_clip: Arc<Mutex<Option<RecordedClip>>>,
    active_variant: Arc<Mutex<ClipVariant>>,
    reference_playing: Arc<AtomicBool>,
    playback_state: Arc<Mutex<Option<PlaybackState>>>,
    engine: Arc<Mutex<SessionEngine>>,
    config: SessionConfig,
    snapshot_sender: Sender<SessionSnapshot>,
    command_receiver: Receiver<SessionCommand>,
    capture_buffer: Arc<Mutex<Vec<f32>>>,
}

impl SessionRuntime {
    pub fn spawn(
        reference_clip: RecordedClip,
        config: SessionConfig,
    ) -> (SessionHandle, SessionController) {
        assert!(
            !reference_clip.samples.is_empty(),
            "reference clip cannot be empty"
        );
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let reference_clip = Arc::new(reference_clip);
        let engine = SessionEngine::new(&reference_clip.samples, &config);
        let runtime = Self {
            reference_clip: reference_clip.clone(),
            flowalyzed_clip: Arc::new(Mutex::new(None)),
            active_variant: Arc::new(Mutex::new(ClipVariant::Original)),
            reference_playing: Arc::new(AtomicBool::new(false)),
            playback_state: Arc::new(Mutex::new(None)),
            engine: Arc::new(Mutex::new(engine)),
            config: config.clone(),
            snapshot_sender: snapshot_tx,
            command_receiver: command_rx,
            capture_buffer: Arc::new(Mutex::new(Vec::new())),
        };
        let handle = SessionHandle {
            snapshot_receiver: snapshot_rx,
            config: config.clone(),
        };
        let controller = SessionController {
            command_sender: command_tx,
        };
        runtime.snapshot_sender.send(runtime.build_snapshot()).ok();
        thread::spawn(move || runtime.run());
        (handle, controller)
    }

    fn run(self) {
        let mut recording = false;
        let mut capture: Option<LiveCapture> = None;
        loop {
            match self.command_receiver.try_recv() {
                Ok(SessionCommand::Start) => {
                    recording = true;
                    capture = self.start_capture().ok();
                    if let Ok(mut engine) = self.engine.lock() {
                        engine.reset();
                    }
                }
                Ok(SessionCommand::Stop) => {
                    recording = false;
                    capture = None;
                    let _ = self.snapshot_sender.send(self.build_snapshot());
                }
                Ok(SessionCommand::Shutdown) => break,
                Ok(SessionCommand::ReplayReference) => {
                    self.start_reference_playback();
                }
                Ok(SessionCommand::StopReplay) => {
                    self.stop_reference_playback();
                }
                Ok(SessionCommand::ApplyRecipe {
                    start_sec,
                    end_sec,
                    recipe,
                }) => self.handle_apply_recipe(start_sec, end_sec, recipe),
                Ok(SessionCommand::ToggleClipVariant(variant)) => {
                    self.handle_toggle_variant(variant)
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => break,
            }

            if recording {
                if let Some(ref cap) = capture {
                    self.process_capture_chunk(cap);
                }
            }

            self.poll_playback_completion();
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn start_capture(&self) -> Result<LiveCapture> {
        let mut capture_config = CaptureConfig::new();
        capture_config.sample_rate = self.config.sample_rate;
        capture_config.latency_ms = self.config.latency_range.clone();
        LiveCapture::start(&capture_config).map_err(|err| PronunciationError::new(err.to_string()))
    }

    fn process_capture_chunk(&self, capture: &LiveCapture) {
        let timeout = Duration::from_millis(self.config.chunk_duration_ms as u64);
        let target_len =
            (self.config.sample_rate as usize * self.config.chunk_duration_ms as usize) / 1_000;
        let incoming_rate = capture.sample_rate();
        let mut buffer = self.capture_buffer.lock().unwrap();
        while let Some(chunk) = capture.recv_chunk(timeout) {
            buffer.extend_from_slice(&chunk);
            match collect_resampled_chunk_from_buffer(
                &buffer,
                target_len,
                incoming_rate,
                self.config.sample_rate,
            ) {
                Ok(Some((processed, remaining))) => {
                    *buffer = remaining;
                    if let Ok(mut engine) = self.engine.lock() {
                        let report = engine.process_chunk(&processed);
                        let snapshot = self.snapshot_for(report, true);
                        let _ = self.snapshot_sender.send(snapshot);
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    buffer.clear();
                    let mut snapshot = self.build_snapshot();
                    snapshot.error = Some(err.to_string());
                    let _ = self.snapshot_sender.send(snapshot);
                    break;
                }
            }
        }
    }

    fn build_snapshot(&self) -> SessionSnapshot {
        self.snapshot_internal(
            AlignmentReport::default(),
            PronunciationScores::default(),
            false,
            None,
            None,
        )
    }

    fn snapshot_for(&self, alignment: AlignmentReport, recording: bool) -> SessionSnapshot {
        self.snapshot_internal(
            alignment,
            PronunciationScores::default(),
            recording,
            None,
            None,
        )
    }

    fn snapshot_internal(
        &self,
        alignment: AlignmentReport,
        scores: PronunciationScores,
        recording: bool,
        recipe_state: Option<RecipeApplicationProgress>,
        error: Option<String>,
    ) -> SessionSnapshot {
        SessionSnapshot {
            alignment,
            scores,
            recording,
            reference_playing: self.reference_playing.load(Ordering::SeqCst),
            active_clip_variant: self.current_variant(),
            has_flowalyzed_clip: self.has_flowalyzed_clip(),
            recipe_state,
            error,
        }
    }

    fn start_reference_playback(&self) {
        self.stop_reference_playback();
        let samples: Vec<f32> = self.reference_clip.samples.iter().copied().collect();
        if samples.is_empty() {
            return;
        }
        let sample_rate = self.reference_clip.sample_rate;
        if let Ok((stream, stream_handle)) = OutputStream::try_default() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let stereo = duplicate_to_stereo(&samples);
                sink.append(SamplesBuffer::new(2, sample_rate, stereo));
                let sink_arc = Arc::new(sink);
                PLAYBACK_STREAM.with(|cell| {
                    *cell.borrow_mut() = Some(stream);
                });
                self.reference_playing.store(true, Ordering::SeqCst);
                *self.playback_state.lock().unwrap() = Some(PlaybackState { sink: sink_arc });
                let _ = self.snapshot_sender.send(self.build_snapshot());
            }
        }
    }

    fn stop_reference_playback(&self) {
        if let Some(state) = self.playback_state.lock().unwrap().take() {
            state.sink.stop();
            self.reference_playing.store(false, Ordering::SeqCst);
            PLAYBACK_STREAM.with(|cell| {
                cell.borrow_mut().take();
            });
            let _ = self.snapshot_sender.send(self.build_snapshot());
        }
    }

    fn poll_playback_completion(&self) {
        if !self.reference_playing.load(Ordering::SeqCst) {
            return;
        }
        let finished = {
            let guard = self.playback_state.lock().unwrap();
            guard
                .as_ref()
                .map(|state| state.sink.empty())
                .unwrap_or(false)
        };
        if finished {
            self.reference_playing.store(false, Ordering::SeqCst);
            self.playback_state.lock().unwrap().take();
            PLAYBACK_STREAM.with(|cell| {
                cell.borrow_mut().take();
            });
            let _ = self.snapshot_sender.send(self.build_snapshot());
        }
    }

    fn handle_apply_recipe(&self, start_sec: f64, end_sec: f64, recipe: Recipe) {
        self.recipe_progress_snapshot(RecipeApplicationStage::ExtractingAudio, 1, 3, None);
        self.recipe_progress_snapshot(RecipeApplicationStage::ApplyingRecipe, 2, 3, None);
        match apply_recipe_to_range(&self.reference_clip, start_sec, end_sec, &recipe) {
            Ok(flowalyzed) => {
                *self.flowalyzed_clip.lock().unwrap() = Some(flowalyzed);
                self.recipe_progress_snapshot(RecipeApplicationStage::SavingResult, 3, 3, None);
            }
            Err(err) => {
                self.recipe_progress_snapshot(
                    RecipeApplicationStage::ApplyingRecipe,
                    2,
                    3,
                    Some(err.to_string()),
                );
            }
        }
    }

    fn recipe_progress_snapshot(
        &self,
        stage: RecipeApplicationStage,
        completed_steps: u32,
        total_steps: u32,
        error: Option<String>,
    ) {
        let progress = self.recipe_progress(stage, completed_steps, total_steps);
        let snapshot = self.snapshot_internal(
            AlignmentReport::default(),
            PronunciationScores::default(),
            false,
            Some(progress),
            error,
        );
        let _ = self.snapshot_sender.send(snapshot);
    }

    fn recipe_progress(
        &self,
        stage: RecipeApplicationStage,
        completed_steps: u32,
        total_steps: u32,
    ) -> RecipeApplicationProgress {
        RecipeApplicationProgress {
            stage,
            completed_steps,
            total_steps,
            sub_stage_index: 0,
            sub_stage_total: 0,
            sub_stage_label: None,
            metric_label: None,
            current_value: None,
            total_value: None,
            elapsed_secs: None,
        }
    }

    fn handle_toggle_variant(&self, variant: ClipVariant) {
        if variant == self.current_variant() {
            return;
        }
        self.stop_reference_playback();
        let clip = match variant {
            ClipVariant::Original => Some((*self.reference_clip).clone()),
            ClipVariant::Flowalyzed => self.flowalyzed_clip.lock().unwrap().clone(),
        };
        match clip {
            Some(clip) => {
                let engine = self.engine.clone();
                let config = self.config.clone();
                std::thread::spawn(move || {
                    if let Ok(mut engine) = engine.lock() {
                        *engine = SessionEngine::new(&clip.samples, &config);
                    }
                });
                if let Ok(mut active) = self.active_variant.lock() {
                    *active = variant;
                }
                let _ = self.snapshot_sender.send(self.build_snapshot());
            }
            None => {
                let mut snapshot = self.build_snapshot();
                snapshot.error = Some("flowalyzed clip not available".to_string());
                let _ = self.snapshot_sender.send(snapshot);
            }
        }
    }

    fn current_variant(&self) -> ClipVariant {
        *self.active_variant.lock().unwrap()
    }

    fn has_flowalyzed_clip(&self) -> bool {
        self.flowalyzed_clip.lock().unwrap().is_some()
    }
}

fn required_raw_samples(target_len: usize, input_rate: u32, output_rate: u32) -> usize {
    (target_len as u64 * input_rate as u64)
        .div_ceil(output_rate as u64) as usize
}

#[allow(dead_code)]
fn collect_resampled_chunk<F>(
    next_chunk: &mut F,
    target_len: usize,
    input_rate: u32,
    output_rate: u32,
) -> std::result::Result<Option<Vec<f32>>, PronunciationError>
where
    F: FnMut() -> Option<Vec<f32>>,
{
    if input_rate == 0 || output_rate == 0 {
        return Err(PronunciationError::new("sample rate must be positive"));
    }
    let mut raw_samples = Vec::new();
    let mut required = required_raw_samples(target_len, input_rate, output_rate);

    loop {
        while raw_samples.len() < required {
            match next_chunk() {
                Some(chunk) => raw_samples.extend_from_slice(&chunk),
                None => return Ok(None),
            }
        }

        if input_rate == output_rate {
            return Ok(Some(raw_samples));
        }

        let resampled = linear_resample(&raw_samples, input_rate, output_rate)
            .map_err(|err| PronunciationError::new(err.to_string()))?;

        if resampled.len() >= target_len {
            return Ok(Some(resampled));
        }

        let shortfall = target_len - resampled.len();
        let extra = required_raw_samples(shortfall, input_rate, output_rate).max(1);
        required = raw_samples.len() + extra;
    }
}

type ChunkResult = std::result::Result<Option<(Vec<f32>, Vec<f32>)>, PronunciationError>;

fn collect_resampled_chunk_from_buffer(
    buffer: &[f32],
    target_len: usize,
    input_rate: u32,
    output_rate: u32,
) -> ChunkResult {
    if buffer.is_empty() {
        return Ok(None);
    }
    let required = required_raw_samples(target_len, input_rate, output_rate);
    if buffer.len() < required {
        return Ok(None);
    }
    let (to_process, remainder) = buffer.split_at(required);
    let resampled = if input_rate == output_rate {
        to_process.to_vec()
    } else {
        linear_resample(to_process, input_rate, output_rate)
            .map_err(|err| PronunciationError::new(err.to_string()))?
    };
    if resampled.len() < target_len {
        // Should not happen given required calculation, but guard anyway.
        return Ok(None);
    }
    Ok(Some((resampled, remainder.to_vec())))
}

struct PlaybackState {
    sink: Arc<Sink>,
}

#[cfg(test)]
mod tests {
    use super::{
        collect_resampled_chunk, collect_resampled_chunk_from_buffer, required_raw_samples,
    };

    #[test]
    fn required_samples_scale_with_input_rate() {
        let target_len = 1_600; // 100ms at 16kHz
        let required = required_raw_samples(target_len, 44_100, 16_000);
        assert_eq!(required, 4_410);
    }

    #[test]
    fn resampled_chunk_meets_target_length() {
        let target_len = 1_600;
        let mut remaining_chunks = vec![vec![0.0_f32; 2_205]; 2].into_iter();
        let mut provider = || remaining_chunks.next();
        let resampled =
            collect_resampled_chunk(&mut provider, target_len, 44_100, 16_000).unwrap();
        let samples = resampled.expect("expected samples");
        assert!(
            samples.len() >= target_len,
            "resampled chunk should meet or exceed target length"
        );
    }

    #[test]
    fn returns_none_when_no_chunks_available() {
        let mut provider = || -> Option<Vec<f32>> { None };
        let resampled = collect_resampled_chunk(&mut provider, 1_000, 16_000, 16_000).unwrap();
        assert!(resampled.is_none());
    }

    #[test]
    fn same_rate_chunks_meet_target_length() {
        let target_len = 1_600;
        let mut remaining_chunks = vec![vec![0.0_f32; target_len / 2]; 2].into_iter();
        let mut provider = || remaining_chunks.next();
        let resampled = collect_resampled_chunk(&mut provider, target_len, 16_000, 16_000).unwrap();
        let samples = resampled.expect("expected samples");
        assert!(
            samples.len() >= target_len,
            "same-rate chunk should meet or exceed target length"
        );
    }

    #[test]
    fn buffered_collection_emits_and_retain_remainder() {
        let target_len = 1_600;
        let input_rate = 48_000;
        let output_rate = 16_000;
        let required = required_raw_samples(target_len, input_rate, output_rate);
        let buffer = vec![0.1_f32; required + 100];
        let result = collect_resampled_chunk_from_buffer(&buffer, target_len, input_rate, output_rate)
            .expect("should resample");
        let (processed, remainder) = result.expect("expected a processed chunk");
        assert!(
            processed.len() >= target_len,
            "processed chunk should meet target length"
        );
        assert_eq!(
            remainder.len(),
            100,
            "should retain unconsumed samples as remainder"
        );
    }
}

pub struct SessionHandle {
    snapshot_receiver: Receiver<SessionSnapshot>,
    config: SessionConfig,
}

impl SessionHandle {
    pub fn drain_snapshots(&self) -> Vec<SessionSnapshot> {
        let mut snapshots = Vec::new();
        while let Ok(snapshot) = self.snapshot_receiver.try_recv() {
            snapshots.push(snapshot);
        }
        snapshots
    }

    pub fn config(&self) -> &SessionConfig {
        &self.config
    }

    pub fn initial_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot::default()
    }
}

pub struct SessionController {
    command_sender: Sender<SessionCommand>,
}

impl SessionController {
    pub fn start(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Start)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn stop(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Stop)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn replay_reference(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::ReplayReference)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn stop_replay(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::StopReplay)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn apply_recipe(&self, start_sec: f64, end_sec: f64, recipe: Recipe) -> Result<()> {
        self.command_sender
            .send(SessionCommand::ApplyRecipe {
                start_sec,
                end_sec,
                recipe,
            })
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn toggle_clip_variant(&self, variant: ClipVariant) -> Result<()> {
        self.command_sender
            .send(SessionCommand::ToggleClipVariant(variant))
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }

    pub fn shutdown(&self) -> Result<()> {
        self.command_sender
            .send(SessionCommand::Shutdown)
            .map_err(|_| PronunciationError::new("runtime disconnected"))
    }
}
