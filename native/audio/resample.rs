//! FFT conversion and delay-line draining adapted from Handy's MIT audio toolkit.
use crate::engine::{Error, Result};
use rubato::{FftFixedIn, Resampler};

pub fn to_16k(samples: &[f32], rate: usize) -> Result<Vec<f32>> {
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err(Error::message("Non-finite microphone samples"));
    }
    if rate == 16_000 {
        return Ok(samples.to_vec());
    }
    if !(8_000..=192_000).contains(&rate) {
        return Err(Error::message("Unsupported microphone sample rate"));
    }
    let mut resampler = FftFixedIn::<f32>::new(rate, 16_000, 1024, 1, 1).map_err(Error::message)?;
    let delay = resampler.output_delay();
    let expected = samples.len() * 16_000 / rate;
    let mut output = Vec::with_capacity(expected + delay + 2048);
    let (chunks, remainder) = samples.as_chunks::<1024>();
    for chunk in chunks {
        output.extend(
            resampler
                .process(&[&chunk[..]], None)
                .map_err(Error::message)?
                .remove(0),
        );
    }
    if !remainder.is_empty() {
        output.extend(
            resampler
                .process_partial(Some(&[remainder]), None)
                .map_err(Error::message)?
                .remove(0),
        );
    }
    for _ in 0..8 {
        if output.len() >= expected + delay {
            break;
        }
        output.extend(
            resampler
                .process_partial::<&[f32]>(None, None)
                .map_err(Error::message)?
                .remove(0),
        );
    }
    if output.len() < expected + delay {
        return Err(Error::message("Incomplete resampler tail"));
    }
    Ok(output[delay..delay + expected]
        .iter()
        .map(|s| s.clamp(-1.0, 1.0))
        .collect())
}

/// Earshot observes full frames without removing samples or cutting syllables.
pub fn voice_score(samples: &[f32]) -> f32 {
    let mut detector = earshot::Detector::default_boxed();
    samples
        .as_chunks::<256>()
        .0
        .iter()
        .map(|frame| detector.predict_f32(frame))
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flush_preserves_tail_at_48k_and_44100() {
        for rate in [48_000, 44_100] {
            let mut input = vec![0.0; rate + 300];
            let end = input.len();
            input[end - 300..].fill(0.5);
            let output = to_16k(&input, rate).unwrap();
            assert_eq!(output.len(), input.len() * 16_000 / rate);
            assert!(output[output.len() - 80..].iter().any(|s| *s > 0.3));
            let silence = to_16k(&vec![0.0; rate], rate).unwrap();
            assert!(silence.iter().all(|s| s.abs() < 0.0001));
        }
    }
    #[test]
    fn silence_has_no_voice_and_samples_are_not_removed() {
        let silence = vec![0.0; 16_000];
        assert!(voice_score(&silence) < 0.5);
        assert_eq!(to_16k(&silence, 16_000).unwrap(), silence);
    }
}
