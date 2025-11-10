use crate::pronunciation::{AlignmentReport, PronunciationScores, Result, SessionConfig};

/// Launches the interactive pronunciation UI.
pub fn launch_ui(
    _config: &SessionConfig,
    _alignment: &AlignmentReport,
    _scores: &PronunciationScores,
) -> Result<()> {
    Ok(())
}
