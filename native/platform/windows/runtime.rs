//! Single instance, message pump and tray-command routing.
use super::{
    application::{Application, Phase},
    hotkey, models_ui, overlay, preferences, tray,
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

pub const WM_TOGGLE: u32 = WM_APP + 1;
pub const WM_CANCEL: u32 = WM_APP + 3;
pub const WM_TRAY: u32 = WM_APP + 4;
const WM_MENU: u32 = WM_APP + 5;
const WM_SHORTCUT: u32 = WM_APP + 6;
const WM_MODELS: u32 = WM_APP + 7;
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
    if msg == overlay::WM_LAYOUT {
        if let Ok(mut app) = state.try_borrow_mut() {
            if app
                .overlay
                .layout()
                .and_then(|()| app.overlay.advance([0.0; 9]).map(|_| ()))
                .is_err()
            {
                app.fail("No se pudo mostrar el indicador de actividad.");
            }
        }
        return LRESULT(0);
    }
    if msg == overlay::WM_OVERLAY {
        if wp.0 == overlay::COPY_OR_OPTIONS {
            if let Ok(mut app) = state.try_borrow_mut() {
                if app.last_text.is_some() {
                    app.copy();
                } else {
                    let _ = PostMessageW(Some(hwnd), WM_MENU, WPARAM(0), LPARAM(0));
                }
            }
        } else if let Ok(mut app) = state.try_borrow_mut() {
            match wp.0 {
                overlay::RETRY => app.retry(),
                overlay::DISMISS => app.cancel(),
                _ => {}
            }
        }
        return LRESULT(0);
    }
    if msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed) {
        if let Ok(app) = state.try_borrow() {
            app.tray.refresh();
        }
        return LRESULT(0);
    }
    if [WM_MENU, WM_SHORTCUT, WM_MODELS].contains(&msg)
        || (msg == WM_TRAY && [WM_RBUTTONUP, WM_LBUTTONUP].contains(&(lp.0 as u32)))
    {
        let view = state.try_borrow().ok().map(|app| {
            (
                app.config.clone(),
                app.phase == Phase::Listening,
                app.paused,
                app.last_text.is_some(),
                app.overlay.menu_owner().unwrap_or(hwnd),
            )
        });
        let Some((config, recording, paused, copy, owner)) = view else {
            return LRESULT(0);
        };
        super::diagnostics::event("menu.devices.begin");
        let devices = if [WM_SHORTCUT, WM_MODELS].contains(&msg) {
            Vec::new()
        } else {
            microphones().unwrap_or_default()
        };
        super::diagnostics::event("menu.devices.end");
        MENU_OPEN.store(true, Ordering::Release);
        let chosen = if msg == WM_SHORTCUT {
            tray::SHORTCUT
        } else if msg == WM_MODELS {
            tray::MODELS
        } else {
            tray::menu(
                owner,
                &config,
                recording,
                paused,
                copy,
                &devices,
                preferences::autostart(),
            )
        };
        super::diagnostics::event(&format!("menu.selected.{chosen}"));
        MENU_OPEN.store(false, Ordering::Release);
        if GetForegroundWindow() == owner {
            let target = HWND(LAST_TARGET.load(Ordering::Acquire) as *mut _);
            let _ = SetForegroundWindow(target);
        }
        if chosen == tray::MODELS {
            if let Ok(mut app) = state.try_borrow_mut() {
                app.cancel();
            }
            hotkey::pause(true);
            let result = models_ui::run(&config, true);
            if let Ok(mut app) = state.try_borrow_mut() {
                match result {
                    Ok(Some((id, path))) => {
                        app.config.model_id = id;
                        app.config.choose_model = false;
                        app.set_model(path);
                        app.save();
                    }
                    Err(error) => app.fail(&error.to_string()),
                    _ => {}
                }
                hotkey::pause(app.paused);
            }
        } else if chosen == tray::SHORTCUT {
            if let Ok(mut app) = state.try_borrow_mut() {
                app.cancel();
            }
            hotkey::pause(true);
            let shortcut = preferences::shortcut(&config);
            if let Ok(mut app) = state.try_borrow_mut() {
                match shortcut {
                    Ok(Some((key, mods))) => {
                        app.config.key = key;
                        app.config.modifiers = mods;
                        app.save();
                    }
                    Err(_) => app.fail("No se pudieron abrir los ajustes de atajo."),
                    _ => {}
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
                    if preferences::set_autostart(!preferences::autostart()).is_err() {
                        app.fail("No se pudieron guardar los ajustes de inicio con Windows.");
                    }
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
    if [WM_TOGGLE, WM_CANCEL, WM_TIMER, WM_CLOSE, WM_ENDSESSION].contains(&msg) {
        if msg == WM_CANCEL || msg == WM_CLOSE || (msg == WM_ENDSESSION && wp.0 != 0) {
            let _ = EndMenu();
        }
        if let Ok(mut app) = state.try_borrow_mut() {
            match msg {
                WM_TOGGLE => {
                    if app.phase == Phase::Listening {
                        app.stop();
                    } else {
                        app.start(GetForegroundWindow());
                    }
                }
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
        } else if msg == WM_CANCEL || msg == WM_CLOSE {
            let _ = PostMessageW(Some(hwnd), msg, wp, lp);
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
            let mut report = serde_json::json!({"instance":true,"controller":false});
            if let Ok(hwnd) = FindWindowExW(None, None, w!("DictadoLiteController"), None) {
                report["controller"] = true.into();
                let message = if args.iter().any(|arg| arg == "--cancel") {
                    WM_CANCEL
                } else if args.iter().any(|arg| arg == "--exit") {
                    WM_CLOSE
                } else if args.iter().any(|arg| arg == "--menu") {
                    WM_MENU
                } else if args.iter().any(|arg| arg == "--shortcut") {
                    WM_SHORTCUT
                } else if args.iter().any(|arg| arg == "--models") {
                    WM_MODELS
                } else {
                    0
                };
                if message != 0 {
                    // Sent controls reach a modal menu loop; menu display
                    // remains asynchronous to the requesting instance.
                    let delivered = ![WM_MENU, WM_SHORTCUT, WM_MODELS].contains(&message)
                        && SendMessageTimeoutW(
                            hwnd,
                            message,
                            WPARAM(0),
                            LPARAM(0),
                            SMTO_ABORTIFHUNG | SMTO_BLOCK,
                            1500,
                            None,
                        )
                        .0 != 0;
                    report["sent"] = delivered.into();
                    report["message"] = message.into();
                    if !delivered {
                        let _ = PostMessageW(Some(hwnd), message, WPARAM(0), LPARAM(0));
                    }
                }
            }
            if args.iter().any(|arg| arg == "--control-report") {
                println!("{report}");
            }
            let _ = CloseHandle(mutex);
            return Ok(());
        }
        if args.iter().any(|arg| {
            ["--cancel", "--exit", "--menu", "--shortcut", "--models"].contains(&arg.as_str())
        }) {
            if args.iter().any(|arg| arg == "--control-report") {
                println!("{}", serde_json::json!({"instance":false}));
            }
            let _ = CloseHandle(mutex);
            return Ok(());
        }
        let option = |name: &str| {
            args.windows(2)
                .find(|pair| pair[0] == name)
                .map(|pair| PathBuf::from(&pair[1]))
        };
        let mut config = Config::load()?;
        let model = if let Some(path) = option("--model") {
            path
        } else {
            let selected = crate::models::selected(&config.model_id);
            let cache = selected.cache_path()?;
            let bundled = std::env::current_exe()?
                .parent()
                .ok_or("Executable directory unavailable")?
                .join("models")
                .join(&selected.weights().filename);
            if selected.ready(&cache) && !config.choose_model {
                cache
            } else if selected.ready(&bundled) && !config.choose_model {
                bundled
            } else {
                let Some((id, path)) = models_ui::run(&config, config.choose_model)? else {
                    let _ = CloseHandle(mutex);
                    return Ok(());
                };
                config.model_id = id;
                config.choose_model = false;
                config.save()?;
                path
            }
        };
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
            args.iter().any(|arg| arg == "--inspect-overlay"),
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
