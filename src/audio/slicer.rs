use crate::types::{AudioChunk, AudioData, ChunkBoundary, FrameCount, FrameIndex, FrameRange};

#[cfg(test)]
use crate::types::SampleRate;

/// Pure function to slice audio data into chunks based on time boundaries
pub fn slice_audio(audio: &AudioData, boundaries: &[ChunkBoundary]) -> Vec<AudioChunk> {
    let mut chunks = Vec::with_capacity(boundaries.len());
    let total_frames = FrameCount::from(audio.samples.len());

    for boundary in boundaries {
        let clamped_range = boundary.frame_range.clamp_to(total_frames);
        let start_sample = clamped_range.start.as_usize();
        let end_sample = clamped_range.end().as_usize();

        // Clamp to valid range
        let start_sample = start_sample.min(audio.samples.len());
        let end_sample = end_sample.min(audio.samples.len());

        // Extract samples for this chunk
        let samples = audio.samples[start_sample..end_sample].to_vec();
        let length = FrameCount::from(samples.len());
        let frame_range = FrameRange::new(FrameIndex::from(start_sample), length);

        chunks.push(AudioChunk {
            samples,
            sample_rate: audio.sample_rate,
            start_time: boundary.start_time,
            end_time: boundary.end_time,
            frame_range,
        });
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_slicing() {
        // Create test audio: 1 second at 44100 Hz
        let sample_rate = SampleRate::new(44100).unwrap();
        let audio = AudioData {
            samples: vec![0.0; 44100],
            sample_rate: 44100,
            frame_range: FrameRange::new(FrameIndex::ZERO, FrameCount::from(44100usize)),
        };

        let boundaries = vec![
            ChunkBoundary {
                start_time: 0.0,
                end_time: 0.5,
                frame_range: FrameRange::from_times(0.0, 0.5, sample_rate),
                source_segment_ids: vec![0],
            },
            ChunkBoundary {
                start_time: 0.5,
                end_time: 1.0,
                frame_range: FrameRange::from_times(0.5, 1.0, sample_rate),
                source_segment_ids: vec![1],
            },
        ];

        let chunks = slice_audio(&audio, &boundaries);

        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].samples.len(), 22050); // 0.5 * 44100
        assert_eq!(chunks[1].samples.len(), 22050);
    }

    #[test]
    fn test_boundary_clamping() {
        let sample_rate = SampleRate::new(1000).unwrap();
        let audio = AudioData {
            samples: vec![0.0; 1000],
            sample_rate: 1000,
            frame_range: FrameRange::new(FrameIndex::ZERO, FrameCount::from(1000usize)),
        };

        // Boundary extends beyond audio length
        let boundaries = vec![ChunkBoundary {
            start_time: 0.5,
            end_time: 2.0, // Beyond 1 second
            frame_range: FrameRange::from_times(0.5, 2.0, sample_rate),
            source_segment_ids: vec![0],
        }];

        let chunks = slice_audio(&audio, &boundaries);

        assert_eq!(chunks.len(), 1);
        // Should clamp to available samples
        assert_eq!(chunks[0].samples.len(), 500); // 0.5 seconds worth
    }

    #[test]
    fn slice_ranges_cover_source_frames() {
        let sample_rate = SampleRate::new(10_000).unwrap();
        let audio = AudioData {
            samples: vec![0.0; 10_000],
            sample_rate: 10_000,
            frame_range: FrameRange::new(FrameIndex::ZERO, FrameCount::from(10_000usize)),
        };
        let boundaries = vec![
            ChunkBoundary {
                start_time: 0.0,
                end_time: 0.25,
                frame_range: FrameRange::from_times(0.0, 0.25, sample_rate),
                source_segment_ids: vec![0],
            },
            ChunkBoundary {
                start_time: 0.25,
                end_time: 1.0,
                frame_range: FrameRange::from_times(0.25, 1.0, sample_rate),
                source_segment_ids: vec![1],
            },
        ];

        let chunks = slice_audio(&audio, &boundaries);
        let total_frames: usize = chunks
            .iter()
            .map(|chunk| chunk.frame_range.length.as_usize())
            .sum();
        assert_eq!(total_frames, audio.frame_range.length.as_usize());
        assert_eq!(chunks[0].frame_range.start.as_usize(), 0);
        assert_eq!(
            chunks[1].frame_range.start.as_usize(),
            chunks[0].frame_range.end().as_usize()
        );
    }
}
