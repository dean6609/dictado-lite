//! One native setup window: choice, models, installation/download, completion.
use super::{install, payload::Result};
use dictado_lite::{
    config::Config,
    locale::text,
    models,
    platform::windows::{dialog, pw, wide},
};
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

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Choice,
    Models,
    Working,
    Finished,
}
enum Event {
    Progress(bool, u64, u64),
    Finished(std::result::Result<PathBuf, String>),
}
struct Dialog {
    test: bool,
    choose: Cell<bool>,
    page: Cell<Page>,
    selected: Cell<usize>,
    all: Cell<bool>,
    visible: RefCell<Vec<usize>>,
    done: Cell<bool>,
    closing: Cell<bool>,
    launch: Cell<bool>,
    result: RefCell<Option<PathBuf>>,
    receiver: RefCell<Option<mpsc::Receiver<Event>>>,
    cancel: Arc<AtomicBool>,
    font: HFONT,
}
unsafe fn set(hwnd: HWND, id: i32, value: &str) {
    if let Ok(control) = GetDlgItem(Some(hwnd), id) {
        let _ = SetWindowTextW(control, pw(&wide(value)));
    }
}
unsafe fn details(hwnd: HWND, d: &Dialog) {
    let m = &models::catalog()[d.selected.get()];
    set(
        hwnd,
        32,
        &format!(
            "{}\r\n{} MiB\r\n{}: {}",
            m.name,
            m.weights().size_bytes / 1_048_576,
            text("Idiomas", "Languages"),
            m.languages.join(", ")
        ),
    );
}
unsafe fn fill(hwnd: HWND, d: &Dialog) {
    let Ok(list) = GetDlgItem(Some(hwnd), 31) else {
        return;
    };
    SendMessageW(list, CB_RESETCONTENT, None, None);
    let visible = d.visible.borrow();
    for index in visible.iter() {
        let model = &models::catalog()[*index];
        let label = format!(
            "{} · {} MiB",
            model.name,
            model.weights().size_bytes / 1_048_576
        );
        SendMessageW(
            list,
            CB_ADDSTRING,
            None,
            Some(LPARAM(wide(&label).as_ptr() as isize)),
        );
    }
    let row = visible
        .iter()
        .position(|i| *i == d.selected.get())
        .unwrap_or(0);
    d.selected.set(visible[row]);
    SendMessageW(list, CB_SETCURSEL, Some(WPARAM(row)), None);
    details(hwnd, d);
}
unsafe fn page(hwnd: HWND, d: &Dialog) {
    let value = d.page.get();
    for (id, visible) in [
        (11, value == Page::Choice),
        (20, value == Page::Choice),
        (21, value == Page::Choice),
        (30, value == Page::Models),
        (31, value == Page::Models),
        (32, value == Page::Models),
        (33, value == Page::Models),
        (34, value == Page::Models),
        (13, value == Page::Working),
    ] {
        if let Ok(control) = GetDlgItem(Some(hwnd), id) {
            let _ = ShowWindow(control, if visible { SW_SHOW } else { SW_HIDE });
        }
    }
    set(
        hwnd,
        10,
        match value {
            Page::Choice => text("Instala Dictado Lite", "Install Dictado Lite"),
            Page::Models => text(
                "Elige un modelo para descargar",
                "Choose a model to download",
            ),
            Page::Working => text("Preparando tu aplicación", "Preparing your application"),
            Page::Finished => text("Todo listo para dictar", "Ready to dictate"),
        },
    );
    set(
        hwnd,
        1,
        match value {
            Page::Choice => text("Continuar", "Continue"),
            Page::Models => text("Instalar", "Install"),
            Page::Working => text("Instalando…", "Installing…"),
            Page::Finished => text("Abrir Dictado Lite", "Open Dictado Lite"),
        },
    );
    set(
        hwnd,
        2,
        if value == Page::Finished {
            text("Cerrar", "Close")
        } else {
            text("Cancelar", "Cancel")
        },
    );
    if let Ok(control) = GetDlgItem(Some(hwnd), 1) {
        let _ = EnableWindow(control, value != Page::Working);
    }
    let _ = InvalidateRect(Some(hwnd), None, true);
}
unsafe fn start(hwnd: HWND, d: &Dialog) {
    d.page.set(Page::Working);
    page(hwnd, d);
    d.cancel.store(false, Ordering::Relaxed);
    set(
        hwnd,
        12,
        text("Preparando el modelo…", "Preparing the model…"),
    );
    let (tx, rx) = mpsc::channel();
    *d.receiver.borrow_mut() = Some(rx);
    let cancel = d.cancel.clone();
    let test = d.test;
    let model = models::catalog()[d.selected.get()].clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<PathBuf> {
            if !test {
                let mut last = std::time::Instant::now();
                models::download(&model, |done, total| {
                    if last.elapsed().as_millis() >= 100 || done == 0 || done == total {
                        let _ = tx.send(Event::Progress(true, done, total));
                        last = std::time::Instant::now();
                    }
                    !cancel.load(Ordering::Relaxed)
                })?;
            }
            let root = install::install(test, |done, total| {
                let _ = tx.send(Event::Progress(false, done, total));
                !cancel.load(Ordering::Relaxed)
            })?;
            if !test {
                let mut config = Config::load()?;
                config.model_id = model.key();
                config.choose_model = false;
                config.save()?;
            }
            Ok(root)
        })()
        .map_err(|e| e.to_string());
        let _ = tx.send(Event::Finished(result));
    });
    SetTimer(Some(hwnd), 1, 100, None);
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_CTLCOLORSTATIC {
        return dialog::background(wp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog;
    if !pointer.is_null() {
        let d = &*pointer;
        match msg {
            WM_CLOSE => {
                if d.page.get() == Page::Working {
                    d.closing.set(true);
                    d.cancel.store(true, Ordering::Relaxed);
                } else {
                    d.done.set(true);
                }
                return LRESULT(0);
            }
            WM_COMMAND => {
                match wp.0 & 0xffff {
                    1 => match d.page.get() {
                        Page::Choice if d.choose.get() => {
                            d.page.set(Page::Models);
                            fill(hwnd, d);
                            page(hwnd, d);
                            set(hwnd,12,text("El modelo se descarga una vez. Después podrás dictar sin conexión.","Download the model once. Then you can dictate offline."));
                        }
                        Page::Choice | Page::Models => start(hwnd, d),
                        Page::Finished => {
                            d.launch.set(true);
                            d.done.set(true);
                        }
                        Page::Working => {}
                    },
                    2 => {
                        if d.page.get() == Page::Working {
                            d.closing.set(true);
                            d.cancel.store(true, Ordering::Relaxed);
                        } else {
                            d.done.set(true);
                        }
                    }
                    20 | 21 if d.page.get() == Page::Choice => {
                        d.choose.set(wp.0 & 0xffff == 21);
                        for id in [20, 21] {
                            if let Ok(control) = GetDlgItem(Some(hwnd), id) {
                                SendMessageW(
                                    control,
                                    BM_SETCHECK,
                                    Some(WPARAM(if id == (wp.0 & 0xffff) as i32 { 1 } else { 0 })),
                                    None,
                                );
                            }
                        }
                    }
                    31 if wp.0 >> 16 == CBN_SELCHANGE as usize => {
                        if let Ok(list) = GetDlgItem(Some(hwnd), 31) {
                            let row = SendMessageW(list, CB_GETCURSEL, None, None).0;
                            if row >= 0 && (row as usize) < d.visible.borrow().len() {
                                d.selected.set(d.visible.borrow()[row as usize]);
                                details(hwnd, d);
                            }
                        }
                    }
                    33 => {
                        let all = !d.all.get();
                        d.all.set(all);
                        *d.visible.borrow_mut() = if all {
                            (0..models::catalog().len()).collect()
                        } else {
                            models::recommendations()
                        };
                        fill(hwnd, d);
                        set(
                            hwnd,
                            30,
                            if all {
                                text("Todos los modelos", "All models")
                            } else {
                                text("Recomendados para empezar", "Recommended to get started")
                            },
                        );
                        set(
                            hwnd,
                            33,
                            if all {
                                text("Ver recomendados", "Show recommended")
                            } else {
                                text("Ver todos los modelos", "Show all models")
                            },
                        );
                    }
                    34 => {
                        let url = wide(&format!(
                            "https://huggingface.co/{}",
                            models::catalog()[d.selected.get()].id
                        ));
                        windows::Win32::UI::Shell::ShellExecuteW(
                            Some(hwnd),
                            w!("open"),
                            pw(&url),
                            None,
                            None,
                            SW_SHOWNORMAL,
                        );
                    }
                    _ => {}
                }
                return LRESULT(0);
            }
            WM_TIMER => {
                if let Some(rx) = d.receiver.borrow().as_ref() {
                    while let Ok(event) = rx.try_recv() {
                        match event {
                            Event::Progress(download, done, total) => {
                                if let Ok(control) = GetDlgItem(Some(hwnd), 13) {
                                    SendMessageW(
                                        control,
                                        PBM_SETPOS,
                                        Some(WPARAM((done * 100 / total.max(1)) as usize)),
                                        None,
                                    );
                                }
                                set(
                                    hwnd,
                                    12,
                                    &format!(
                                        "{}\r\n{} / {} MiB",
                                        if download {
                                            text("Preparando el modelo…", "Preparing the model…")
                                        } else {
                                            text(
                                                "Instalando la aplicación…",
                                                "Installing the application…",
                                            )
                                        },
                                        done / 1_048_576,
                                        total / 1_048_576
                                    ),
                                );
                            }
                            Event::Finished(result) => {
                                let _ = KillTimer(Some(hwnd), 1);
                                if d.closing.get() {
                                    d.done.set(true);
                                } else {
                                    match result {
                                        Ok(root) => {
                                            *d.result.borrow_mut() = Some(root);
                                            d.page.set(Page::Finished);
                                            page(hwnd, d);
                                            set(hwnd,12,text("Pulsa Ctrl+Espacio para empezar a dictar.\r\nPulsa otra vez para insertar el texto. Escape cancela.","Press Ctrl+Space to start dictating.\r\nPress again to insert the text. Escape cancels."));
                                        }
                                        Err(error) => {
                                            d.page.set(if d.choose.get() {
                                                Page::Models
                                            } else {
                                                Page::Choice
                                            });
                                            page(hwnd, d);
                                            set(
                                                hwnd,
                                                12,
                                                &format!(
                                                    "{}\r\n{error}",
                                                    text(
                                                        "No se pudo completar. Puedes reintentar.",
                                                        "Could not complete. You can try again."
                                                    )
                                                ),
                                            );
                                        }
                                    }
                                }
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
pub fn run(test: bool) -> Result<Option<(PathBuf, bool)>> {
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
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            w!("DictadoSetup"),
            pw(&wide(text("Instalar Dictado Lite", "Install Dictado Lite"))),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            590,
            440,
            None,
            None,
            Some(instance),
            None,
        )?;
        let scale = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
        let mut font = LOGFONTW {
            lfHeight: -(14.0 * scale) as i32,
            ..Default::default()
        };
        let face = wide("Segoe UI");
        font.lfFaceName[..face.len()].copy_from_slice(&face);
        let font = CreateFontIndirectW(&font);
        let child = |class: PCWSTR,
                     value: &str,
                     style,
                     x,
                     y,
                     width,
                     height,
                     id|
         -> windows::core::Result<HWND> {
            let control = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                pw(&wide(value)),
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
                control,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
            Ok(control)
        };
        child(w!("STATIC"), "", WINDOW_STYLE(0), 24, 22, 530, 26, 10)?;
        child(w!("STATIC"),text("Instala la aplicación y descarga el modelo que prefieras.\r\nDespués podrás dictar sin conexión.\r\nNo necesitas una cuenta.","Install the application and download the model you prefer.\r\nThen you can dictate offline.\r\nNo account required."),WINDOW_STYLE(0),24,62,530,80,11)?;
        let recommended = child(
            w!("BUTTON"),
            text(
                "Descargar el modelo recomendado",
                "Download the recommended model",
            ),
            WS_TABSTOP | WINDOW_STYLE(BS_RADIOBUTTON as u32),
            24,
            156,
            530,
            26,
            20,
        )?;
        SendMessageW(recommended, BM_SETCHECK, Some(WPARAM(1)), None);
        child(
            w!("BUTTON"),
            text(
                "Elegir un modelo para descargar",
                "Choose a model to download",
            ),
            WS_TABSTOP | WINDOW_STYLE(BS_RADIOBUTTON as u32),
            24,
            190,
            530,
            26,
            21,
        )?;
        child(
            w!("STATIC"),
            text("Recomendados para empezar", "Recommended to get started"),
            WINDOW_STYLE(0),
            24,
            60,
            530,
            24,
            30,
        )?;
        child(
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            24,
            94,
            530,
            240,
            31,
        )?;
        child(
            w!("EDIT"),
            "",
            WINDOW_STYLE(ES_MULTILINE as u32 | ES_READONLY as u32) | WS_VSCROLL,
            24,
            140,
            530,
            76,
            32,
        )?;
        child(
            w!("BUTTON"),
            text("Ver todos los modelos", "Show all models"),
            WS_TABSTOP,
            24,
            220,
            250,
            28,
            33,
        )?;
        child(
            w!("BUTTON"),
            text("Detalles del modelo", "Model details"),
            WS_TABSTOP,
            284,
            220,
            270,
            28,
            34,
        )?;
        child(
            w!("STATIC"),
            text(
                "El instalador no incluye modelos preinstalados.",
                "The installer does not include preinstalled models.",
            ),
            WINDOW_STYLE(0),
            24,
            265,
            530,
            50,
            12,
        )?;
        child(PROGRESS_CLASSW, "", WINDOW_STYLE(0), 24, 318, 530, 12, 13)?;
        child(
            w!("BUTTON"),
            "",
            WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
            290,
            348,
            164,
            32,
            1,
        )?;
        child(
            w!("BUTTON"),
            text("Cancelar", "Cancel"),
            WS_TABSTOP,
            466,
            348,
            88,
            32,
            2,
        )?;
        let d = Box::new(Dialog {
            test,
            choose: Cell::new(false),
            page: Cell::new(Page::Choice),
            selected: Cell::new(models::recommendations()[0]),
            all: Cell::new(false),
            visible: RefCell::new(models::recommendations()),
            done: Cell::new(false),
            closing: Cell::new(false),
            launch: Cell::new(false),
            result: RefCell::new(None),
            receiver: RefCell::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
            font,
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*d) as *const Dialog as isize);
        page(hwnd, &d);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            (590.0 * scale) as i32,
            (440.0 * scale) as i32,
            SWP_NOMOVE | SWP_NOZORDER,
        );
        dialog::center(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let mut msg = MSG::default();
        while !d.done.get() {
            let status = GetMessageW(&mut msg, None, 0, 0).0;
            if status <= 0 {
                if status == 0 {
                    PostQuitMessage(msg.wParam.0 as i32);
                }
                break;
            }
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        let _ = DestroyWindow(hwnd);
        let _ = DeleteObject(HGDIOBJ(d.font.0));
        let result = d.result.borrow_mut().take();
        Ok(result.map(|root| (root, d.launch.get())))
    }
}
