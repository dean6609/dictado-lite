//! A dedicated clipboard owner pumps delayed-render receipts without borrowing
//! UI state across reentrant Win32 calls. Strategy adapted from Handy (MIT).
use super::{
    clipboard_snapshot::{publish_bytes, Open, Snapshot},
    input, pw, wide,
};
use std::cell::{Cell, RefCell};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc, Mutex, Once,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::System::DataExchange::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::UI::WindowsAndMessaging::*;

struct Flags {
    cancelled: AtomicBool,
    failed: AtomicBool,
    injected: Mutex<Option<Instant>>,
}
pub struct Outcome {
    pub result: Result<(), String>,
    pub received_at: Option<Instant>,
}
pub struct Paste {
    pub published: Receiver<Result<(), String>>,
    pub outcomes: Receiver<Outcome>,
    flags: Arc<Flags>,
    thread: Option<JoinHandle<()>>,
}
impl Paste {
    pub fn start(text: String) -> Self {
        let flags = Arc::new(Flags {
            cancelled: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            injected: Mutex::new(None),
        });
        let worker_flags = Arc::clone(&flags);
        let (ready, published) = mpsc::channel();
        let (result, outcomes) = mpsc::channel();
        let thread = thread::spawn(move || {
            if let Err(error) = pump(text, worker_flags, result, &ready) {
                let _ = ready.send(Err(error));
            }
        });
        Self {
            published,
            outcomes,
            flags,
            thread: Some(thread),
        }
    }
    pub fn inject(&self, target: HWND) -> Result<(), String> {
        if self.flags.cancelled.load(Ordering::Acquire) || super::hotkey::cancel_requested() {
            return Err("Cancelled".into());
        }
        *self
            .flags
            .injected
            .lock()
            .map_err(|_| "Clipboard state failed")? = Some(Instant::now());
        if let Err(error) = input::paste(target) {
            self.flags.failed.store(true, Ordering::Release);
            return Err(error);
        }
        Ok(())
    }
    pub fn cancel(&self) {
        self.flags.cancelled.store(true, Ordering::Release);
    }
}
impl Drop for Paste {
    fn drop(&mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            // Publication may synchronously notify an original clipboard owner
            // on this thread. Service sent messages while joining, without
            // consuming posted application commands or reentering its state.
            while !thread.is_finished() {
                unsafe {
                    let mut message = MSG::default();
                    let _ = PeekMessageW(&mut message, None, WM_NULL, WM_NULL, PM_NOREMOVE);
                }
                thread::sleep(Duration::from_millis(2));
            }
            let _ = thread.join();
        }
    }
}

