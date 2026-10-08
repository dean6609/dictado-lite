//! Single instance, message pump and tray-command routing.
use super::{
    application::{Application, Phase},
    hotkey, preferences, tray,
};
use crate::{audio::capture::microphones, config::Config};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::time::Duration;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const WM_START: u32 = WM_APP + 1;
pub const WM_RELEASE: u32 = WM_APP + 2;
pub const WM_CANCEL: u32 = WM_APP + 3;
pub const WM_TRAY: u32 = WM_APP + 4;
const WM_MENU: u32 = WM_APP + 5;
static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static LAST_TARGET: AtomicIsize = AtomicIsize::new(0);
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn focus(
    _: HWINEVENTHOOK,
    _: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    let mut class = [0_u16; 128];
    let size = GetClassNameW(hwnd, &mut class);
    let class = String::from_utf16_lossy(&class[..size.max(0) as usize]);
    if ![
        "Shell_TrayWnd",
        "Progman",
        "WorkerW",
        "NotifyIconOverflowWindow",
        "#32768",
    ]
    .contains(&class.as_str())
    {
        LAST_TARGET.store(hwnd.0 as isize, Ordering::Release);
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_DESTROY {
        PostQuitMessage(0);
        return LRESULT(0);
    }
    if msg == WM_QUERYENDSESSION {
        return LRESULT(1);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<Application>;
    if pointer.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let state = &*pointer;
    if msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed) {
        if let Ok(app) = state.try_borrow() {
            app.tray.refresh();
        }
        return LRESULT(0);
    }
    if msg == WM_MENU || (msg == WM_TRAY && [WM_RBUTTONUP, WM_LBUTTONUP].contains(&(lp.0 as u32))) {
        let view = state.try_borrow().ok().map(|app| {
            (
                app.config.clone(),
                app.phase == Phase::Listening,
                app.paused,
                app.last_text.is_some(),
            )
        });
        let Some((config, recording, paused, copy)) = view else {
            return LRESULT(0);
        };
        let devices = microphones().unwrap_or_default();
        MENU_OPEN.store(true, Ordering::Release);
        let chosen = tray::menu(
            hwnd,
            &config,
            recording,
            paused,
            copy,
            &devices,
            preferences::autostart(),
        );
        MENU_OPEN.store(false, Ordering::Release);
        if chosen == tray::SHORTCUT {
            if let Ok(mut app) = state.try_borrow_mut() {
                app.cancel();
            }
            hotkey::pause(true);
            let shortcut = preferences::shortcut(&config);
            if let Ok(mut app) = state.try_borrow_mut() {
                if let Ok(Some((key, mods))) = shortcut {
                    app.config.key = key;
                    app.config.modifiers = mods;
                    app.save();
                }
                hotkey::pause(app.paused);
            }
        } else if let Ok(mut app) = state.try_borrow_mut() {
            match chosen {
                tray::DICTATE => {
                    if recording {
                        app.restore_target();
                        app.stop();
                    } else {
                        let target = HWND(LAST_TARGET.load(Ordering::Acquire) as *mut _);
                        let _ = SetForegroundWindow(target);
                        app.start(target);
                    }
                }
                tray::PAUSE => {
                    let paused = !app.paused;
                    app.pause(paused);
                }
                tray::CLEANUP => {
                    app.config.cleanup = !app.config.cleanup;
                    app.save();
                }
                tray::COPY => app.copy(),
                tray::AUTOSTART => {
                    let _ = preferences::set_autostart(!preferences::autostart());
                }
                tray::EXIT => {
                    app.cancel();
                    PostQuitMessage(0);
                }
                tray::DEFAULT_MIC => {
                    app.cancel();
                    app.config.microphone = None;
                    app.save();
                }
                index
                    if index >= tray::MIC_FIRST
                        && index < tray::MIC_FIRST + devices.len() as u32 =>
                {
                    app.cancel();
                    app.config.microphone =
                        Some(devices[(index - tray::MIC_FIRST) as usize].clone());
                    app.save();
                }
                _ => {}
            }
        }
        return LRESULT(0);
    }
    if [
        WM_START,
        WM_RELEASE,
        WM_CANCEL,
        WM_TIMER,
        WM_CLOSE,
        WM_ENDSESSION,
    ]
    .contains(&msg)
    {
        if let Ok(mut app) = state.try_borrow_mut() {
            match msg {
                WM_START => app.start(GetForegroundWindow()),
                WM_RELEASE => app.stop(),
                WM_CANCEL => app.cancel(),
                WM_TIMER => app.tick(MENU_OPEN.load(Ordering::Acquire)),
                WM_CLOSE => {
                    app.cancel();
                    PostQuitMessage(0);
                }
                WM_ENDSESSION if wp.0 != 0 => {
                    app.cancel();
                    PostQuitMessage(0);
                }
                _ => {}
            }
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
pub fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let mutex = CreateMutexW(None, false, w!("Local\\DictadoLite.Native.v1"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if let Ok(hwnd) = FindWindowExW(None, None, w!("DictadoLiteController"), None) {
                let message = if args.iter().any(|arg| arg == "--cancel") {
                    WM_CANCEL
                } else if args.iter().any(|arg| arg == "--exit") {
                    WM_CLOSE
                } else if args.iter().any(|arg| arg == "--menu") {
                    WM_MENU
                } else {
                    0
                };
                if message != 0 {
                    let _ = PostMessageW(Some(hwnd), message, WPARAM(0), LPARAM(0));
                }
            }
            let _ = CloseHandle(mutex);
            return Ok(());
        }
        let option = |name: &str| {
            args.windows(2)
                .find(|pair| pair[0] == name)
                .map(|pair| PathBuf::from(&pair[1]))
        };
        let model = option("--model").unwrap_or(
            std::env::current_exe()?
                .parent()
                .ok_or("Executable directory unavailable")?
                .join("models/parakeet-tdt-0.6b-v3-Q8_0.gguf"),
        );
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        TASKBAR_CREATED.store(
            RegisterWindowMessageW(w!("TaskbarCreated")),
            Ordering::Relaxed,
        );
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance,
            lpszClassName: w!("DictadoLiteController"),
            ..Default::default()
        });
        let target = GetForegroundWindow();
        LAST_TARGET.store(target.0 as isize, Ordering::Release);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("DictadoLiteController"),
            w!("Dictado Lite Controller"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )?;
        let config = Config::load()?;
        let stop_after = args
            .iter()
            .find_map(|arg| arg.strip_prefix("--capture-seconds="))
            .map(str::parse::<u64>)
            .transpose()?
            .map(|value| Duration::from_secs(value.min(60)));
        let app = Box::new(RefCell::new(Application::new(
            hwnd,
            config,
            model,
            option("--metrics-file"),
            stop_after,
        )?));
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&*app) as *const RefCell<Application> as isize,
        );
        let hook = hotkey::Hook::install(hwnd)?;
        let focus_hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(focus),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        if let Some(wav) = option("--dictate-wav") {
            app.borrow_mut().wav(wav, target);
        } else if stop_after.is_some() {
            app.borrow_mut().start(target);
        }
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result <= 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        drop(hook);
        let _ = UnhookWinEvent(focus_hook);
        app.borrow_mut().cancel();
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        drop(app);
        let _ = DestroyWindow(hwnd);
        let _ = CloseHandle(mutex);
    }
    Ok(())
}
