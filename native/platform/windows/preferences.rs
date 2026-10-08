use super::wide;
use crate::config::Config;
use std::cell::Cell;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Controls::*;
use windows::Win32::UI::HiDpi::*;
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
    unsafe {
        let mut key = HKEY::default();
        RegOpenKeyExW(HKEY_CURRENT_USER, RUN, None, KEY_SET_VALUE, &mut key)
            .ok()
            .map_err(|e| e.to_string())?;
        let result = if enabled {
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            let command = wide(&format!("\"{}\"", executable.display()));
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
                    let value =
                        SendMessageW(dialog.input.get(), HKM_GETHOTKEY, None, None).0 as u16;
                    let key = value & 0xFF;
                    let mods = (value >> 8) as u8;
                    if (mods & 6 != 0 || (0x70..=0x87).contains(&key)) && key != 0 && key != 0x1B {
                        dialog.result.set(Some((key, mods & 7)));
                        dialog.done.set(true);
                    } else {
                        MessageBoxW(
                            Some(hwnd),
                            w!("Elige una tecla F o una combinación con Ctrl o Alt."),
                            w!("Atajo"),
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
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH((6_usize) as *mut _),
            ..Default::default()
        });
        let hwnd = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            w!("DictadoShortcut"),
            w!("Cambiar atajo"),
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
            w!("Mantén el atajo para hablar. Escape cancela."),
            WINDOW_STYLE(0),
            16,
            15,
            295,
            24,
            10,
        );
        let input = child(
            w!("msctls_hotkey32"),
            w!(""),
            WS_TABSTOP,
            16,
            45,
            280,
            28,
            11,
        )
        .map_err(|e| e.to_string())?;
        dialog.input.set(input);
        SendMessageW(
            input,
            HKM_SETHOTKEY,
            Some(WPARAM(
                config.key as usize | ((config.modifiers as usize) << 8),
            )),
            None,
        );
        let _ = child(w!("BUTTON"), w!("Guardar"), WS_TABSTOP, 116, 90, 85, 28, 1);
        let _ = child(w!("BUTTON"), w!("Cancelar"), WS_TABSTOP, 211, 90, 85, 28, 2);
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
