use flowalyzer::pronunciation::{apply_recipe_to_range, extract_audio_range, RecordedClip};
use flowalyzer::types::{Recipe, RecipeStep};

const TARGET_SAMPLE_RATE: u32 = 16_000;

fn create_test_clip(duration_secs: f64) -> RecordedClip {
    let num_samples = (TARGET_SAMPLE_RATE as f64 * duration_secs) as usize;
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / TARGET_SAMPLE_RATE as f32;
            (t * 2.0 * std::f32::consts::PI * 440.0).sin()
        })
        .collect();
    RecordedClip::from_samples(samples, TARGET_SAMPLE_RATE)
}

#[test]
fn test_extract_audio_range_valid() {
    let clip = create_test_clip(2.0);
    let result = extract_audio_range(&clip, 0.5, 1.5).unwrap();
    assert_eq!(result.sample_rate, TARGET_SAMPLE_RATE);
    assert_eq!(result.start_time, 0.5);
    assert_eq!(result.end_time, 1.5);
    let expected_samples = (TARGET_SAMPLE_RATE as f64 * 1.0) as usize;
    assert_eq!(result.samples.len(), expected_samples);
}

#[test]
fn test_extract_audio_range_invalid_start() {
    let clip = create_test_clip(2.0);
    let result = extract_audio_range(&clip, -0.1, 1.0);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("start_time must be non-negative"));
}

#[test]
fn test_extract_audio_range_invalid_end() {
    let clip = create_test_clip(2.0);
    let result = extract_audio_range(&clip, 1.0, 0.5);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("end_time must be greater than start_time"));
}

#[test]
fn test_extract_audio_range_exceeds_duration() {
    let clip = create_test_clip(2.0);
    let result = extract_audio_range(&clip, 1.0, 3.0);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("end_time exceeds clip duration"));
}

#[test]
fn test_extract_audio_range_boundary_clamping() {
    let clip = create_test_clip(1.0);
    let result = extract_audio_range(&clip, 0.0, 1.0).unwrap();
    assert_eq!(result.samples.len(), TARGET_SAMPLE_RATE as usize);
    let result2 = extract_audio_range(&clip, 0.5, 1.0).unwrap();
    assert_eq!(
        result2.samples.len(),
        (TARGET_SAMPLE_RATE as f64 * 0.5) as usize
    );
}

#[test]
fn test_apply_recipe_to_range_basic() {
    let clip = create_test_clip(2.0);
    let recipe = Recipe::new("test").add_step(RecipeStep {
        repeat_count: 2,
        speed_factor: 1.0,
        silent: false,
    });
    let result = apply_recipe_to_range(&clip, 0.0, 1.0, &recipe).unwrap();
    assert_eq!(result.sample_rate, TARGET_SAMPLE_RATE);
    assert!(result.duration.as_secs_f64() > 1.0);
    assert!(result.duration.as_secs_f64() < 3.0);
}

#[test]
fn test_apply_recipe_to_range_empty_recipe() {
    let clip = create_test_clip(2.0);
    let recipe = Recipe::new("empty");
    let result = apply_recipe_to_range(&clip, 0.0, 1.0, &recipe);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("recipe produced no output chunks"));
}

#[test]
fn test_apply_recipe_to_range_duration_validation() {
    let clip = create_test_clip(60.0);
    let recipe = Recipe::new("long").add_step(RecipeStep {
        repeat_count: 10,
        speed_factor: 1.0,
        silent: false,
    });
    let result = apply_recipe_to_range(&clip, 0.0, 60.0, &recipe);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
}

#[test]
fn test_apply_recipe_to_range_assembles_chunks() {
    let clip = create_test_clip(2.0);
    let recipe = Recipe::new("multi-step")
        .add_step(RecipeStep {
            repeat_count: 2,
            speed_factor: 1.0,
            silent: false,
        })
        .add_step(RecipeStep {
            repeat_count: 1,
            speed_factor: 1.0,
            silent: true,
        });
    let result = apply_recipe_to_range(&clip, 0.0, 1.0, &recipe).unwrap();
    assert!(result.duration.as_secs_f64() > 2.0);
    assert!(result.samples.len() > TARGET_SAMPLE_RATE as usize * 2);
}

#[test]
fn test_apply_recipe_to_range_preserves_sample_rate() {
    let clip = create_test_clip(2.0);
    let recipe = Recipe::new("test").add_step(RecipeStep {
        repeat_count: 1,
        speed_factor: 0.75,
        silent: false,
    });
    let result = apply_recipe_to_range(&clip, 0.0, 1.0, &recipe).unwrap();
    assert_eq!(result.sample_rate, TARGET_SAMPLE_RATE);
}
