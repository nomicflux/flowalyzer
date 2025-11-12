pub mod components;
pub mod screens;

use eframe::NativeOptions;

use crate::pronunciation::{PronunciationError, Result, SessionConfig, SessionRuntime};

pub fn launch_ui(runtime: SessionRuntime) -> Result<()> {
    let handle = runtime.into_handle();
    let title = window_title(handle.config());
    let app = screens::session::SessionApp::new(handle);
    let options = NativeOptions::default();
    eframe::run_native(&title, options, Box::new(move |_cc| Box::new(app)))
        .map_err(|err| PronunciationError::new(err.to_string()))
}

fn window_title(config: &SessionConfig) -> String {
    config
        .reference_wav
        .file_name()
        .map(|name| format!("Flowalyzer Pronunciation – {}", name.to_string_lossy()))
        .unwrap_or_else(|| "Flowalyzer Pronunciation".to_string())
}
