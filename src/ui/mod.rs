pub mod components;
pub mod screens;

use eframe::NativeOptions;

use crate::pronunciation::{
    AlignmentReport, PronunciationError, PronunciationScores, Result, SessionConfig,
};

pub fn launch_ui(
    config: &SessionConfig,
    alignment: &AlignmentReport,
    scores: &PronunciationScores,
) -> Result<()> {
    let app = screens::session::SessionApp::new(alignment.clone(), scores.clone());
    let options = NativeOptions::default();
    eframe::run_native(
        &window_title(config),
        options,
        Box::new(move |_cc| Box::new(app)),
    )
    .map_err(|err| PronunciationError::new(err.to_string()))
}

fn window_title(config: &SessionConfig) -> String {
    config
        .reference_wav
        .file_name()
        .map(|name| format!("Flowalyzer Pronunciation – {}", name.to_string_lossy()))
        .unwrap_or_else(|| "Flowalyzer Pronunciation".to_string())
}
