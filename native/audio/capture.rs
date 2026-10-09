//! CPAL capture: preallocated mono ring, no locks/file work/allocations in callback.
use crate::engine::{Error, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Producer, RingBuffer};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub struct Levels {
    pub bars: [AtomicU32; 9],
    pub error: AtomicU8,
}
impl Default for Levels {
    fn default() -> Self {
        Self {
            bars: std::array::from_fn(|_| AtomicU32::new(0)),
            error: AtomicU8::new(0),
        }
    }
}
impl Levels {
    pub fn read(&self) -> [f32; 9] {
        std::array::from_fn(|i| f32::from_bits(self.bars[i].load(Ordering::Relaxed)))
    }
}
pub struct Capture {
    stream: Option<cpal::Stream>,
    collector: Option<JoinHandle<Vec<f32>>>,
    stop: Arc<AtomicBool>,
    rate: usize,
    pub levels: Arc<Levels>,
}
pub struct Recording {
    pub samples: Vec<f32>,
    pub rate: usize,
}

pub fn microphones() -> Result<Vec<String>> {
    cpal::default_host()
        .input_devices()
        .map_err(Error::message)?
        .map(|device| device.name().map_err(Error::message))
        .collect()
}
impl Capture {
    pub fn start(selected: Option<&str>) -> Result<Self> {
        let host = cpal::default_host();
        let device = if let Some(name) = selected {
            host.input_devices()
                .map_err(Error::message)?
                .find(|d| d.name().ok().as_deref() == Some(name))
        } else {
            host.default_input_device()
        }
        .ok_or_else(|| Error::message("Microphone unavailable"))?;
        let supported = device.default_input_config().map_err(Error::message)?;
        let config = supported.config();
        let rate = config.sample_rate.0 as usize;
        if !(8_000..=192_000).contains(&rate) || config.channels == 0 {
            return Err(Error::message("Unsupported microphone format"));
        }
        let (producer, mut consumer) = RingBuffer::new(rate * 2);
        let levels = Arc::new(Levels::default());
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, producer, &levels),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, producer, &levels),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, producer, &levels),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, producer, &levels),
            _ => return Err(Error::message("Unsupported microphone sample format")),
        }?;
        let stop = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&stop);
        let errors = Arc::clone(&levels);
        let collector = thread::spawn(move || {
            let mut samples = Vec::with_capacity(rate * 2);
            loop {
                // Observe stop before draining: close() has already stopped the
                // producer. Checking afterwards can lose the final callback's PCM.
                let stopping = done.load(Ordering::Acquire);
                while let Ok(sample) = consumer.pop() {
                    if samples.len() < rate * 60 * 10 {
                        samples.push(sample);
                    } else {
                        errors.error.store(1, Ordering::Relaxed);
                    }
                }
                if stopping {
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
            samples
        });
        let capture = Self {
            stream: Some(stream),
            collector: Some(collector),
            stop,
            rate,
            levels,
        };
        capture
            .stream
            .as_ref()
            .ok_or_else(|| Error::message("No input stream"))?
            .play()
            .map_err(Error::message)?;
        Ok(capture)
    }
    pub fn finish(mut self) -> Result<Recording> {
        let raw = self.close()?;
        if self.levels.error.load(Ordering::Relaxed) != 0 {
            return Err(Error::message(
                "Microphone disconnected or recording limit reached",
            ));
        }
        Ok(Recording {
            samples: raw,
            rate: self.rate,
        })
    }
    fn close(&mut self) -> Result<Vec<f32>> {
        drop(self.stream.take()); // no callback can publish new samples after close
        self.stop.store(true, Ordering::Release);
        match self.collector.take() {
            Some(thread) => thread
                .join()
                .map_err(|_| Error::message("Audio collector failed")),
            None => Ok(Vec::new()),
        }
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut producer: Producer<f32>,
    levels: &Arc<Levels>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let visual = Arc::clone(levels);
    let error = Arc::clone(levels);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let frames = data.len() / channels;
                if frames == 0 {
                    return;
                }
                let mut power = [0.0_f32; 9];
                let mut count = [0_usize; 9];
                for (i, frame) in data.chunks_exact(channels).enumerate() {
                    let sample =
                        frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() / channels as f32;
                    if !sample.is_finite() || producer.push(sample).is_err() {
                        visual.error.store(1, Ordering::Relaxed);
                    }
                    let bin = (i * 9 / frames).min(8);
                    power[bin] += sample * sample;
                    count[bin] += 1;
                }
                for i in 0..9 {
                    visual.bars[i].store(
                        (power[i] / count[i].max(1) as f32).sqrt().to_bits(),
                        Ordering::Relaxed,
                    );
                }
            },
            move |_| error.error.store(2, Ordering::Relaxed),
            None,
        )
        .map_err(Error::message)
}
