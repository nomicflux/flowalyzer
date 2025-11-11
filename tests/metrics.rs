use approx::assert_relative_eq;
use flowalyzer::pronunciation::metrics::MetricCalculator;
use flowalyzer::pronunciation::{AlignedPhoneme, AlignmentReport};
use std::time::Duration;

fn make_report(phonemes: Vec<AlignedPhoneme>, confidence: f32) -> AlignmentReport {
    let similarity_band = phonemes.iter().map(|p| p.similarity).collect();
    AlignmentReport {
        phonemes,
        total_duration: Duration::from_millis(120),
        reference_path_cost: 0.5,
        learner_path_cost: 0.5,
        global_time_offset_ms: 0.0,
        confidence,
        reference_energy: vec![0.8, 0.7, 0.75],
        learner_energy: vec![0.8, 0.7, 0.75],
        similarity_band,
    }
}

fn phoneme(symbol: &str, delta: f32, variance: f32, similarity: f32) -> AlignedPhoneme {
    AlignedPhoneme {
        symbol: symbol.to_string(),
        reference_start_ms: 0.0,
        reference_end_ms: 40.0,
        learner_start_ms: 0.0,
        learner_end_ms: 40.0,
        timing_delta_ms: delta,
        similarity,
        articulation_variance: variance,
    }
}

#[test]
fn metrics_scoring_balanced_alignment() {
    let phonemes = vec![
        phoneme("T", 10.0, 0.05, 0.9),
        phoneme("AY", -12.0, 0.08, 0.92),
        phoneme("M", 6.0, 0.04, 0.88),
    ];
    let report = make_report(phonemes, 0.85);
    let scores = MetricCalculator::new().score(&report).unwrap();

    println!(
        "balanced timing={:.3} articulation={:.3} intonation={:.3} overall={:.3}",
        scores.timing, scores.articulation, scores.intonation, scores.overall
    );
    assert_relative_eq!(scores.timing, 0.9, epsilon = 0.05);
    assert_relative_eq!(scores.articulation, 0.92, epsilon = 0.05);
    assert_relative_eq!(scores.intonation, 1.0, epsilon = 0.05);
    assert_relative_eq!(scores.overall, 0.89, epsilon = 0.05);
    assert_eq!(scores.per_phoneme.len(), 3);
    assert!(scores
        .per_phoneme
        .iter()
        .all(|score| score.timing <= 1.0 && score.intonation <= 1.0));
}

#[test]
fn metrics_penalize_large_deviation() {
    let phonemes = vec![
        phoneme("T", 180.0, 0.2, 0.5),
        phoneme("AY", -150.0, 0.4, 0.45),
    ];
    let mut report = make_report(phonemes, 0.4);
    report.reference_energy = vec![0.95, 0.9, 0.88];
    report.learner_energy = vec![0.2, 0.25, 0.22];
    let scores = MetricCalculator::new().score(&report).unwrap();

    println!(
        "deviation timing={:.3} articulation={:.3} intonation={:.3} overall={:.3}",
        scores.timing, scores.articulation, scores.intonation, scores.overall
    );
    assert!(scores.timing < 0.2);
    assert!(scores.articulation <= 0.8);
    assert!(scores.intonation < 0.6);
    assert!(scores.overall < 0.5);
}

#[test]
fn metrics_handle_empty_alignment() {
    let report = make_report(Vec::new(), 0.7);
    let scores = MetricCalculator::new().score(&report).unwrap();

    assert_relative_eq!(scores.overall, 0.7, epsilon = 1e-6);
    assert_eq!(scores.timing, 1.0);
    assert_eq!(scores.articulation, 1.0);
    assert_eq!(scores.intonation, 1.0);
    assert!(scores.per_phoneme.is_empty());
}
