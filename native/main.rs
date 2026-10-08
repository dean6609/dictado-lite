//! Native tray entry point and explicit local regression/diagnostic commands.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use dictado_lite::{
    audio,
    engine::{self, Backend, Worker},
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct Measurement {
    backend: String,
    load_ms: u128,
    inference_ms: u128,
    text: String,
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        #[cfg(windows)]
        if std::env::args().len() == 1 {
            let message: Vec<u16> = error.to_string().encode_utf16().chain(Some(0)).collect();
            unsafe {
                windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    None,
                    windows::core::PCWSTR(message.as_ptr()),
                    windows::core::w!("Dictado Lite"),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK
                        | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
                );
            }
        }
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--list-microphones") {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        println!(
            "{}",
            serde_json::json!({"devices":audio::capture::microphones()?,"default":host.default_input_device().and_then(|device| device.name().ok())})
        );
        return Ok(());
    }
    if args.first().is_some_and(|arg| arg == "--capture-probe") {
        let selected = args
            .windows(2)
            .find(|pair| pair[0] == "--microphone")
            .map(|pair| pair[1].as_str());
        let capture = audio::capture::Capture::start(selected)?;
        std::thread::sleep(std::time::Duration::from_secs(5));
        let recording = capture.finish()?;
        let rms = (recording
            .samples
            .iter()
            .map(|s| (*s as f64).powi(2))
            .sum::<f64>()
            / recording.samples.len().max(1) as f64)
            .sqrt();
        let peak = recording
            .samples
            .iter()
            .map(|s| s.abs())
            .fold(0.0_f32, f32::max);
        let samples = audio::resample::to_16k(&recording.samples, recording.rate)?;
        println!(
            "{}",
            serde_json::json!({"rate":recording.rate,"duration_secs":recording.samples.len() as f64 / recording.rate as f64,"rms":rms,"peak":peak,"voice_score":audio::resample::voice_score(&samples),"resampled_count":samples.len()})
        );
        return Ok(());
    }
    #[cfg(windows)]
    if args.is_empty() || args[0].starts_with("--") {
        return dictado_lite::platform::windows::run(&args);
    }
    if args.len() < 2 {
        return Err(
            "Usage: dictado-lite <model.gguf> <mono-16k.wav> [--cpu] [--verify-model] [--repeat=N] [--idle-after=N]"
                .into(),
        );
    }
    let model = PathBuf::from(&args[0]);
    let wav = PathBuf::from(&args[1]);
    let backend = if args.iter().any(|a| a == "--cpu") {
        Backend::Cpu
    } else {
        Backend::PreferVulkan
    };
    if args.iter().any(|a| a == "--verify-model") {
        engine::verify_model(&model)?;
    }
    let repeat: usize = args
        .iter()
        .find_map(|a| a.strip_prefix("--repeat="))
        .unwrap_or("1")
        .parse()?;
    if !(1..=20).contains(&repeat) {
        return Err("Repeat must be between 1 and 20".into());
    }
    let samples = audio::read_wav(&wav)?;
    let mut worker = Worker::start(model, backend);
    for id in 1..=repeat as u64 {
        worker.submit(id, samples.clone())?;
        let result = worker.completions.recv()?;
        println!(
            "{}",
            serde_json::to_string(&Measurement {
                backend: result.backend,
                load_ms: result.load_ms,
                inference_ms: result.inference_ms,
                text: result.result?
            })?
        );
    }
    drop(samples);
    if let Some(seconds) = args.iter().find_map(|a| a.strip_prefix("--idle-after=")) {
        let seconds: u64 = seconds.parse()?;
        if seconds > 120 {
            return Err("Idle measurement is limited to 120 seconds".into());
        }
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }
    Ok(())
}
