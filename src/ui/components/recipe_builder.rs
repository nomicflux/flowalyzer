use eframe::egui;

use crate::types::{RuntimeRecipe, RuntimeRecipeStep};

#[derive(Debug, Clone)]
pub struct RecipeBuilderState {
    pub name: String,
    pub steps: Vec<RecipeStepState>,
}

#[derive(Debug, Clone)]
pub struct RecipeStepState {
    pub repeat_count: u32,
    pub speed_factor: f32,
    pub silent: bool,
}

pub struct RecipeBuilder<'a> {
    pub state: &'a mut RecipeBuilderState,
    pub enabled: bool,
}

#[derive(Default)]
pub struct RecipeBuilderOutput {
    pub apply_requested: bool,
    pub clear_requested: bool,
    pub validation_error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum RecipeValidationError {
    EmptySteps,
    InvalidRepeatCount(usize),
    InvalidSpeedFactor(usize),
}

impl RecipeBuilderState {
    pub fn new() -> Self {
        Self {
            name: String::from("custom"),
            steps: Vec::new(),
        }
    }

    pub fn from_preset(name: &str) -> Option<Self> {
        match name {
            "language_learning" => Some(Self {
                name: String::from("language_learning"),
                steps: vec![
                    RecipeStepState {
                        repeat_count: 3,
                        speed_factor: 0.5,
                        silent: false,
                    },
                    RecipeStepState {
                        repeat_count: 1,
                        speed_factor: 0.5,
                        silent: true,
                    },
                    RecipeStepState {
                        repeat_count: 3,
                        speed_factor: 1.0,
                        silent: false,
                    },
                    RecipeStepState {
                        repeat_count: 1,
                        speed_factor: 1.0,
                        silent: true,
                    },
                    RecipeStepState {
                        repeat_count: 3,
                        speed_factor: 1.5,
                        silent: false,
                    },
                    RecipeStepState {
                        repeat_count: 1,
                        speed_factor: 1.5,
                        silent: true,
                    },
                ],
            }),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), RecipeValidationError> {
        if self.steps.is_empty() {
            return Err(RecipeValidationError::EmptySteps);
        }
        for (idx, step) in self.steps.iter().enumerate() {
            if step.repeat_count == 0 {
                return Err(RecipeValidationError::InvalidRepeatCount(idx));
            }
            if step.speed_factor <= 0.0 {
                return Err(RecipeValidationError::InvalidSpeedFactor(idx));
            }
        }
        Ok(())
    }

    pub fn to_runtime_recipe(&self) -> RuntimeRecipe {
        RuntimeRecipe {
            name: Some(self.name.clone()),
            steps: self
                .steps
                .iter()
                .map(|s| RuntimeRecipeStep {
                    repeat_count: s.repeat_count,
                    speed_factor: s.speed_factor,
                    silent: s.silent,
                })
                .collect(),
        }
    }
}

impl Default for RecipeBuilderState {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> RecipeBuilder<'a> {
    pub fn show(&mut self, ui: &mut egui::Ui) -> RecipeBuilderOutput {
        let mut output = RecipeBuilderOutput::default();

        ui.add_enabled_ui(self.enabled, |ui| {
            ui.heading("Recipe Builder");
            ui.separator();

            show_name_input(ui, &mut self.state.name);
            show_preset_selector(ui, self.state);
            ui.separator();

            show_steps_list(ui, &mut self.state.steps);
            ui.separator();

            show_action_buttons(ui, self.state, &mut output);
        });

        output
    }
}

fn show_name_input(ui: &mut egui::Ui, name: &mut String) {
    ui.horizontal(|ui| {
        ui.label("Recipe Name:");
        let response = ui.text_edit_singleline(name);
        if name.len() > 50 {
            name.truncate(50);
            response.request_focus();
        }
    });
}

fn show_preset_selector(ui: &mut egui::Ui, state: &mut RecipeBuilderState) {
    ui.horizontal(|ui| {
        ui.label("Presets:");
        if ui.button("Language Learning").clicked() {
            if let Some(preset) = RecipeBuilderState::from_preset("language_learning") {
                *state = preset;
            }
        }
    });
}

fn show_steps_list(ui: &mut egui::Ui, steps: &mut Vec<RecipeStepState>) {
    ui.label(format!("Steps ({})", steps.len()));

    let mut to_remove = None;
    for (idx, step) in steps.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Step {}", idx + 1));
                if ui.button("Remove").clicked() {
                    to_remove = Some(idx);
                }
            });
            show_step_controls(ui, step);
        });
    }

    if let Some(idx) = to_remove {
        steps.remove(idx);
    }

    if ui.button("Add Step").clicked() {
        add_default_step(steps);
    }
}

