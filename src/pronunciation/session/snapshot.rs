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
    pub recording: bool,
    pub reference_playing: bool,
}
