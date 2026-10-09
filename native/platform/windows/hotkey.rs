//! One toggle per shortcut press; release and auto-repeat never stop dictation.
use super::runtime::{WM_CANCEL, WM_TOGGLE};
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU8, Ordering};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

static WINDOW: AtomicIsize = AtomicIsize::new(0);
static KEY: AtomicU32 = AtomicU32::new(0x20);
static MODIFIERS: AtomicU8 = AtomicU8::new(2);
static HOLDING: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static EPOCH: AtomicU32 = AtomicU32::new(0);
static CANCELLED: AtomicU32 = AtomicU32::new(0);
const MAGIC: usize = 0xD1C7_AD00;

pub struct Hook(HHOOK);
impl Hook {
    pub fn install(hwnd: HWND) -> windows::core::Result<Self> {
        WINDOW.store(hwnd.0 as isize, Ordering::Release);
        unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(callback),
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                0,
            )
            .map(Self)
        }
    }
}
impl Drop for Hook {
    fn drop(&mut self) {
        unsafe {
            let _ = UnhookWindowsHookEx(self.0);
        }
        WINDOW.store(0, Ordering::Release);
    }
}
pub fn configure(key: u16, modifiers: u8) {
    KEY.store(key as u32, Ordering::Release);
    MODIFIERS.store(modifiers, Ordering::Release);
}
pub fn pause(value: bool) {
    PAUSED.store(value, Ordering::Release);
    if value {
        HOLDING.store(false, Ordering::Release);
    }
}
pub fn active(id: Option<u64>) {
    if let Some(id) = id {
        EPOCH.store(id as u32, Ordering::Release);
    }
    ACTIVE.store(id.is_some(), Ordering::Release);
}
pub fn cancel_requested() -> bool {
    let epoch = EPOCH.load(Ordering::Acquire);
    epoch > 0 && CANCELLED.load(Ordering::Acquire) >= epoch
}
pub fn cancel() {
    CANCELLED.fetch_max(EPOCH.load(Ordering::Acquire), Ordering::SeqCst);
}
pub fn input_marker() -> usize {
    ((EPOCH.load(Ordering::Acquire) as usize) << 32) | MAGIC
}

unsafe extern "system" fn callback(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let event = &*(lp.0 as *const KBDLLHOOKSTRUCT);
        let down = wp.0 as u32 == WM_KEYDOWN || wp.0 as u32 == WM_SYSKEYDOWN;
        let up = wp.0 as u32 == WM_KEYUP || wp.0 as u32 == WM_SYSKEYUP;
        if event.dwExtraInfo & 0xFFFF_FFFF == MAGIC
            && event.vkCode == 0x56
            && (event.dwExtraInfo >> 32) as u32 <= CANCELLED.load(Ordering::Acquire)
        {
            return LRESULT(1);
        }
        let hwnd = HWND(WINDOW.load(Ordering::Acquire) as *mut _);
        if down && event.vkCode == VK_ESCAPE.0 as u32 && ACTIVE.load(Ordering::Acquire) {
            cancel();
            let _ = PostMessageW(Some(hwnd), WM_CANCEL, WPARAM(0), LPARAM(0));
            return LRESULT(1);
        }
        if !PAUSED.load(Ordering::Acquire) && event.vkCode == KEY.load(Ordering::Acquire) {
            if up && HOLDING.swap(false, Ordering::AcqRel) {
                return LRESULT(1);
            }
            // Keep consuming repeats until the trigger key is released, even
            // if Ctrl/Alt was released first. No stray spaces reach the editor.
            if down && HOLDING.load(Ordering::Acquire) {
                return LRESULT(1);
            }
            let mods = MODIFIERS.load(Ordering::Acquire);
            let matches = [(1, VK_SHIFT), (2, VK_CONTROL), (4, VK_MENU)]
                .into_iter()
                .all(|(bit, key)| (GetAsyncKeyState(key.0 as i32) < 0) == (mods & bit != 0));
            if down && matches {
                if !HOLDING.swap(true, Ordering::AcqRel) {
                    let _ = PostMessageW(Some(hwnd), WM_TOGGLE, WPARAM(0), LPARAM(0));
                }
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
