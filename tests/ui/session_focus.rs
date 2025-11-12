use flowalyzer::pronunciation::{AlignedPhoneme, SessionSnapshot};
use flowalyzer::ui::components::phoneme_timeline::needs_attention;

#[test]
fn phoneme_attention_flags_risky_segments() {
    let mut phoneme = AlignedPhoneme {
        symbol: "ka".into(),
        similarity: 0.82,
        articulation_variance: 0.18,
        contour_similarity: 0.88,
        ..AlignedPhoneme::default()
    };
    assert!(!needs_attention(&phoneme));

    phoneme.similarity = 0.60;
    assert!(needs_attention(&phoneme));

    phoneme.similarity = 0.82;
    phoneme.articulation_variance = 0.55;
    assert!(needs_attention(&phoneme));
}

#[test]
fn latency_budget_overrun_is_reported() {
    let snapshot = SessionSnapshot::default().with_latency(260.0, 200);
    assert!(snapshot
        .error
        .as_ref()
        .is_some_and(|message| message.contains("latency")));
}

