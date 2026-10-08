//! Native dictation foundation. No web runtime or persistent audio storage.

/// Audio accepted by the recognizer must be mono at this rate.
pub const RECOGNITION_SAMPLE_RATE: u32 = 16_000;

pub mod audio;
pub mod cleanup;
pub mod config;
pub mod engine;
#[cfg(windows)]
pub mod platform;
pub mod session;
