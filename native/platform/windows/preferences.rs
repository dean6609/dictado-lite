use super::wide;
use crate::{
    config::{valid_shortcut, Config},
    locale::{shortcut as label, text},
};
use std::cell::Cell;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::*;

const RUN: windows::core::PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
pub fn autostart() -> bool {
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN,
            w!("DictadoLite"),
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        ) == ERROR_SUCCESS
    }
}
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let command = if enabled {
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        Some(wide(&format!("\"{}\"", executable.display())))
    } else {
        None
    };
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            RUN,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()
        .map_err(|e| e.to_string())?;
        let result = if let Some(command) = command {
            let bytes =
                std::slice::from_raw_parts(command.as_ptr().cast::<u8>(), command.len() * 2);
            RegSetValueExW(key, w!("DictadoLite"), None, REG_SZ, Some(bytes)).ok()
        } else {
            let result = RegDeleteValueW(key, w!("DictadoLite"));
            if result == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                result.ok()
            }
        };
        let _ = RegCloseKey(key);
        result.map_err(|e| e.to_string())
    }
}
struct Dialog {
    done: Cell<bool>,
    result: Cell<Option<(u16, u8)>>,
    input: Cell<HWND>,
    font: Cell<HFONT>,
    pending: Cell<(u16, u8)>,
}
unsafe extern "system" fn capture_key(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    let dialog = &*(data as *const Dialog);
    if msg == WM_GETDLGCODE {
        return LRESULT((DLGC_WANTALLKEYS | DLGC_WANTCHARS) as isize);
    }
    if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
        let key = wp.0 as u16;
        if key == VK_TAB.0
            && GetKeyState(VK_CONTROL.0 as i32) >= 0
            && GetKeyState(VK_MENU.0 as i32) >= 0
        {
            let _ = SetFocus(GetDlgItem(GetParent(hwnd).ok(), 1).ok());
        } else if key == VK_ESCAPE.0 {
            dialog.done.set(true);
        } else {
            let mods = [(1, VK_SHIFT), (2, VK_CONTROL), (4, VK_MENU)]
                .into_iter()
                .fold(0, |mods, (bit, key)| {
                    mods | if GetKeyState(key.0 as i32) < 0 {
                        bit
                    } else {
                        0
                    }
                });
            if valid_shortcut(key, mods) {
                dialog.pending.set((key, mods));
                let value = wide(&label(key, mods));
                let _ = SetWindowTextW(hwnd, windows::core::PCWSTR(value.as_ptr()));
            }
        }
        return LRESULT(0);
    }
    if matches!(msg, WM_CHAR | WM_SYSCHAR | WM_KEYUP | WM_SYSKEYUP) {
        return LRESULT(0);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}