struct Owner {
    text: Vec<u16>,
    original: RefCell<Option<Snapshot>>,
    sequence: Cell<u32>,
    lost: Cell<bool>,
    receipt: Cell<Option<Instant>>,
    rendered: Cell<bool>,
    retries: Cell<u32>,
    busy: Cell<bool>,
    flags: Arc<Flags>,
    outcomes: Sender<Outcome>,
    started: Instant,
}
impl Owner {
    fn render(&self, receipt: bool) -> Result<(), String> {
        // SAFETY: UTF-16 buffer is immutable, lives through the synchronous copy.
        let bytes = unsafe {
            std::slice::from_raw_parts(self.text.as_ptr().cast::<u8>(), self.text.len() * 2)
        };
        publish_bytes(CF_UNICODETEXT.0 as u32, bytes)?;
        self.sequence.set(unsafe { GetClipboardSequenceNumber() });
        // Windows can advance the sequence after the render callback returns.
        self.rendered.set(true);
        if receipt {
            self.receipt.set(Some(Instant::now()));
        }
        Ok(())
    }
    fn restore(&self, hwnd: HWND) -> Result<(), String> {
        if self.lost.get()
            || unsafe { GetClipboardSequenceNumber() } != self.sequence.get()
            || !unsafe { GetClipboardOwner() }.is_ok_and(|owner| owner == hwnd)
        {
            return Ok(());
        }
        let _guard = Open::new(hwnd)?;
        if unsafe { GetClipboardSequenceNumber() } == self.sequence.get() {
            if let Some(snapshot) = self.original.borrow_mut().as_mut() {
                let restored = snapshot.restore();
                self.sequence.set(unsafe { GetClipboardSequenceNumber() });
                restored?;
            }
        }
        self.lost.set(true);
        Ok(())
    }
    fn tick(&self, hwnd: HWND) {
        if self.busy.replace(true) {
            return;
        }
        if self.rendered.replace(false)
            && unsafe { GetClipboardOwner() }.is_ok_and(|owner| owner == hwnd)
        {
            self.sequence.set(unsafe { GetClipboardSequenceNumber() });
        }
        let injected = self.flags.injected.lock().ok().and_then(|value| *value);
        let receipt = self
            .receipt
            .get()
            .filter(|at| injected.is_some_and(|start| *at >= start));
        let failed = self.flags.failed.load(Ordering::Acquire);
        let cancelled = self.flags.cancelled.load(Ordering::Acquire);
        let deadline = if failed {
            Duration::from_millis(500)
        } else {
            Duration::from_secs(8)
        };
        let finished = cancelled
            || self.lost.get()
            || receipt.is_some_and(|at| at.elapsed() >= Duration::from_millis(200))
            || self.started.elapsed() >= deadline;
        if finished {
            let restoration = self.restore(hwnd);
            if restoration.is_err() && self.retries.get() < 80 {
                self.retries.set(self.retries.get() + 1);
                self.busy.set(false);
                return;
            }
            let result = restoration.and_then(|()| {
                if cancelled {
                    Err("Cancelled".into())
                } else if failed {
                    Err("Windows rejected paste input".into())
                } else if receipt.is_none() {
                    Err("Paste could not be confirmed; text is available to copy".into())
                } else {
                    Ok(())
                }
            });
            let _ = self.outcomes.send(Outcome {
                result,
                received_at: receipt,
            });
            unsafe {
                PostQuitMessage(0);
            }
        }
        self.busy.set(false);
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Owner;
    if !pointer.is_null() {
        let owner = &*pointer;
        match msg {
            WM_RENDERFORMAT => {
                if wp.0 == CF_UNICODETEXT.0 as usize {
                    let _ = owner.render(true);
                }
                return LRESULT(0);
            }
            WM_RENDERALLFORMATS => {
                if !owner.lost.get() {
                    if let Ok(_guard) = Open::new(hwnd) {
                        if GetClipboardOwner().is_ok_and(|value| value == hwnd) {
                            let _ = owner.render(false);
                        }
                    }
                }
                return LRESULT(0);
            }
            WM_DESTROYCLIPBOARD => {
                if !owner.busy.get() {
                    owner.lost.set(true);
                }
                return LRESULT(0);
            }
            WM_TIMER => {
                owner.tick(hwnd);
                return LRESULT(0);
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
fn pump(
    text: String,
    flags: Arc<Flags>,
    outcomes: Sender<Outcome>,
    ready: &Sender<Result<(), String>>,
) -> Result<(), String> {
    // Every Win32 resource/clipboard handle here is owned by this thread.
    unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None).map_err(|e| e.to_string())?.0);
        static CLASS: Once = Once::new();
        CLASS.call_once(|| {
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("DictadoClipboardOwner"),
                lpfnWndProc: Some(procedure),
                ..Default::default()
            });
        });
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("DictadoClipboardOwner"),
            w!("Dictado Clipboard"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(instance),
            None,
        )
        .map_err(|e| e.to_string())?;
        let owner = Box::new(Owner {
            text: wide(&text),
            original: RefCell::new(None),
            sequence: Cell::new(0),
            lost: Cell::new(false),
            receipt: Cell::new(None),
            rendered: Cell::new(false),
            retries: Cell::new(0),
            busy: Cell::new(false),
            flags,
            outcomes,
            started: Instant::now(),
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*owner) as *const Owner as isize);
        let published = (|| {
            let _guard = Open::new(hwnd)?;
            *owner.original.borrow_mut() = Some(Snapshot::capture()?);
            EmptyClipboard().map_err(|e| e.to_string())?;
            privacy_markers()?;
            SetLastError(ERROR_SUCCESS);
            if let Err(error) = SetClipboardData(CF_UNICODETEXT.0 as u32, None) {
                if error.code().is_err() {
                    return Err(error.to_string());
                }
            }
            owner.sequence.set(GetClipboardSequenceNumber());
            Ok(())
        })();
        if let Err(error) = published {
            // If publication failed after EmptyClipboard, restore while locked.
            if let Ok(_guard) = Open::new(hwnd) {
                if GetClipboardOwner().is_ok_and(|value| value == hwnd) {
                    if let Some(snapshot) = owner.original.borrow_mut().as_mut() {
                        let _ = snapshot.restore();
                    }
                }
            }
            let _ = DestroyWindow(hwnd);
            return Err(error);
        }
        owner.sequence.set(GetClipboardSequenceNumber());
        SetTimer(Some(hwnd), 1, 25, None);
        let _ = ready.send(Ok(()));
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result <= 0 {
                break;
            }
            DispatchMessageW(&message);
        }
        let _ = KillTimer(Some(hwnd), 1);
        let _ = DestroyWindow(hwnd);
    }
    Ok(())
}
fn privacy_markers() -> Result<(), String> {
    for (name, value) in [
        ("ExcludeClipboardContentFromMonitorProcessing", 1_u32),
        ("CanIncludeInClipboardHistory", 0),
        ("CanUploadToCloudClipboard", 0),
    ] {
        let name = wide(name);
        let format = unsafe { RegisterClipboardFormatW(pw(&name)) };
        if format == 0 {
            return Err("Clipboard privacy format failed".into());
        }
        publish_bytes(format, &value.to_le_bytes())?;
    }
    Ok(())
}
pub fn copy(hwnd: HWND, text: &str) -> Result<(), String> {
    let text = wide(text);
    let _guard = Open::new(hwnd)?;
    unsafe { EmptyClipboard() }.map_err(|e| e.to_string())?;
    privacy_markers()?;
    let bytes = unsafe { std::slice::from_raw_parts(text.as_ptr().cast::<u8>(), text.len() * 2) };
    publish_bytes(CF_UNICODETEXT.0 as u32, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::System::Memory::*;
    use windows::Win32::System::Ole::CF_BITMAP;

    unsafe extern "system" fn default_procedure(
        hwnd: HWND,
        msg: u32,
        wp: WPARAM,
        lp: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    unsafe fn bytes(format: u32) -> Vec<u8> {
        let handle = HGLOBAL(GetClipboardData(format).unwrap().0);
        let pointer = GlobalLock(handle);
        assert!(!pointer.is_null());
        let result = std::slice::from_raw_parts(pointer.cast::<u8>(), GlobalSize(handle)).to_vec();
        let _ = GlobalUnlock(handle);
        result
    }
    fn receive<T>(channel: &Receiver<T>) -> T {
        let started = Instant::now();
        loop {
            if let Ok(value) = channel.try_recv() {
                return value;
            }
            assert!(
                started.elapsed() < Duration::from_secs(3),
                "Clipboard protocol timed out"
            );
            unsafe {
                let mut message = MSG::default();
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&message);
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[ignore = "physical Windows clipboard; preserves original, run serially in a local test session"]
    fn receipt_restores_rich_bitmap_and_respects_new_copy() {
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
            RegisterClassW(&WNDCLASSW {
                hInstance: instance,
                lpszClassName: w!("DictadoClipboardTest"),
                lpfnWndProc: Some(default_procedure),
                ..Default::default()
            });
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("DictadoClipboardTest"),
                w!("Clipboard test"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance),
                None,
            )
            .unwrap();
            // Restore the user's original data even if an assertion unwinds.
            struct Original {
                hwnd: HWND,
                data: Snapshot,
            }
            impl Drop for Original {
                fn drop(&mut self) {
                    if let Ok(_guard) = Open::new(self.hwnd) {
                        let _ = self.data.restore();
                    }
                    unsafe {
                        let _ = DestroyWindow(self.hwnd);
                    }
                }
            }
            let original = {
                let _guard = Open::new(hwnd).unwrap();
                Original {
                    hwnd,
                    data: Snapshot::capture().unwrap(),
                }
            };
            let html = RegisterClipboardFormatW(w!("HTML Format"));
            let plain = wide("Original fixture");
            let plain_bytes =
                std::slice::from_raw_parts(plain.as_ptr().cast::<u8>(), plain.len() * 2);
            let rich = b"Version:0.9\r\n<b>Original fixture</b>\0";
            {
                let _guard = Open::new(hwnd).unwrap();
                EmptyClipboard().unwrap();
                publish_bytes(CF_UNICODETEXT.0 as u32, plain_bytes).unwrap();
                publish_bytes(html, rich).unwrap();
                let pixels = [0x00CC8844_u32; 4];
                let bitmap = CreateBitmap(2, 2, 1, 32, Some(pixels.as_ptr().cast()));
                assert!(!bitmap.0.is_null());
                SetClipboardData(CF_BITMAP.0 as u32, Some(HANDLE(bitmap.0))).unwrap();
            }
            let paste = Paste::start("Recognition fixture".into());
            receive(&paste.published).unwrap();
            *paste.flags.injected.lock().unwrap() = Some(Instant::now());
            {
                let _guard = Open::new(hwnd).unwrap();
                let rendered = bytes(CF_UNICODETEXT.0 as u32);
                assert!(rendered.starts_with(&[b'R', 0, b'e', 0]));
            }
            receive(&paste.outcomes).result.unwrap();
            drop(paste);
            {
                let _guard = Open::new(hwnd).unwrap();
                assert!(bytes(CF_UNICODETEXT.0 as u32).starts_with(plain_bytes));
                assert!(bytes(html).starts_with(rich));
                let bitmap = HBITMAP(GetClipboardData(CF_BITMAP.0 as u32).unwrap().0);
                let mut pixels = [0_u32; 4];
                assert_eq!(GetBitmapBits(bitmap, 16, pixels.as_mut_ptr().cast()), 16);
                assert_eq!(pixels, [0x00CC8844; 4]);
            }
            let paste = Paste::start("Second fixture".into());
            receive(&paste.published).unwrap();
            copy(hwnd, "New user copy fixture").unwrap();
            paste.cancel();
            let _ = receive(&paste.outcomes);
            drop(paste);
            {
                let _guard = Open::new(hwnd).unwrap();
                assert!(bytes(CF_UNICODETEXT.0 as u32).starts_with(&[b'N', 0, b'e', 0, b'w', 0]));
            }
            let paste = Paste::start("Cancelled before insertion".into());
            receive(&paste.published).unwrap();
            paste.cancel();
            assert!(receive(&paste.outcomes).result.is_err());
            drop(paste);
            {
                let _guard = Open::new(hwnd).unwrap();
                assert!(bytes(CF_UNICODETEXT.0 as u32).starts_with(&[b'N', 0, b'e', 0, b'w', 0]));
            }
            drop(original);
        }
    }
}
