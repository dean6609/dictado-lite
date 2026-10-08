//! Explicit local diagnostics only. Normal use writes no UI event log.
use std::io::Write;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*};
pub fn event(tag: &str) {
    append(serde_json::json!({"event":tag}));
}
pub fn window(tag: &str, hwnd: HWND) {
    unsafe {
        let mut rect = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rect);
        append(
            serde_json::json!({"event":tag,"visible":IsWindowVisible(hwnd).as_bool(),"dpi":GetDpiForWindow(hwnd),"x":rect.left,"y":rect.top,"width":rect.right-rect.left,"height":rect.bottom-rect.top}),
        );
    }
}
fn append(value: serde_json::Value) {
    if let Some(path) = std::env::var_os("DICTADO_UI_TRACE") {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{value}");
        }
    }
}
