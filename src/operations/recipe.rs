//! Recipe application - apply sequences of operations to chunks
//!
//! Pure function module following "bricks & studs" philosophy:
//! - Takes AudioChunk and Recipe as input
//! - Returns Vec of processed chunks
//! - No side effects
//! - Reuses existing operation functions

use super::{change_speed, insert_silence, repeat_chunk};
use crate::types::{AudioChunk, Recipe};

/// Apply a recipe (sequence of operations) to a single audio chunk
///
/// For each step in the recipe:
/// 1. Compute a speed-adjusted view of the original chunk
/// 2. If `silent` is false, repeat that audio `repeat_count` times
/// 3. If `silent` is true, emit `repeat_count` silence chunks matching the adjusted duration
///
/// # Arguments
/// * `chunk` - The audio chunk to process
/// * `recipe` - The recipe defining the sequence of operations
///
/// # Returns
/// Vector of audio chunks representing all operations applied
///
/// # Example
/// ```
/// use flowalyzer::types::{AudioChunk, Recipe};
/// use flowalyzer::operations::recipe::apply_recipe;
///
/// let chunk = AudioChunk { /* ... */ };
/// let recipe = Recipe::new("example")
///     .add_step(RecipeStep {
///         repeat_count: 3,
///         speed_factor: 0.75,
///         silent: false,
///     })
///     .add_step(RecipeStep {
///         repeat_count: 1,
///         speed_factor: 0.75,
///         silent: true,
///     });
/// let results = apply_recipe(&chunk, &recipe);
/// assert_eq!(results.len(), 4);
/// ```
pub fn apply_recipe(chunk: &AudioChunk, recipe: &Recipe) -> Vec<AudioChunk> {
    let mut results = Vec::new();

    for step in &recipe.steps {
        let speed_adjusted = change_speed(chunk, step.speed_factor);
        if step.silent {
            let silence_duration = speed_adjusted.end_time - speed_adjusted.start_time;
            for _ in 0..step.repeat_count {
                results.push(insert_silence(silence_duration, speed_adjusted.sample_rate));
            }
        } else {
            let repeated = repeat_chunk(&speed_adjusted, step.repeat_count);
            results.extend(repeated);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::speed::change_speed;
    use crate::types::RecipeStep;

    fn create_test_chunk() -> AudioChunk {
        // Create a 1-second chunk
        let sample_rate = 44100;
        let duration = 1.0;
        let num_samples = (sample_rate as f64 * duration) as usize;

        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((t * 2.0 * std::f32::consts::PI * 440.0).sin());
        }

        AudioChunk {
            samples,
            sample_rate,
            start_time: 0.0,
            end_time: duration,
        }
    }

    #[test]
    fn test_apply_empty_recipe() {
        let chunk = create_test_chunk();
        let recipe = Recipe::new("empty");

        let results = apply_recipe(&chunk, &recipe);
        assert_eq!(results.len(), 0, "Empty recipe should produce no output");
    }

    #[test]
    fn test_apply_single_step_no_silence() {
        let chunk = create_test_chunk();
        let mut recipe = Recipe::new("single-step");
        recipe = recipe.add_step(RecipeStep {
            repeat_count: 2,
            speed_factor: 1.0,
            silent: false,
        });

        let results = apply_recipe(&chunk, &recipe);

        // Should have 2 chunks (2 repeats, no silence)
        assert_eq!(results.len(), 2);

        // Each should be same length as original (speed 1.0)
        for result in &results {
            assert!((result.samples.len() as i32 - chunk.samples.len() as i32).abs() < 100);
        }
    }

    #[test]
    fn test_apply_single_step_with_silence() {
        let chunk = create_test_chunk();
        let mut recipe = Recipe::new("with-silence");
        recipe = recipe.add_step(RecipeStep {
            repeat_count: 2,
            speed_factor: 1.0,
            silent: true,
        });

        let results = apply_recipe(&chunk, &recipe);

        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|chunk| chunk.samples.iter().all(|&s| s == 0.0)));
        let expected_len = change_speed(&chunk, 1.0).samples.len();
        assert!(results
            .iter()
            .all(|silence| silence.samples.len() == expected_len));
    }

    #[test]
    fn test_apply_language_learning_recipe() {
        let chunk = create_test_chunk(); // 1-second chunk
        let recipe = Recipe::new("language-learning")
            .add_step(RecipeStep {
                repeat_count: 3,
                speed_factor: 0.5,
                silent: false,
            })
            .add_step(RecipeStep {
                repeat_count: 1,
                speed_factor: 0.5,
                silent: true,
            })
            .add_step(RecipeStep {
                repeat_count: 3,
                speed_factor: 1.0,
                silent: false,
            })
            .add_step(RecipeStep {
                repeat_count: 1,
                speed_factor: 1.0,
                silent: true,
            })
            .add_step(RecipeStep {
                repeat_count: 3,
                speed_factor: 1.5,
                silent: false,
            })
            .add_step(RecipeStep {
                repeat_count: 1,
                speed_factor: 1.5,
                silent: true,
            });

        let results = apply_recipe(&chunk, &recipe);

        // Expected: (3 slow + silence) + (3 normal + silence) + (3 fast + silence) = 12 chunks
        assert_eq!(
            results.len(),
            12,
            "Language learning recipe should produce 12 chunks"
        );

        // Verify structure:
        // Chunks 0-2: slow (3 repeats at 0.5 speed)
        // Chunk 3: silence
        // Chunks 4-6: normal (3 repeats at 1.0 speed)
        // Chunk 7: silence
        // Chunks 8-10: fast (3 repeats at 1.5 speed)
        // Chunk 11: silence

        // Verify silences are at correct positions and are actually silent
        assert!(
            results[3].samples.iter().all(|&s| s == 0.0),
            "Position 3 should be silence"
        );
        assert!(
            results[7].samples.iter().all(|&s| s == 0.0),
            "Position 7 should be silence"
        );
        assert!(
            results[11].samples.iter().all(|&s| s == 0.0),
            "Position 11 should be silence"
        );

        // Verify slow chunks are longer than normal chunks
        assert!(
            results[0].samples.len() > results[4].samples.len(),
            "Slow chunks should be longer than normal"
        );

        // Verify fast chunks are shorter than normal chunks
        assert!(
            results[8].samples.len() < results[4].samples.len(),
            "Fast chunks should be shorter than normal"
        );
    }

    #[test]
    fn test_recipe_silence_duration_matches_speed() {
        let chunk = create_test_chunk(); // 1-second chunk
        let mut recipe = Recipe::new("test");
        recipe = recipe.add_step(RecipeStep {
            repeat_count: 1,
            speed_factor: 0.5,
            silent: true,
        });

        let results = apply_recipe(&chunk, &recipe);

        assert_eq!(results.len(), 1); // one silence chunk

        assert!(
            results[0].samples.iter().all(|&s| s == 0.0),
            "Silence chunk should be silent"
        );
        let expected_len = change_speed(&chunk, 0.5).samples.len();
        assert_eq!(
            results[0].samples.len(),
            expected_len,
            "Silence duration should match speed-adjusted chunk duration"
        );
    }

    #[test]
    fn test_recipe_with_multiple_speeds() {
        let chunk = create_test_chunk();
        let mut recipe = Recipe::new("multi-speed");
        recipe = recipe
            .add_step(RecipeStep {
                repeat_count: 2,
                speed_factor: 0.5,
                silent: false,
            })
            .add_step(RecipeStep {
                repeat_count: 2,
                speed_factor: 2.0,
                silent: false,
            });

        let results = apply_recipe(&chunk, &recipe);

        // Should have 4 chunks total (2 slow + 2 fast)
        assert_eq!(results.len(), 4);

        // First 2 should be slow (longer)
        assert!(results[0].samples.len() > chunk.samples.len());
        assert!(results[1].samples.len() > chunk.samples.len());

        // Last 2 should be fast (shorter)
        assert!(results[2].samples.len() < chunk.samples.len());
        assert!(results[3].samples.len() < chunk.samples.len());
    }

    #[test]
    fn test_recipe_uses_original_chunk_each_step() {
        let chunk = create_test_chunk();
        let mut recipe = Recipe::new("reuse");
        recipe = recipe
            .add_step(RecipeStep {
                repeat_count: 1,
                speed_factor: 0.75,
                silent: false,
            })
            .add_step(RecipeStep {
                repeat_count: 1,
                speed_factor: 1.5,
                silent: false,
            });

        let results = apply_recipe(&chunk, &recipe);
        assert_eq!(results.len(), 2);

        let slow = change_speed(&chunk, 0.75);
        let fast = change_speed(&chunk, 1.5);

        assert_eq!(results[0].samples, slow.samples);
        assert_eq!(results[1].samples, fast.samples);
    }
}
