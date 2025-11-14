use crate::pronunciation::ClipVariant;
use eframe::egui;

#[derive(Default, Debug)]
pub struct ControlStripOutput {
    pub toggle_recording: bool,
    pub replay_reference: bool,
    pub stop_replay: bool,
    pub toggle_to_variant: Option<ClipVariant>,
}

pub struct ControlStrip {
    pub is_recording: bool,
    pub reference_playing: bool,
    pub latency_ms: f32,
    pub latency_budget_ms: u32,
    pub active_clip_variant: ClipVariant,
    pub has_flowalyzed_clip: bool,
}

impl ControlStrip {
    pub fn show(&self, ui: &mut egui::Ui) -> ControlStripOutput {
        let mut output = ControlStripOutput::default();
        if record_button(ui, self.is_recording) {
            output.toggle_recording = true;
        }
        ui.separator();
        if self.reference_playing {
            if stop_replay_button(ui) {
                output.stop_replay = true;
            }
        } else if replay_button(ui, self.is_recording) {
            output.replay_reference = true;
        }
        ui.separator();
        latency_badge(ui, self.latency_ms, self.latency_budget_ms);
        ui.separator();
        if let Some(variant) =
            clip_variant_toggle(ui, self.active_clip_variant, self.has_flowalyzed_clip)
        {
            output.toggle_to_variant = Some(variant);
        }
        output
    }
}

fn record_button(ui: &mut egui::Ui, is_recording: bool) -> bool {
    let label = if is_recording {
        "Stop Shadowing"
    } else {
        "Start Shadowing"
    };
    ui.button(label)
        .on_hover_text(
            "Space toggles shadowing. Starts reference playback and records your pronunciation.",
        )
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

fn stop_replay_button(ui: &mut egui::Ui) -> bool {
    ui.button("Stop Replay")
        .on_hover_text("Stop the reference audio playback.")
        .clicked()
}

fn latency_badge(ui: &mut egui::Ui, latency_ms: f32, budget_ms: u32) {
    let color = latency_color(latency_ms, budget_ms);
    let text = format!("Latency {:.0} ms (budget {} ms)", latency_ms, budget_ms);
    ui.colored_label(color, text)
        .on_hover_text("Capture-to-feedback latency must stay within the 200 ms budget.");
}

fn clip_variant_toggle(
    ui: &mut egui::Ui,
    active: ClipVariant,
    has_flowalyzed: bool,
) -> Option<ClipVariant> {
    let mut result = None;
    ui.horizontal(|ui| {
        ui.label("Clip:");
        let original_selected = active == ClipVariant::Original;
        if ui
            .selectable_label(original_selected, "Original")
            .on_hover_text("Use original reference clip for analysis")
            .clicked()
            && !original_selected
        {
            result = Some(ClipVariant::Original);
            return;
        }
        ui.add_enabled_ui(has_flowalyzed, |ui| {
            let flowalyzed_selected = active == ClipVariant::Flowalyzed;
            if ui
                .selectable_label(flowalyzed_selected, "Flowalyzed")
                .on_hover_text(if has_flowalyzed {
                    "Use flowalyzed clip for analysis"
                } else {
                    "Apply a recipe first to generate flowalyzed clip"
                })
                .clicked()
                && !flowalyzed_selected
            {
                result = Some(ClipVariant::Flowalyzed);
            }
        });
    });
    result
}

fn latency_color(latency_ms: f32, budget_ms: u32) -> egui::Color32 {
    if latency_ms > budget_ms as f32 {
        egui::Color32::from_rgb(200, 60, 60)
    } else if latency_ms > budget_ms as f32 * 0.75 {
        egui::Color32::from_rgb(210, 160, 20)
    } else {
        egui::Color32::from_rgb(30, 180, 80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_color_green_below_threshold() {
        let color = latency_color(50.0, 200);
        assert_eq!(color, egui::Color32::from_rgb(30, 180, 80));
    }

    #[test]
    fn test_latency_color_yellow_above_75_percent() {
        let color = latency_color(151.0, 200);
        assert_eq!(color, egui::Color32::from_rgb(210, 160, 20));
    }

    #[test]
    fn test_latency_color_yellow_just_under_budget() {
        let color = latency_color(199.0, 200);
        assert_eq!(color, egui::Color32::from_rgb(210, 160, 20));
    }

    #[test]
    fn test_latency_color_red_over_budget() {
        let color = latency_color(201.0, 200);
        assert_eq!(color, egui::Color32::from_rgb(200, 60, 60));
    }

    #[test]
    fn test_latency_color_yellow_at_budget() {
        let color = latency_color(200.0, 200);
        assert_eq!(color, egui::Color32::from_rgb(210, 160, 20));
    }

    #[test]
    fn test_latency_color_green_at_75_percent() {
        let color = latency_color(150.0, 200);
        assert_eq!(color, egui::Color32::from_rgb(30, 180, 80));
    }
}
