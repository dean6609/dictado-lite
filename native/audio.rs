//! File input for local regression. Capture is added in the Windows milestone.
use crate::engine::{Error, Result};
use crate::RECOGNITION_SAMPLE_RATE;
use std::path::Path;

pub mod capture;
pub mod resample;

pub fn read_wav(path: &Path) -> Result<Vec<f32>> {
    let mut wav = hound::WavReader::open(path).map_err(Error::message)?;
    let spec = wav.spec();
    if spec.channels != 1 || spec.sample_rate != RECOGNITION_SAMPLE_RATE {
        return Err(Error::message("WAV must be mono at 16 kHz"));
    }
    let samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Float, 32) => wav
            .samples::<f32>()
            .collect::<std::result::Result<_, _>>()
            .map_err(Error::message)?,
        (hound::SampleFormat::Int, 16) => wav
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<std::result::Result<_, _>>()
            .map_err(Error::message)?,
        _ => return Err(Error::message("WAV must be PCM16 or float32")),
    };
    validate(&samples)?;
    Ok(samples)
}

pub fn validate(samples: &[f32]) -> Result<()> {
    if samples.is_empty() || samples.iter().any(|s| !s.is_finite() || s.abs() > 1.0) {
        return Err(Error::message(
            "Audio must be nonempty, finite PCM in [-1, 1]",
        ));
    }
    if samples.len() > RECOGNITION_SAMPLE_RATE as usize * 60 * 10 {
        return Err(Error::message("Maximum recording is ten minutes"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_audio_before_native_code() {
        for audio in [&[][..], &[f32::NAN], &[f32::INFINITY], &[1.1]] {
            assert!(validate(audio).is_err());
        }
        assert!(validate(&[0.0, -1.0, 1.0]).is_ok());
    }
}
