use eframe::egui;

#[derive(Default, Debug)]
pub struct ControlStripOutput {
    pub toggle_recording: bool,
    pub replay_reference: bool,
}

pub struct ControlStrip {
    pub is_recording: bool,
    pub latency_ms: f32,
    pub latency_budget_ms: u32,
}

impl ControlStrip {
    pub fn show(&self, ui: &mut egui::Ui) -> ControlStripOutput {
        let mut output = ControlStripOutput::default();
        if record_button(ui, self.is_recording) {
            output.toggle_recording = true;
        }
        ui.separator();
        if replay_button(ui, self.is_recording) {
            output.replay_reference = true;
        }
        ui.separator();
        latency_badge(ui, self.latency_ms, self.latency_budget_ms);
        output
    }
}

fn record_button(ui: &mut egui::Ui, is_recording: bool) -> bool {
    let label = if is_recording {
        "Stop Recording"
    } else {
        "Start Recording"
    };
    ui.button(label)
        .on_hover_text("Space toggles recording. Input is always live audio captured in-session.")
        .clicked()
}

fn replay_button(ui: &mut egui::Ui, is_recording: bool) -> bool {
    let mut clicked = false;
    ui.add_enabled_ui(!is_recording, |ui| {
        if ui
            .button("Replay Reference")
            .on_hover_text("Press R to restart the shadowing session with the reference clip.")
            .clicked()
        {
            clicked = true;
        }
    });
    clicked
}

fn latency_badge(ui: &mut egui::Ui, latency_ms: f32, budget_ms: u32) {
    let color = if latency_ms > budget_ms as f32 {
        egui::Color32::from_rgb(200, 60, 60)
    } else if latency_ms > budget_ms as f32 * 0.75 {
        egui::Color32::from_rgb(210, 160, 20)
    } else {
        egui::Color32::from_rgb(30, 180, 80)
    };
    let text = format!("Latency {:.0} ms (budget {} ms)", latency_ms, budget_ms);
    ui.colored_label(color, text)
        .on_hover_text("Capture-to-feedback latency must stay within the 200 ms budget.");
}
