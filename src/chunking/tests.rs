use super::calculate_chunk_boundaries;
use crate::types::{ChunkConfig, Granularity, Segment, Transcript};

#[test]
fn test_basic_chunking() {
    let transcript = Transcript {
        segments: vec![
            Segment {
                text: "Hello".to_string(),
                start_time: 0.0,
                end_time: 0.5,
                granularity: Granularity::Word,
            },
            Segment {
                text: "world".to_string(),
                start_time: 0.5,
                end_time: 1.0,
                granularity: Granularity::Word,
            },
            Segment {
                text: "This is a test".to_string(),
                start_time: 1.5,
                end_time: 2.5,
                granularity: Granularity::Sentence,
            },
        ],
    };

    let config = ChunkConfig::new(2.0);
    let boundaries = calculate_chunk_boundaries(&transcript, config, &[]);

    assert!(!boundaries.is_empty());
    assert!(boundaries[0].end_time > boundaries[0].start_time);
}

#[test]
fn test_long_segment_splitting() {
    let transcript = Transcript {
        segments: vec![Segment {
            text: "Very long sentence".to_string(),
            start_time: 0.0,
            end_time: 5.0,
            granularity: Granularity::Sentence,
        }],
    };

    let config = ChunkConfig::new(2.0);
    let boundaries = calculate_chunk_boundaries(&transcript, config, &[]);

    assert!(boundaries.len() > 1);
}

#[test]
fn test_allows_small_overshoot() {
    let transcript = Transcript {
        segments: vec![
            Segment {
                text: "Phrase part one".to_string(),
                start_time: 0.0,
                end_time: 0.4,
                granularity: Granularity::Sentence,
            },
            Segment {
                text: "Phrase finishing".to_string(),
                start_time: 0.4,
                end_time: 1.6,
                granularity: Granularity::Sentence,
            },
            Segment {
                text: "Next phrase".to_string(),
                start_time: 1.6,
                end_time: 2.4,
                granularity: Granularity::Sentence,
            },
        ],
    };

    let config = ChunkConfig::new(1.0);
    let boundaries = calculate_chunk_boundaries(&transcript, config, &[]);

    assert_eq!(boundaries.len(), 2);
    assert!((boundaries[0].start_time - 0.0).abs() < 1e-9);
    assert!((boundaries[0].end_time - 1.6).abs() < 1e-9);
    assert_eq!(boundaries[0].source_segment_ids, vec![0, 1]);
    assert!((boundaries[1].start_time - 1.6).abs() < 1e-9);
    assert!((boundaries[1].end_time - 2.4).abs() < 1e-9);
    assert_eq!(boundaries[1].source_segment_ids, vec![2]);
}

#[test]
fn test_segment_exceeding_overshoot_is_split() {
    let transcript = Transcript {
        segments: vec![Segment {
            text: "Long music bed".to_string(),
            start_time: 0.0,
            end_time: 2.5,
            granularity: Granularity::Sentence,
        }],
    };

    let config = ChunkConfig::new(1.0);
    let boundaries = calculate_chunk_boundaries(&transcript, config, &[]);

    assert_eq!(boundaries.len(), 3);
    let starts: Vec<f64> = boundaries
        .iter()
        .map(|boundary| boundary.start_time)
        .collect();
    let ends: Vec<f64> = boundaries
        .iter()
        .map(|boundary| boundary.end_time)
        .collect();
    assert_eq!(starts, vec![0.0, 1.0, 2.0]);
    assert_eq!(ends, vec![1.0, 2.0, 2.5]);
}

#[test]
fn test_prefers_pause_boundaries() {
    let transcript = Transcript {
        segments: vec![
            Segment {
                text: "Lead in".to_string(),
                start_time: 0.0,
                end_time: 0.3,
                granularity: Granularity::Sentence,
            },
            Segment {
                text: "Main content continues".to_string(),
                start_time: 0.3,
                end_time: 2.3,
                granularity: Granularity::Sentence,
            },
        ],
    };

    let config = ChunkConfig::new(1.0);
    let pauses = vec![1.2];
    let boundaries = calculate_chunk_boundaries(&transcript, config, &pauses);

    assert_eq!(boundaries.len(), 2);
    assert!((boundaries[0].start_time - 0.0).abs() < 1e-9);
    assert!((boundaries[0].end_time - 1.2).abs() < 1e-9);
    assert_eq!(boundaries[0].source_segment_ids, vec![0, 1]);
    assert!((boundaries[1].start_time - 1.2).abs() < 1e-9);
    assert!((boundaries[1].end_time - 2.3).abs() < 1e-9);
    assert_eq!(boundaries[1].source_segment_ids, vec![1]);
}
