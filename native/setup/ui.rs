//! Small native installer; copy/hash work lives off the UI thread.
use super::{install, payload::Result};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::*, HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::EnableWindow,
            WindowsAndMessaging::*,
        },
    },
};
enum Event {
    Progress(u64, u64),
    Finished(std::result::Result<PathBuf, String>),
}
struct Dialog {
    test: bool,
    running: Cell<bool>,
    done: Cell<bool>,
    result: RefCell<Option<std::result::Result<PathBuf, String>>>,
    receiver: RefCell<Option<mpsc::Receiver<Event>>>,
    cancel: Arc<AtomicBool>,
    status: Cell<HWND>,
    progress: Cell<HWND>,
    button: Cell<HWND>,
    font: Cell<HFONT>,
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog;
    if !ptr.is_null() {
        let d = &*ptr;
        match msg {
            WM_CLOSE => {
                if d.running.get() {
                    d.cancel.store(true, Ordering::Relaxed);
                } else {
                    d.done.set(true);
                }
                return LRESULT(0);
            }
            WM_COMMAND => {
                match wp.0 & 0xffff {
                    1 if !d.running.replace(true) => {
                        let (tx, rx) = mpsc::channel();
                        *d.receiver.borrow_mut() = Some(rx);
                        let cancel = d.cancel.clone();
                        let test = d.test;
                        let _ = EnableWindow(d.button.get(), false);
                        let _ = SetWindowTextW(
                            d.status.get(),
                            w!("Copiando y verificando los archivos…"),
                        );
                        std::thread::spawn(move || {
                            let outcome = install::install(test, |done, total| {
                                let _ = tx.send(Event::Progress(done, total));
                                !cancel.load(Ordering::Relaxed)
                            })
                            .map_err(|e| e.to_string());
                            let _ = tx.send(Event::Finished(outcome));
                        });
                        SetTimer(Some(hwnd), 1, 100, None);
                    }
                    2 => {
                        if d.running.get() {
                            d.cancel.store(true, Ordering::Relaxed);
                        } else {
                            d.done.set(true);
                        }
                    }
                    _ => {}
                }
                return LRESULT(0);
            }
            WM_TIMER => {
                if let Some(rx) = d.receiver.borrow().as_ref() {
                    while let Ok(event) = rx.try_recv() {
                        match event {
                            Event::Progress(done, total) => {
                                SendMessageW(
                                    d.progress.get(),
                                    PBM_SETPOS,
                                    Some(WPARAM((done * 100 / total.max(1)) as usize)),
                                    None,
                                );
                            }
                            Event::Finished(result) => {
                                *d.result.borrow_mut() = Some(result);
                                d.done.set(true);
                                let _ = KillTimer(Some(hwnd), 1);
                            }
                        }
                    }
                }
                return LRESULT(0);
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
pub fn run(test: bool) -> Result<Option<PathBuf>> {
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        })
        .ok()?;
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("DictadoSetup"),
            lpfnWndProc: Some(procedure),
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            w!("DictadoSetup"),
            w!("Instalar Dictado Lite"),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            510,
            300,
            None,
            None,
            Some(instance),
            None,
        )?;
        let d = Box::new(Dialog {
            test,
            running: Cell::new(false),
            done: Cell::new(false),
            result: RefCell::new(None),
            receiver: RefCell::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
            status: Cell::new(HWND::default()),
            progress: Cell::new(HWND::default()),
            button: Cell::new(HWND::default()),
            font: Cell::new(HFONT::default()),
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*d) as *const Dialog as isize);
        let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
        let mut font = LOGFONTW {
            lfHeight: -(14.0 * scale).round() as i32,
            lfQuality: CLEARTYPE_QUALITY,
            ..Default::default()
        };
        let name: Vec<_> = "Segoe UI".encode_utf16().chain(Some(0)).collect();
        font.lfFaceName[..name.len()].copy_from_slice(&name);
        let font = CreateFontIndirectW(&font);
        d.font.set(font);
        let child = |class: PCWSTR,
                     text: PCWSTR,
                     style: WINDOW_STYLE,
                     x: i32,
                     y: i32,
                     width: i32,
                     height: i32,
                     id: usize|
         -> windows::core::Result<HWND> {
            let c = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                text,
                WS_CHILD | WS_VISIBLE | style,
                (x as f32 * scale) as i32,
                (y as f32 * scale) as i32,
                (width as f32 * scale) as i32,
                (height as f32 * scale) as i32,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(instance),
                None,
            )?;
            SendMessageW(
                c,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
            Ok(c)
        };
        child(
            w!("STATIC"),
            w!("Dictado local, ligero y sin conexión."),
            WINDOW_STYLE(0),
            24,
            22,
            440,
            24,
            10,
        )?;
        child(w!("STATIC"),w!("Se instalará para tu usuario con Parakeet Q8_0 incluido.\nNecesita unos 800 MB libres. No requiere administrador.\n\nAplicación: MIT · Modelo NVIDIA / handy-computer: CC BY 4.0.\nLos avisos completos se incluyen en la carpeta de instalación."),WINDOW_STYLE(0),24,55,440,110,11)?;
        d.status.set(child(
            w!("STATIC"),
            w!("Todo preparado para instalar."),
            WINDOW_STYLE(0),
            24,
            166,
            440,
            20,
            12,
        )?);
        d.progress.set(child(
            PROGRESS_CLASSW,
            w!(""),
            WINDOW_STYLE(0),
            24,
            193,
            440,
            10,
            13,
        )?);
        d.button.set(child(
            w!("BUTTON"),
            w!("Instalar"),
            WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            270,
            220,
            92,
            30,
            1,
        )?);
        child(
            w!("BUTTON"),
            w!("Cancelar"),
            WS_TABSTOP,
            372,
            220,
            92,
            30,
            2,
        )?;
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            (510.0 * scale) as i32,
            (300.0 * scale) as i32,
            SWP_NOMOVE | SWP_NOZORDER | SWP_SHOWWINDOW,
        );
        let _ = SetForegroundWindow(hwnd);
        let mut msg = MSG::default();
        while !d.done.get() {
            if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
                break;
            }
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let _ = DestroyWindow(hwnd);
        let _ = DeleteObject(HGDIOBJ(d.font.get().0));
        let result = d.result.borrow_mut().take();
        match result {
            Some(Ok(root)) => Ok(Some(root)),
            Some(Err(e)) if e == "Installation cancelled" => Ok(None),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}
