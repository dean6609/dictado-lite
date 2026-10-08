//! UI-thread coordinator; inference, audio collection and clipboard pumping are
//! separate owners. Only the current, uncancelled session can reach insertion.
use super::{clipboard::Paste, hotkey, input, pw, tray::Tray, wide};
use crate::{
    audio::{self, capture::Capture},
    config::Config,
    engine::{Backend, Worker},
    session::Sessions,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Copy, PartialEq)]
pub enum Phase {
    Idle,
    Listening,
    Processing,
    Pasting,
}
pub struct Application {
    pub hwnd: HWND,
    pub config: Config,
    pub tray: Tray,
    pub paused: bool,
    pub phase: Phase,
    pub last_text: Option<String>,
    worker: Worker,
    sessions: Sessions,
    capture: Option<Capture>,
    id: Option<u64>,
    target: HWND,
    paste: Option<Paste>,
    injected: bool,
    released: Option<Instant>,
    metrics: Option<PathBuf>,
    stop_after: Option<Duration>,
    began: Option<Instant>,
    recognition: Option<serde_json::Value>,
}
impl Application {
    pub fn new(
        hwnd: HWND,
        config: Config,
        model: PathBuf,
        metrics: Option<PathBuf>,
        stop_after: Option<Duration>,
    ) -> windows::core::Result<Self> {
        hotkey::configure(config.key, config.modifiers);
        Ok(Self {
            hwnd,
            config,
            tray: Tray::create(hwnd)?,
            paused: false,
            phase: Phase::Idle,
            last_text: None,
            worker: Worker::start(model, Backend::PreferVulkan),
            sessions: Sessions::default(),
            capture: None,
            id: None,
            target: HWND::default(),
            paste: None,
            injected: false,
            released: None,
            metrics,
            stop_after,
            began: None,
            recognition: None,
        })
    }
    fn begin(&mut self, target: HWND) -> Option<u64> {
        if self.phase != Phase::Idle || self.paused {
            return None;
        }
        let id = self.sessions.begin()?;
        if id > u32::MAX as u64 {
            self.sessions.cancel();
            return None;
        }
        self.id = Some(id);
        self.target = target;
        self.last_text = None;
        self.recognition = None;
        hotkey::active(Some(id));
        Some(id)
    }
    pub fn start(&mut self, target: HWND) {
        if self.begin(target).is_none() {
            return;
        }
        match Capture::start(self.config.microphone.as_deref()) {
            Ok(capture) => {
                self.capture = Some(capture);
                self.began = Some(Instant::now());
                self.phase = Phase::Listening;
                self.timer();
            }
            Err(error) => self.fail(&format!("No se pudo abrir el micrófono. {error}")),
        }
    }
    pub fn wav(&mut self, path: PathBuf, target: HWND) {
        let Some(id) = self.begin(target) else {
            return;
        };
        self.released = Some(Instant::now());
        match audio::read_wav(&path).and_then(|samples| self.worker.submit(id, samples)) {
            Ok(_) => {
                self.phase = Phase::Processing;
                self.timer();
            }
            Err(error) => self.fail(&error.to_string()),
        }
    }
    pub fn stop(&mut self) {
        let Some(capture) = self.capture.take() else {
            return;
        };
        self.released = Some(Instant::now());
        let Some(id) = self.id else {
            self.cancel();
            return;
        };
        match capture
            .finish()
            .and_then(|recording| self.worker.submit_recording(id, recording))
        {
            Ok(_) => {
                self.phase = Phase::Processing;
                self.timer();
            }
            Err(error) => self.fail(&error.to_string()),
        }
    }
    pub fn cancel(&mut self) {
        hotkey::cancel();
        hotkey::active(None);
        self.sessions.cancel();
        drop(self.capture.take());
        self.worker.cancel();
        let _ = self.worker.unload();
        drop(self.paste.take());
        self.id = None;
        self.phase = Phase::Idle;
        self.last_text = None;
        self.timer();
    }
    pub fn pause(&mut self, paused: bool) {
        self.cancel();
        self.paused = paused;
        self.sessions.set_paused(paused);
        hotkey::pause(paused);
    }
    pub fn save(&mut self) {
        hotkey::configure(self.config.key, self.config.modifiers);
        if let Err(error) = self.config.save() {
            self.fail(&error.to_string());
        }
    }
    pub fn tick(&mut self, menu_open: bool) {
        if hotkey::cancel_requested() && self.phase != Phase::Idle {
            self.cancel();
            return;
        }
        if self.capture.as_ref().is_some_and(|capture| {
            capture
                .levels
                .error
                .load(std::sync::atomic::Ordering::Relaxed)
                != 0
        }) {
            self.fail("El micrófono se desconectó o se alcanzó el límite de grabación.");
            return;
        }
        if self.phase == Phase::Listening
            && self
                .stop_after
                .is_some_and(|limit| self.began.is_some_and(|at| at.elapsed() >= limit))
        {
            self.stop();
        }
        while let Ok(completion) = self.worker.completions.try_recv() {
            if !self.sessions.accept(completion.id) || self.id != Some(completion.id) {
                continue;
            }
            match completion.result {
                Ok(text) if !text.trim().is_empty() => {
                    self.recognition = Some(
                        serde_json::json!({"backend":completion.backend,"load_ms":completion.load_ms,"inference_ms":completion.inference_ms,"voice_score":completion.voice_score}),
                    );
                    let text = if self.config.cleanup && !text.contains(['`', '{', '}', '\n']) {
                        text.split_whitespace().collect::<Vec<_>>().join(" ")
                    } else {
                        text
                    };
                    self.last_text = Some(text.clone());
                    if !menu_open && unsafe { GetForegroundWindow() } != self.target {
                        self.fail("Cambió la ventana de destino. El texto no se ha pegado.");
                        return;
                    }
                    self.paste = Some(Paste::start(text));
                    self.injected = false;
                    self.phase = Phase::Pasting;
                }
                Ok(_) => self.fail("No se detectó texto. Vuelve a intentarlo."),
                Err(error) => self.fail(&error.to_string()),
            }
        }
        if self.phase == Phase::Pasting && !self.injected && input::modifiers_released() {
            let ready = self
                .paste
                .as_ref()
                .and_then(|paste| paste.published.try_recv().ok());
            if let Some(ready) = ready {
                match ready.and_then(|()| {
                    self.paste
                        .as_ref()
                        .ok_or("No clipboard transaction")?
                        .inject(self.target)
                }) {
                    Ok(()) => self.injected = true,
                    Err(error) => {
                        self.fail(&error);
                        return;
                    }
                }
            }
        }
        let outcome = self
            .paste
            .as_ref()
            .and_then(|paste| paste.outcomes.try_recv().ok());
        if let Some(outcome) = outcome {
            drop(self.paste.take());
            match outcome.result {
                Ok(()) => {
                    if let (Some(path), Some(released), Some(received)) =
                        (&self.metrics, self.released, outcome.received_at)
                    {
                        let value = serde_json::json!({"scope":"release to post-injection clipboard read (receipt proxy)","release_to_clipboard_read_ms":received.saturating_duration_since(released).as_millis(),"recognition":self.recognition});
                        let _ = std::fs::write(path, value.to_string());
                    }
                    self.last_text = None;
                    self.id = None;
                    self.phase = Phase::Idle;
                    hotkey::active(None);
                }
                Err(error) => self.fail(&error),
            }
        }
        self.timer();
    }
    pub fn copy(&mut self) {
        if let Some(text) = self.last_text.as_deref() {
            if let Err(error) = super::clipboard::copy(self.hwnd, text) {
                let message = wide(&format!("No se pudo copiar. {error}"));
                unsafe {
                    MessageBoxW(
                        None,
                        pw(&message),
                        w!("Dictado Lite"),
                        MB_OK | MB_ICONWARNING,
                    );
                }
            }
        }
    }
    pub fn restore_target(&self) {
        unsafe {
            let _ = SetForegroundWindow(self.target);
        }
    }
    fn fail(&mut self, error: &str) {
        let retained = self.last_text.take();
        self.cancel();
        self.last_text = retained;
        let message = if self.last_text.is_some() {
            format!("{error}\n\nEl resultado sigue disponible. ¿Copiarlo al portapapeles?")
        } else {
            error.to_string()
        };
        let message = wide(&message);
        let action = unsafe {
            MessageBoxW(
                None,
                pw(&message),
                w!("Dictado Lite"),
                if self.last_text.is_some() {
                    MB_YESNO | MB_ICONWARNING
                } else {
                    MB_OK | MB_ICONWARNING
                },
            )
        };
        if action == IDYES {
            self.copy();
        }
    }
    fn timer(&self) {
        unsafe {
            if self.phase == Phase::Idle {
                let _ = KillTimer(Some(self.hwnd), 1);
            } else {
                SetTimer(Some(self.hwnd), 1, 33, None);
            }
        }
    }
}
impl Drop for Application {
    fn drop(&mut self) {
        self.cancel();
    }
}