fn show_step_controls(ui: &mut egui::Ui, step: &mut RecipeStepState) {
    ui.horizontal(|ui| {
        ui.label("Repeat:");
        ui.add(egui::Slider::new(&mut step.repeat_count, 1..=10));
    });
    ui.horizontal(|ui| {
        ui.label("Speed:");
        ui.add(egui::Slider::new(&mut step.speed_factor, 0.25..=2.0));
    });
    ui.checkbox(&mut step.silent, "Silent (pause instead of audio)");
}

fn add_default_step(steps: &mut Vec<RecipeStepState>) {
    steps.push(RecipeStepState {
        repeat_count: 1,
        speed_factor: 1.0,
        silent: false,
    });
}

fn show_action_buttons(
    ui: &mut egui::Ui,
    state: &RecipeBuilderState,
    output: &mut RecipeBuilderOutput,
) {
    ui.horizontal(|ui| {
        let validation = state.validate();
        let is_valid = validation.is_ok();

        ui.add_enabled_ui(is_valid, |ui| {
            if ui.button("Apply Recipe").clicked() {
                output.apply_requested = true;
            }
        });

        if ui.button("Clear").clicked() {
            output.clear_requested = true;
        }

        if let Err(err) = validation {
            display_validation_error(ui, err, output);
        }
    });
}

