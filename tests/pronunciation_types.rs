use flowalyzer::pronunciation::session::{
    AlignmentReport, ClipVariant, RecipeApplicationStage, SessionSnapshot,
};

#[test]
fn alignment_report_default_has_empty_vectors() {
    let report = AlignmentReport::default();
    assert!(report.reference_energy.is_empty());
    assert!(report.learner_energy.is_empty());
    assert_eq!(report.global_time_offset_ms, 0.0);
    assert_eq!(report.confidence, 0.0);
}

#[test]
fn session_snapshot_default_not_recording() {
    let snapshot = SessionSnapshot::default();
    assert!(!snapshot.recording);
    assert!(!snapshot.reference_playing);
    assert!(snapshot.error.is_none());
}

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
