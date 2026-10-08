//! Local WAV regression entry point; Windows tray follows in the next milestone.
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
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
