mod application;
mod clipboard;
mod clipboard_snapshot;
mod diagnostics;
mod hotkey;
mod input;
mod overlay;
mod preferences;
mod runtime;
mod tray;
pub use runtime::run;

use windows::core::PCWSTR;
pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
pub fn pw(text: &[u16]) -> PCWSTR {
    PCWSTR(text.as_ptr())
}