fn display_validation_error(
    ui: &mut egui::Ui,
    err: RecipeValidationError,
    output: &mut RecipeBuilderOutput,
) {
    let message = match err {
        RecipeValidationError::EmptySteps => "Recipe must contain at least one step".to_string(),
        RecipeValidationError::InvalidRepeatCount(idx) => {
            format!("Step {} has invalid repeat count (must be > 0)", idx + 1)
        }
        RecipeValidationError::InvalidSpeedFactor(idx) => {
            format!("Step {} has invalid speed factor (must be > 0)", idx + 1)
        }
    };
    ui.colored_label(egui::Color32::from_rgb(200, 60, 60), &message);
    output.validation_error = Some(message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_steps() {
        let state = RecipeBuilderState::new();
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::EmptySteps)
        );
    }

    #[test]
    fn test_validate_zero_repeat() {
        let mut state = RecipeBuilderState::new();
        state.steps.push(RecipeStepState {
            repeat_count: 0,
            speed_factor: 1.0,
            silent: false,
        });
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::InvalidRepeatCount(0))
        );
    }

    #[test]
    fn test_validate_zero_speed() {
        let mut state = RecipeBuilderState::new();
        state.steps.push(RecipeStepState {
            repeat_count: 1,
            speed_factor: 0.0,
            silent: false,
        });
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::InvalidSpeedFactor(0))
        );
    }

    #[test]
    fn test_validate_negative_speed() {
        let mut state = RecipeBuilderState::new();
        state.steps.push(RecipeStepState {
            repeat_count: 1,
            speed_factor: -0.5,
            silent: false,
        });
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::InvalidSpeedFactor(0))
        );
    }

    #[test]
    fn test_validate_valid_recipe() {
        let mut state = RecipeBuilderState::new();
        state.steps.push(RecipeStepState {
            repeat_count: 3,
            speed_factor: 0.5,
            silent: false,
        });
        state.steps.push(RecipeStepState {
            repeat_count: 1,
            speed_factor: 1.0,
            silent: true,
        });
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_to_runtime_recipe() {
        let mut state = RecipeBuilderState::new();
        state.name = String::from("test_recipe");
        state.steps.push(RecipeStepState {
            repeat_count: 2,
            speed_factor: 0.75,
            silent: false,
        });
        state.steps.push(RecipeStepState {
            repeat_count: 1,
            speed_factor: 1.0,
            silent: true,
        });

        let runtime = state.to_runtime_recipe();
        assert_eq!(runtime.name, Some(String::from("test_recipe")));
        assert_eq!(runtime.steps.len(), 2);
        assert_eq!(runtime.steps[0].repeat_count, 2);
        assert_eq!(runtime.steps[0].speed_factor, 0.75);
        assert!(!runtime.steps[0].silent);
        assert_eq!(runtime.steps[1].repeat_count, 1);
        assert_eq!(runtime.steps[1].speed_factor, 1.0);
        assert!(runtime.steps[1].silent);
    }

    #[test]
    fn test_preset_language_learning() {
        let state = RecipeBuilderState::from_preset("language_learning").unwrap();
        assert_eq!(state.name, "language_learning");
        assert_eq!(state.steps.len(), 6);

        assert_eq!(state.steps[0].repeat_count, 3);
        assert_eq!(state.steps[0].speed_factor, 0.5);
        assert!(!state.steps[0].silent);

        assert_eq!(state.steps[1].repeat_count, 1);
        assert_eq!(state.steps[1].speed_factor, 0.5);
        assert!(state.steps[1].silent);

        assert_eq!(state.steps[2].repeat_count, 3);
        assert_eq!(state.steps[2].speed_factor, 1.0);
        assert!(!state.steps[2].silent);

        assert_eq!(state.steps[3].repeat_count, 1);
        assert_eq!(state.steps[3].speed_factor, 1.0);
        assert!(state.steps[3].silent);

        assert_eq!(state.steps[4].repeat_count, 3);
        assert_eq!(state.steps[4].speed_factor, 1.5);
        assert!(!state.steps[4].silent);

        assert_eq!(state.steps[5].repeat_count, 1);
        assert_eq!(state.steps[5].speed_factor, 1.5);
        assert!(state.steps[5].silent);
    }

    #[test]
    fn test_preset_validation() {
        let state = RecipeBuilderState::from_preset("language_learning").unwrap();
        assert!(state.validate().is_ok());
    }

    #[test]
    fn test_preset_unknown() {
        let state = RecipeBuilderState::from_preset("unknown_preset");
        assert!(state.is_none());
    }

    #[test]
    fn test_builder_initializes_empty() {
        let state = RecipeBuilderState::new();
        assert_eq!(state.name, "custom");
        assert_eq!(state.steps.len(), 0);
    }

    #[test]
    fn test_add_remove_steps() {
        let mut state = RecipeBuilderState::new();

        state.steps.push(RecipeStepState {
            repeat_count: 1,
            speed_factor: 1.0,
            silent: false,
        });
        assert_eq!(state.steps.len(), 1);

        state.steps.push(RecipeStepState {
            repeat_count: 2,
            speed_factor: 0.5,
            silent: true,
        });
        assert_eq!(state.steps.len(), 2);

        state.steps.remove(0);
        assert_eq!(state.steps.len(), 1);
        assert_eq!(state.steps[0].repeat_count, 2);
    }

    #[test]
    fn test_validation_display() {
        let mut state = RecipeBuilderState::new();
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::EmptySteps)
        );

        state.steps.push(RecipeStepState {
            repeat_count: 0,
            speed_factor: 1.0,
            silent: false,
        });
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::InvalidRepeatCount(0))
        );

        state.steps[0].repeat_count = 1;
        state.steps[0].speed_factor = 0.0;
        assert_eq!(
            state.validate().err(),
            Some(RecipeValidationError::InvalidSpeedFactor(0))
        );
    }

    #[test]
    fn test_apply_disabled_when_invalid() {
        let state = RecipeBuilderState::new();
        assert!(state.validate().is_err());

        let mut valid_state = RecipeBuilderState::new();
        valid_state.steps.push(RecipeStepState {
            repeat_count: 1,
            speed_factor: 1.0,
            silent: false,
        });
        assert!(valid_state.validate().is_ok());
    }

    #[test]
    fn test_state_persists_until_clear() {
        let mut state = RecipeBuilderState::from_preset("language_learning").unwrap();
        assert_eq!(state.steps.len(), 6);

        state.steps[0].repeat_count = 5;
        assert_eq!(state.steps[0].repeat_count, 5);

        state.name = String::from("modified");
        assert_eq!(state.name, "modified");
    }
}
