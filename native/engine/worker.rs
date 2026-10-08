//! One inference worker; no periodic wakeups when there is no resident model.
use super::{Backend, Cancellation, Engine, Error, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct Completion {
    pub id: u64,
    pub result: Result<String>,
    pub backend: String,
    pub load_ms: u128,
    pub inference_ms: u128,
}
enum Command {
    Run {
        id: u64,
        samples: Vec<f32>,
        cancel: Cancellation,
    },
    Unload,
    Close,
}
pub struct Worker {
    commands: SyncSender<Command>,
    pub completions: Receiver<Completion>,
    join: Option<JoinHandle<()>>,
    current_cancel: Option<Cancellation>,
    unload_requested: Arc<AtomicBool>,
}
impl Worker {
    pub fn start(path: PathBuf, backend: Backend) -> Self {
        let (commands, incoming) = mpsc::sync_channel(1);
        let (outgoing, completions) = mpsc::channel();
        let unload_requested = Arc::new(AtomicBool::new(false));
        let unload_flag = Arc::clone(&unload_requested);
        let join = thread::spawn(move || {
            let mut engine: Option<Engine> = None;
            loop {
                if unload_flag.swap(false, Ordering::SeqCst) {
                    engine = None;
                }
                let command = if engine.is_some() {
                    match incoming.recv_timeout(Duration::from_secs(30)) {
                        Ok(command) => command,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            engine = None;
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                } else {
                    match incoming.recv() {
                        Ok(command) => command,
                        Err(_) => break,
                    }
                };
                match command {
                    Command::Run {
                        id,
                        samples,
                        cancel,
                    } => {
                        let start = Instant::now();
                        let loaded = if cancel.cancelled() {
                            Err(Error::Cancelled)
                        } else if engine.is_none() {
                            Engine::load(&path, backend).map(|value| engine = Some(value))
                        } else {
                            Ok(())
                        };
                        let mut load_ms = start.elapsed().as_millis();
                        let start = Instant::now();
                        let mut result = loaded.and_then(|()| {
                            engine
                                .as_mut()
                                .ok_or_else(|| Error::message("No engine"))?
                                .run(&samples, &cancel)
                        });
                        let mut inference_ms = start.elapsed().as_millis();
                        // Vulkan can fail during allocation/compute after loading
                        // successfully. Release its model before retrying on CPU.
                        if matches!(result, Err(Error::Message(_)))
                            && !cancel.cancelled()
                            && engine
                                .as_ref()
                                .is_some_and(|e| e.backend.starts_with("Vulkan"))
                        {
                            engine = None;
                            let start = Instant::now();
                            let loaded =
                                Engine::load(&path, Backend::Cpu).map(|value| engine = Some(value));
                            load_ms += start.elapsed().as_millis();
                            let start = Instant::now();
                            result = loaded.and_then(|()| {
                                engine
                                    .as_mut()
                                    .ok_or_else(|| Error::message("No CPU engine"))?
                                    .run(&samples, &cancel)
                            });
                            inference_ms += start.elapsed().as_millis();
                        }
                        let backend = engine
                            .as_ref()
                            .map(|e| e.backend.clone())
                            .unwrap_or_default();
                        if outgoing
                            .send(Completion {
                                id,
                                result,
                                backend,
                                load_ms,
                                inference_ms,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Command::Unload => engine = None,
                    Command::Close => break,
                }
            }
        });
        Self {
            commands,
            completions,
            join: Some(join),
            current_cancel: None,
            unload_requested,
        }
    }
    pub fn submit(&mut self, id: u64, samples: Vec<f32>) -> Result<Cancellation> {
        crate::audio::validate(&samples)?;
        self.cancel();
        let cancel = Cancellation::default();
        self.commands
            .try_send(Command::Run {
                id,
                samples,
                cancel: cancel.clone(),
            })
            .map_err(Error::message)?;
        self.current_cancel = Some(cancel.clone());
        Ok(cancel)
    }
    pub fn cancel(&self) {
        if let Some(cancel) = &self.current_cancel {
            cancel.cancel();
        }
    }
    pub fn unload(&self) -> Result<()> {
        self.cancel();
        self.unload_requested.store(true, Ordering::SeqCst);
        match self.commands.try_send(Command::Unload) {
            Ok(()) | Err(mpsc::TrySendError::Full(_)) => Ok(()),
            Err(error) => Err(Error::message(error)),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        let _ = self.commands.send(Command::Close);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
