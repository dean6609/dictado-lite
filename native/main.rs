//! Native tray entry point and explicit local regression/diagnostic commands.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use dictado_lite::{
    audio,
    cleanup::{Conservative, TextCleaner},
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
    cleanup_us: u128,
}
const USAGE: &str = "Usage:
  dictado-lite <model.gguf> <mono-16k.wav> [--cpu] [--verify-model] [--cleanup] [--repeat=N] [--idle-after=N]
  dictado-lite --record [--model <model.gguf>] [--microphone <name>] [--seconds=N] [--cpu] [--raw]
  dictado-lite --download-model [<catalog-key>]
  dictado-lite --list-microphones | --capture-probe [--microphone <name>]";

fn main() {
    // The release build has no console of its own; terminal commands print
    // into the console that launched them.
    #[cfg(windows)]
    if std::env::args().len() > 1 {
        unsafe {
            let _ = windows::Win32::System::Console::AttachConsole(
                windows::Win32::System::Console::ATTACH_PARENT_PROCESS,
            );
        }
    }
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
fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--download-model") {
        // Without a key, fetch the model the settings select (the recommended
        // one on a fresh install), so installers need no model list of their own.
        let model = match args.get(1).filter(|arg| !arg.starts_with("--")) {
            Some(key) => dictado_lite::models::catalog()
                .iter()
                .find(|m| m.key() == *key)
                .ok_or("Unknown catalog model")?,
            None => dictado_lite::models::selected(&dictado_lite::config::Config::load()?.model_id),
        };
        let cancel_after = args
            .iter()
            .find_map(|a| a.strip_prefix("--cancel-after-bytes="))
            .map(str::parse::<u64>)
            .transpose()?;
        // Progress goes to a terminal only; the result line stays plain JSON.
        let show = std::io::IsTerminal::is_terminal(&std::io::stderr());
        let mut shown = None;
        let result = dictado_lite::models::download(model, |done, total| {
            let percent = done * 100 / total.max(1);
            if show && shown != Some(percent) {
                shown = Some(percent);
                eprint!("\rDownloading {}: {percent}%", model.weights().filename);
            }
            cancel_after.is_none_or(|limit| done < limit)
        });
        if shown.is_some() {
            eprintln!();
        }
        match result {
            Ok(path) => {
                dictado_lite::models::verify(&path, model.weights())?;
                println!(
                    "{}",
                    serde_json::json!({"model":model.key(),"path":path,"verified":true,"bytes":model.weights().size_bytes})
                );
            }
            Err(engine::Error::Cancelled) => println!("{}", serde_json::json!({"cancelled":true})),
            Err(error) => return Err(error.into()),
        }
        return Ok(());
    }
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
        let selected = option(&args, "--microphone");
        let capture = audio::capture::Capture::start(selected.as_deref())?;
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
    if args.first().is_some_and(|arg| arg == "--record") {
        return record(&args);
    }
    #[cfg(windows)]
    if args.is_empty() || args[0].starts_with("--") {
        return dictado_lite::platform::windows::run(&args);
    }
    if args.len() < 2 || args[0].starts_with("--") {
        return Err(USAGE.into());
    }
    let model = PathBuf::from(&args[0]);
    let wav = PathBuf::from(&args[1]);
    let backend = backend(&args);
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
        let text = result.result?;
        let cleanup = std::time::Instant::now();
        let (text, cleanup_us) = if args.iter().any(|a| a == "--cleanup") {
            let text = Conservative::load().clean(&text);
            (text, cleanup.elapsed().as_micros())
        } else {
            (text, 0)
        };
        println!(
            "{}",
            serde_json::to_string(&Measurement {
                backend: result.backend,
                load_ms: result.load_ms,
                inference_ms: result.inference_ms,
                text,
                cleanup_us,
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
fn backend(args: &[String]) -> Backend {
    if args.iter().any(|a| a == "--cpu") {
        Backend::Cpu
    } else {
        Backend::Gpu
    }
}
/// Terminal dictation for any desktop: record until Enter (or a time limit),
/// recognize locally and print the text. Nothing is typed into another app.
fn record(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let config = dictado_lite::config::Config::load()?;
    let model = match option(args, "--model") {
        Some(path) => PathBuf::from(path),
        None => {
            let selected = dictado_lite::models::selected(&config.model_id);
            let cache = selected.cache_path()?;
            if !selected.ready(&cache) {
                return Err(format!(
                    "Model not downloaded. Run: dictado-lite --download-model {}",
                    selected.key()
                )
                .into());
            }
            cache
        }
    };
    let seconds = args
        .iter()
        .find_map(|a| a.strip_prefix("--seconds="))
        .map(str::parse::<u64>)
        .transpose()?
        .map(|s| s.clamp(1, 600));
    let microphone = option(args, "--microphone").or(config.microphone.clone());
    let capture = audio::capture::Capture::start(microphone.as_deref())?;
    match seconds {
        Some(seconds) => {
            eprintln!("Recording for {seconds} s...");
            std::thread::sleep(std::time::Duration::from_secs(seconds));
        }
        None => {
            eprintln!("Recording... press Enter to recognize.");
            let mut line = String::new();
            std::io::stdin().read_line(&mut line)?;
        }
    }
    let recording = capture.finish()?;
    eprintln!("Recognizing...");
    let mut worker = Worker::start(model, backend(args));
    worker.submit_recording(1, recording)?;
    let text = worker.completions.recv()?.result?;
    let text = if args.iter().any(|a| a == "--raw") || !config.cleanup {
        text
    } else {
        Conservative::load().clean(&text)
    };
    if text.trim().is_empty() {
        return Err("No text was recognized".into());
    }
    println!("{text}");
    Ok(())
}
