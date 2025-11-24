use flowalyzer::pronunciation::session::{ClipVariant, RecipeApplicationStage};

#[test]
fn clip_variant_equality() {
    assert_eq!(ClipVariant::Original, ClipVariant::Original);
    assert_ne!(ClipVariant::Original, ClipVariant::Flowalyzed);
}

#[test]
fn recipe_stage_ordering() {
    assert_eq!(RecipeApplicationStage::ExtractingAudio.order(), 0);
    assert_eq!(RecipeApplicationStage::ApplyingRecipe.order(), 1);
    assert_eq!(RecipeApplicationStage::SavingResult.order(), 2);
    let ordered = RecipeApplicationStage::ordered();
    assert_eq!(ordered[0], RecipeApplicationStage::ExtractingAudio);
    assert_eq!(ordered[2], RecipeApplicationStage::SavingResult);
}