unsafe fn layout(hwnd: HWND, dialog: &Dialog, dpi: u32) {
    let scale = dpi as f32 / 96.0;
    let mut rect = RECT {
        right: (320.0 * scale).round() as i32,
        bottom: (136.0 * scale).round() as i32,
        ..Default::default()
    };
    let _ = AdjustWindowRectExForDpi(
        &mut rect,
        WS_CAPTION | WS_SYSMENU,
        false,
        WS_EX_DLGMODALFRAME,
        dpi,
    );
    let _ = SetWindowPos(
        hwnd,
        None,
        0,
        0,
        rect.right - rect.left,
        rect.bottom - rect.top,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
    );
    let mut font = LOGFONTW {
        lfHeight: -(12.0 * scale).round() as i32,
        lfQuality: CLEARTYPE_QUALITY,
        ..Default::default()
    };
    let face = wide("Segoe UI");
    font.lfFaceName[..face.len()].copy_from_slice(&face);
    let font = CreateFontIndirectW(&font);
    for (id, x, y, width, height) in [
        (10, 16, 15, 288, 24),
        (11, 16, 45, 288, 28),
        (1, 116, 90, 85, 28),
        (2, 211, 90, 85, 28),
    ] {
        if let Ok(control) = GetDlgItem(Some(hwnd), id) {
            let _ = SetWindowPos(
                control,
                None,
                (x as f32 * scale).round() as i32,
                (y as f32 * scale).round() as i32,
                (width as f32 * scale).round() as i32,
                (height as f32 * scale).round() as i32,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            SendMessageW(
                control,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }
    let old = dialog.font.replace(font);
    if !old.0.is_null() {
        let _ = DeleteObject(HGDIOBJ(old.0));
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_CTLCOLORSTATIC {
        return super::dialog::background(wp);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Dialog;
    if !pointer.is_null() {
        let dialog = &*pointer;
        if msg == WM_DPICHANGED {
            let rect = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            layout(hwnd, dialog, (wp.0 & 0xFFFF) as u32);
            return LRESULT(0);
        }
        if msg == WM_CLOSE {
            dialog.done.set(true);
            return LRESULT(0);
        }
        if msg == WM_COMMAND {
            match wp.0 & 0xFFFF {
                1 => {
                    let (key, mods) = dialog.pending.get();
                    if valid_shortcut(key, mods) {
                        dialog.result.set(Some((key, mods & 7)));
                        dialog.done.set(true);
                    } else {
                        MessageBoxW(
                            Some(hwnd),
                            windows::core::PCWSTR(
                                wide(text(
                                    "Elige una tecla F o una combinación con Ctrl o Alt.",
                                    "Choose an F key or a combination with Ctrl or Alt.",
                                ))
                                .as_ptr(),
                            ),
                            windows::core::PCWSTR(wide(text("Atajo", "Shortcut")).as_ptr()),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
                    return LRESULT(0);
                }
                2 => {
                    dialog.done.set(true);
                    return LRESULT(0);
                }
                _ => {}
            }
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
pub fn shortcut(config: &Config) -> Result<Option<(u16, u8)>, String> {
    unsafe {
        let _ = InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_HOTKEY_CLASS,
        });
        let instance = HINSTANCE(GetModuleHandleW(None).map_err(|e| e.to_string())?.0);
        RegisterClassW(&WNDCLASSW {
            hInstance: instance,
            lpszClassName: w!("DictadoShortcut"),
            lpfnWndProc: Some(procedure),
            hbrBackground: GetSysColorBrush(COLOR_BTNFACE),
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            w!("DictadoShortcut"),
            windows::core::PCWSTR(wide(text("Cambiar atajo", "Change shortcut")).as_ptr()),
            WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            330,
            180,
            None,
            None,
            Some(instance),
            None,
        )
        .map_err(|e| e.to_string())?;
        let dialog = Box::new(Dialog {
            done: Cell::new(false),
            result: Cell::new(None),
            input: Cell::new(HWND::default()),
            font: Cell::new(HFONT::default()),
            pending: Cell::new((config.key, config.modifiers)),
        });
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*dialog) as *const Dialog as isize);
        let child = |class, title, style, x, y, width, height, id| {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class,
                title,
                WS_CHILD | WS_VISIBLE | style,
                x,
                y,
                width,
                height,
                Some(hwnd),
                Some(HMENU(id as *mut _)),
                Some(instance),
                None,
            )
        };
        let _ = child(
            w!("STATIC"),
            windows::core::PCWSTR(
                wide(text(
                    "Pulsa el atajo. Tab pasa a Guardar.",
                    "Press a shortcut. Tab moves to Save.",
                ))
                .as_ptr(),
            ),
            WINDOW_STYLE(0),
            16,
            15,
            295,
            24,
            10,
        );
        let input = child(
            w!("EDIT"),
            windows::core::PCWSTR(wide(&label(config.key, config.modifiers)).as_ptr()),
            WS_TABSTOP | WS_BORDER | WINDOW_STYLE(ES_READONLY as u32),
            16,
            45,
            280,
            28,
            11,
        )
        .map_err(|e| e.to_string())?;
        dialog.input.set(input);
        SetWindowSubclass(
            input,
            Some(capture_key),
            1,
            (&*dialog) as *const Dialog as usize,
        )
        .ok()
        .map_err(|e| e.to_string())?;
        let _ = child(
            w!("BUTTON"),
            windows::core::PCWSTR(wide(text("Guardar", "Save")).as_ptr()),
            WS_TABSTOP,
            116,
            90,
            85,
            28,
            1,
        );
        let _ = child(
            w!("BUTTON"),
            windows::core::PCWSTR(wide(text("Cancelar", "Cancel")).as_ptr()),
            WS_TABSTOP,
            211,
            90,
            85,
            28,
            2,
        );
        layout(hwnd, &dialog, GetDpiForWindow(hwnd).max(96));
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW,
        )
        .map_err(|e| e.to_string())?;
        super::diagnostics::window("shortcut.shown", hwnd);
        super::dialog::center(hwnd);
        let _ = SetForegroundWindow(hwnd);
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(input));
        let mut message = MSG::default();
        while !dialog.done.get() {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result <= 0 {
                if result == 0 {
                    PostQuitMessage(0);
                }
                break;
            }
            if !IsDialogMessageW(hwnd, &message).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        let _ = DestroyWindow(hwnd);
        let _ = DeleteObject(HGDIOBJ(dialog.font.get().0));
        Ok(dialog.result.get())
    }
}
