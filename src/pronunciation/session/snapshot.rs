use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AlignmentReport {
    pub reference_energy: Vec<f32>,    // Current chunk only
    pub learner_energy: Vec<f32>,      // Current chunk only
    pub reference_pitch: Vec<f32>,     // Current chunk only
    pub learner_pitch: Vec<f32>,       // Current chunk only
    pub similarity_band: Vec<f32>,     // Current chunk only
    pub contour_band: Vec<f32>,        // Current chunk only
    pub phonemes: Vec<AlignedPhoneme>, // Current chunk only
    pub total_duration: Duration,
    pub global_time_offset_ms: f32,
    pub confidence: f32,
}

impl Default for AlignmentReport {
    fn default() -> Self {
        Self {
            reference_energy: Vec::new(),
            learner_energy: Vec::new(),
            reference_pitch: Vec::new(),
            learner_pitch: Vec::new(),
            similarity_band: Vec::new(),
            contour_band: Vec::new(),
            phonemes: Vec::new(),
            total_duration: Duration::ZERO,
            global_time_offset_ms: 0.0,
            confidence: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    pub alignment: AlignmentReport,
    pub scores: PronunciationScores,
    pub recording: bool,
    pub reference_playing: bool,
    pub active_clip_variant: ClipVariant,
    pub has_flowalyzed_clip: bool,
    pub recipe_state: Option<RecipeApplicationProgress>,
    pub error: Option<String>,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            alignment: AlignmentReport::default(),
            scores: PronunciationScores::default(),
            recording: false,
            reference_playing: false,
            active_clip_variant: ClipVariant::Original,
            has_flowalyzed_clip: false,
            recipe_state: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlignedPhoneme {
    pub symbol: String,
    pub timing_delta_ms: f32,
    pub similarity: f32,
    pub articulation_variance: f32,
    pub contour_similarity: f32,
}

#[derive(Debug, Clone)]
pub struct PronunciationScores {
    pub overall: f32,
    pub timing: f32,
    pub articulation: f32,
    pub intonation: f32,
}

impl Default for PronunciationScores {
    fn default() -> Self {
        Self {
            overall: 0.0,
            timing: 0.0,
            articulation: 0.0,
            intonation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipVariant {
    Original,
    Flowalyzed,
}

#[derive(Debug, Clone)]
pub struct RecipeApplicationProgress {
    pub stage: RecipeApplicationStage,
    pub completed_steps: u32,
    pub total_steps: u32,
    pub sub_stage_index: u32,
    pub sub_stage_total: u32,
    pub sub_stage_label: Option<String>,
    pub metric_label: Option<String>,
    pub current_value: Option<u32>,
    pub total_value: Option<u32>,
    pub elapsed_secs: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipeApplicationStage {
    ExtractingAudio,
    ApplyingRecipe,
    SavingResult,
}

impl RecipeApplicationStage {
    pub fn order(&self) -> usize {
        match self {
            Self::ExtractingAudio => 0,
            Self::ApplyingRecipe => 1,
            Self::SavingResult => 2,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::ExtractingAudio => "Extracting audio",
            Self::ApplyingRecipe => "Applying recipe",
            Self::SavingResult => "Saving result",
        }
    }

    pub fn ordered() -> [Self; 3] {
        [
            Self::ExtractingAudio,
            Self::ApplyingRecipe,
            Self::SavingResult,
        ]
    }
}
