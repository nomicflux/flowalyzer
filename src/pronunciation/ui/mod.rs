use crate::pronunciation::{AlignmentReport, PronunciationScores, Result};

/// Lightweight snapshot of visualization state.
#[derive(Debug, Default, Clone)]
pub struct VisualizationState {
    pub updated: bool,
}

/// Prepares visualization data for the outer UI layer.
pub fn prepare_visualization(
    _alignment: &AlignmentReport,
    _scores: &PronunciationScores,
) -> Result<VisualizationState> {
    Ok(VisualizationState { updated: true })
}
