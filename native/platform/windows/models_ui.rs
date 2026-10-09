//! Native model picker and first-run download; network work never blocks the UI.
use super::{pw, wide};
use crate::{config::Config, locale::text, models};
use std::{
    cell::{Cell, RefCell},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};
use windows::{
    core::w,
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
    Finished(Result<PathBuf, String>),
}
struct Dialog {
    done: Cell<bool>,
    running: Cell<bool>,
    closing: Cell<bool>,
    selected: Cell<usize>,
    visible: RefCell<Vec<usize>>,
    all: Cell<bool>,
    result: RefCell<Option<(String, PathBuf)>>,
    receiver: RefCell<Option<mpsc::Receiver<Event>>>,
    cancel: Arc<AtomicBool>,
    list: HWND,
    details: HWND,
    status: HWND,
    progress: HWND,
    button: HWND,
}
unsafe fn details(d: &Dialog) {
    let model = &models::catalog()[d.selected.get()];
    let info = format!(
        "{}\r\n{} MiB\r\n{}: {}\r\n{}",
        text(
            "Reconocimiento de voz en tu equipo.",
            "Speech recognition on your computer."
        ),
        model.weights().size_bytes / 1_048_576,
        text("Idiomas", "Languages"),
        model.languages.join(", "),
        text(
            "Modelo local. Solo la descarga necesita Internet.",
            "Local model. Only the download needs Internet."
        )
    );
    let _ = SetWindowTextW(d.details, pw(&wide(&info)));
}
unsafe fn fill_list(d: &Dialog) {
    SendMessageW(d.list, CB_RESETCONTENT, None, None);
    let visible = d.visible.borrow();
    for index in visible.iter() {
        let model = &models::catalog()[*index];
        let label = format!(
            "{} · {} MiB{}",
            model.name,
            model.weights().size_bytes / 1_048_576,
            if *index == models::recommendations()[0] {
                text(" · Recomendado", " · Recommended")
            } else {
                ""
            }
        );
        SendMessageW(
            d.list,
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
    SendMessageW(d.list, CB_SETCURSEL, Some(WPARAM(row)), None);
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_CTLCOLORSTATIC {
        return super::dialog::background(wp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog;
    if !pointer.is_null() {
        let d = &*pointer;
        match msg {
            WM_CLOSE => {
                if d.running.get() {
                    d.closing.set(true);
                    d.cancel.store(true, Ordering::Relaxed);
                } else {
                    d.done.set(true);
                }
                return LRESULT(0);
            }
            WM_COMMAND => {
                match wp.0 & 0xffff {
                    10 if wp.0 >> 16 == CBN_SELCHANGE as usize => {
                        let selected = SendMessageW(d.list, CB_GETCURSEL, None, None).0;
                        if selected >= 0 && (selected as usize) < d.visible.borrow().len() {
                            d.selected.set(d.visible.borrow()[selected as usize]);
                            details(d);
                        }
                    }
                    1 if !d.running.replace(true) => {
                        d.cancel.store(false, Ordering::Relaxed);
                        let (tx, rx) = mpsc::channel();
                        *d.receiver.borrow_mut() = Some(rx);
                        let model = models::catalog()[d.selected.get()].clone();
                        let cancel = d.cancel.clone();
                        let _ = EnableWindow(d.list, false);
                        let _ = EnableWindow(d.button, false);
                        let _ = SetWindowTextW(
                            d.status,
                            pw(&wide(text(
                                "Conectando y verificando el modelo…",
                                "Connecting and verifying the model…",
                            ))),
                        );
                        std::thread::spawn(move || {
                            let mut last = std::time::Instant::now();
                            let result = models::download(&model, |done, total| {
                                // Bound UI events, even on a fast connection.
                                if last.elapsed().as_millis() >= 100 || done == 0 || done == total {
                                    let _ = tx.send(Event::Progress(done, total));
                                    last = std::time::Instant::now();
                                }
                                !cancel.load(Ordering::Relaxed)
                            })
                            .map_err(|e| e.to_string());
                            let _ = tx.send(Event::Finished(result));
                        });
                        SetTimer(Some(hwnd), 1, 100, None);
                    }
                    2 => {
                        if d.running.get() {
                            d.closing.set(true);
                            d.cancel.store(true, Ordering::Relaxed);
                        } else {
                            d.done.set(true);
                        }
                    }
                    3 => {
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
                    4 if !d.running.get() => {
                        let all = !d.all.get();
                        d.all.set(all);
                        *d.visible.borrow_mut() = if all {
                            (0..models::catalog().len()).collect()
                        } else {
                            models::recommendations()
                        };
                        fill_list(d);
                        details(d);
                        if let Ok(button) = GetDlgItem(Some(hwnd), 4) {
                            let _ = SetWindowTextW(
                                button,
                                pw(&wide(if all {
                                    text("Ver recomendados", "Show recommended")
                                } else {
                                    text("Ver todos los modelos", "Show all models")
                                })),
                            );
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
                                    d.progress,
                                    PBM_SETPOS,
                                    Some(WPARAM((done * 100 / total.max(1)) as usize)),
                                    None,
                                );
                                let status =
                                    format!("{} / {} MiB", done / 1_048_576, total / 1_048_576);
                                let _ = SetWindowTextW(d.status, pw(&wide(&status)));
                            }
                            Event::Finished(result) => {
                                d.running.set(false);
                                let _ = KillTimer(Some(hwnd), 1);
                                if d.closing.get() {
                                    d.done.set(true);
                                } else {
                                    match result {
                                        Ok(path) => {
                                            *d.result.borrow_mut() = Some((
                                                models::catalog()[d.selected.get()].key(),
                                                path,
                                            ));
                                            d.done.set(true);
                                        }
                                        Err(error) => {
                                            let status = format!("{}\r\n{error}", text("No se pudo descargar. Revisa Internet y reintenta.", "Download failed. Check your connection and try again."));
                                            let _ = SetWindowTextW(d.status, pw(&wide(&status)));
                                            let _ = EnableWindow(d.list, true);
                                            let _ = EnableWindow(d.button, true);
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
pub fn run(
    config: &Config,
    choose: bool,
) -> Result<Option<(String, PathBuf)>, Box<dyn std::error::Error>> {
    unsafe {
        InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        })
        .ok()?;
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("DictadoModels"),
            lpfnWndProc: Some(procedure),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            w!("DictadoModels"),
            pw(&wide(text(
                "Dictado Lite · Modelos",
                "Dictado Lite · Models",
            ))),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            420,
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
        let child =
            |class, value: &str, style, x, y, width, height, id| -> windows::core::Result<HWND> {
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
        child(
            w!("STATIC"),
            text(
                "Elige tu modelo de reconocimiento",
                "Choose your recognition model",
            ),
            WINDOW_STYLE(0),
            24,
            20,
            560,
            26,
            11,
        )?;
        let list = child(
            w!("COMBOBOX"),
            "",
            WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32),
            24,
            56,
            560,
            280,
            10,
        )?;
        for model in models::catalog() {
            let label = format!(
                "{} · {} MiB{}",
                model.name,
                model.weights().size_bytes / 1_048_576,
                if model.key() == models::recommended().key() {
                    text(" · Recomendado", " · Recommended")
                } else {
                    ""
                }
            );
            SendMessageW(
                list,
                CB_ADDSTRING,
                None,
                Some(LPARAM(wide(&label).as_ptr() as isize)),
            );
        }
        let selected = models::catalog()
            .iter()
            .position(|m| m.key() == models::selected(&config.model_id).key())
            .unwrap_or(0);
        SendMessageW(list, CB_SETCURSEL, Some(WPARAM(selected)), None);
        let d = Box::new(Dialog {
            done: Cell::new(false),
            running: Cell::new(false),
            closing: Cell::new(false),
            selected: Cell::new(selected),
            visible: RefCell::new(models::recommendations()),
            all: Cell::new(false),
            result: RefCell::new(None),
            receiver: RefCell::new(None),
            cancel: Arc::new(AtomicBool::new(false)),
            list,
            details: child(
                w!("EDIT"),
                "",
                WINDOW_STYLE(ES_MULTILINE as u32 | ES_READONLY as u32) | WS_VSCROLL,
                24,
                98,
                560,
                82,
                12,
            )?,
            status: child(
                w!("STATIC"),
                text(
                    "Descarga una vez y dicta sin conexión.",
                    "Download once and dictate offline.",
                ),
                WINDOW_STYLE(0),
                24,
                224,
                560,
                52,
                13,
            )?,
            progress: child(PROGRESS_CLASSW, "", WINDOW_STYLE(0), 24, 286, 560, 12, 14)?,
            button: child(
                w!("BUTTON"),
                text("Descargar y usar", "Download and use"),
                WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as u32),
                280,
                320,
                180,
                32,
                1,
            )?,
        });
        child(
            w!("BUTTON"),
            text("Cancelar", "Cancel"),
            WS_TABSTOP,
            472,
            320,
            112,
            32,
            2,
        )?;
        child(
            w!("BUTTON"),
            text("Detalles del modelo", "Model details"),
            WS_TABSTOP,
            24,
            320,
            220,
            32,
            3,
        )?;
        child(
            w!("BUTTON"),
            text("Ver todos los modelos", "Show all models"),
            WS_TABSTOP,
            340,
            188,
            244,
            28,
            4,
        )?;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*d) as *const Dialog as isize);
        fill_list(&d);
        details(&d);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            (620.0 * scale) as i32,
            (420.0 * scale) as i32,
            SWP_NOMOVE | SWP_NOZORDER | SWP_SHOWWINDOW,
        );
        super::dialog::center(hwnd);
        let _ = SetForegroundWindow(hwnd);
        if !choose {
            let _ = PostMessageW(Some(hwnd), WM_COMMAND, WPARAM(1), LPARAM(0));
        }
        let mut msg = MSG::default();
        while !d.done.get() {
            let result = GetMessageW(&mut msg, None, 0, 0).0;
            if result <= 0 {
                if result == 0 {
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
        let _ = DeleteObject(HGDIOBJ(font.0));
        let result = d.result.borrow_mut().take();
        Ok(result)
    }
}
