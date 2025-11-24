#[derive(Debug, Clone)]
pub struct AlignmentReport {
    pub reference_energy: Vec<f32>, // Current chunk only
    pub learner_energy: Vec<f32>,   // Current chunk only
    pub energy_error: Vec<f32>,     // Current chunk only
    pub reference_pitch: Vec<f32>,  // Current chunk only
    pub learner_pitch: Vec<f32>,    // Current chunk only
    pub similarity_band: Vec<f32>,  // Current chunk only
    pub contour_band: Vec<f32>,     // Current chunk only
    pub start_frame_idx: usize,
    pub end_frame_idx: usize,
    pub hop_ms: f32,
    pub global_time_offset_ms: f32,
    pub total_duration: f32,
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
