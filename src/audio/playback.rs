use std::io::BufReader;
use std::path::Path;

use anyhow::{Context, Result};
use rodio::buffer::SamplesBuffer;
use rodio::source::{Source, UniformSourceIterator};
use rodio::{Decoder, OutputStream, Sink};

use crate::types::AudioData;

pub fn play_file(path: &Path) -> Result<()> {
    let file = std::fs::File::open(path).with_context(|| format!("failed to open {:?}", path))?;
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader).context("unsupported audio format")?;
    let converted = decoder.convert_samples::<f32>();
    play_source(converted)
}

pub fn play_audio(data: &AudioData) -> Result<()> {
    let stereo = duplicate_to_stereo(&data.samples);
    let buffer = SamplesBuffer::new(2, data.sample_rate, stereo);
    play_source(buffer)
}

fn play_source<S>(source: S) -> Result<()>
where
    S: Source<Item = f32> + Send + 'static,
{
    let (_stream, handle) = OutputStream::try_default().context("failed to open output stream")?;
    let sink = Sink::try_new(&handle).context("failed to create sink")?;
    let unified = ensure_stereo(source);
    sink.append(unified);
    sink.set_volume(1.0);
    sink.sleep_until_end();
    Ok(())
}

fn ensure_stereo<S>(source: S) -> Box<dyn Source<Item = f32> + Send>
where
    S: Source<Item = f32> + Send + 'static,
{
    if source.channels() == 2 {
        Box::new(source)
    } else {
        let sample_rate = source.sample_rate();
        Box::new(UniformSourceIterator::new(source, 2, sample_rate))
    }
}

pub fn duplicate_to_stereo(samples: &[f32]) -> Vec<f32> {
    let mut output = Vec::with_capacity(samples.len() * 2);
    for &sample in samples {
        output.push(sample);
        output.push(sample);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::duplicate_to_stereo;

    #[test]
    fn replicates_each_sample_into_two_channels() {
        let stereo = duplicate_to_stereo(&[0.3, -0.3]);
        assert_eq!(stereo, vec![0.3, 0.3, -0.3, -0.3]);
    }
}
