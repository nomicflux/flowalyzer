use crate::pronunciation::{AlignmentReport, PronunciationScores, Result};

#[derive(Debug, Clone)]
pub struct VisualizationState {
    pub alignment: AlignmentReport,
    pub scores: PronunciationScores,
}

/// Prepares visualization data for the outer UI layer.
pub fn prepare_visualization(
    alignment: &AlignmentReport,
    scores: &PronunciationScores,
) -> Result<VisualizationState> {
    Ok(VisualizationState {
        alignment: alignment.clone(),
        scores: scores.clone(),
    })
}
