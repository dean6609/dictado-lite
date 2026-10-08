use super::{pw, runtime::WM_TRAY, wide};
use crate::config::Config;
use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub const DICTATE: u32 = 1;
pub const PAUSE: u32 = 2;
pub const SHORTCUT: u32 = 3;
pub const CLEANUP: u32 = 4;
pub const COPY: u32 = 5;
pub const EXIT: u32 = 6;
pub const AUTOSTART: u32 = 7;
pub const DEFAULT_MIC: u32 = 100;
pub const MIC_FIRST: u32 = 101;

pub struct Tray {
    data: NOTIFYICONDATAW,
}
impl Tray {
    pub fn create(hwnd: HWND) -> windows::core::Result<Self> {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: unsafe {
                LoadIconW(
                    Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                    w!("DICTADO_ICON"),
                )?
            },
            ..Default::default()
        };
        let tip = wide("Dictado Lite · Mantén Ctrl+Alt+Espacio para hablar");
        data.szTip[..tip.len()].copy_from_slice(&tip);
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
            return Err(windows::core::Error::from_win32());
        }
        Ok(Self { data })
    }
    pub fn refresh(&self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_ADD, &self.data);
        }
    }
    pub fn update(&mut self, config: &Config, paused: bool) {
        let key = if config.key == 0x20 {
            "Espacio".into()
        } else if (0x70..=0x87).contains(&config.key) {
            format!("F{}", config.key - 0x70 + 1)
        } else {
            char::from_u32(config.key as u32).unwrap_or('?').to_string()
        };
        let mut keys: Vec<String> = [(2, "Ctrl"), (4, "Alt"), (1, "Mayús")]
            .into_iter()
            .filter(|(bit, _)| config.modifiers & bit != 0)
            .map(|(_, name)| name.into())
            .collect();
        keys.push(key);
        let text = if paused {
            "Dictado Lite · Pausado".into()
        } else {
            format!("Dictado Lite · Mantén {} para hablar", keys.join("+"))
        };
        let mut tip: Vec<u16> = text.encode_utf16().take(126).collect();
        tip.push(0);
        self.data.szTip.fill(0);
        self.data.szTip[..tip.len()].copy_from_slice(&tip);
        let mut data = self.data;
        data.uFlags = NIF_TIP;
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }
    pub fn notify(&self, message: &str) {
        let mut data = self.data;
        data.uFlags = NIF_INFO;
        data.dwInfoFlags = NIIF_ERROR;
        let title = wide("Dictado Lite");
        data.szInfoTitle[..title.len()].copy_from_slice(&title);
        let mut text: Vec<u16> = message.encode_utf16().take(254).collect();
        text.push(0);
        data.szInfo[..text.len()].copy_from_slice(&text);
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
        }
    }
}
pub fn menu(
    hwnd: HWND,
    config: &Config,
    recording: bool,
    paused: bool,
    copy: bool,
    microphones: &[String],
    autostart: bool,
) -> u32 {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else {
            return 0;
        };
        let add = |id: u32, text: &str, flags| {
            let text = wide(text);
            let _ = AppendMenuW(menu, flags, id as usize, pw(&text));
        };
        add(
            DICTATE,
            if recording {
                "Detener dictado"
            } else {
                "Iniciar dictado"
            },
            MF_STRING
                | if paused {
                    MF_GRAYED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
        );
        add(PAUSE, if paused { "Reanudar" } else { "Pausar" }, MF_STRING);
        if let Ok(microphone) = CreatePopupMenu() {
            let _ = AppendMenuW(
                microphone,
                MF_STRING
                    | if config.microphone.is_none() {
                        MF_CHECKED
                    } else {
                        MENU_ITEM_FLAGS(0)
                    },
                DEFAULT_MIC as usize,
                w!("Predeterminado de Windows"),
            );
            for (i, name) in microphones.iter().enumerate() {
                let flags = MF_STRING
                    | if config.microphone.as_deref() == Some(name.as_str()) {
                        MF_CHECKED
                    } else {
                        MENU_ITEM_FLAGS(0)
                    };
                let label = wide(name);
                let _ = AppendMenuW(microphone, flags, MIC_FIRST as usize + i, pw(&label));
            }
            let _ = AppendMenuW(menu, MF_POPUP, microphone.0 as usize, w!("Micrófono"));
        }
        add(SHORTCUT, "Cambiar atajo…", MF_STRING);
        add(
            CLEANUP,
            "Limpieza básica",
            MF_STRING
                | if config.cleanup {
                    MF_CHECKED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
        );
        add(
            AUTOSTART,
            "Iniciar con Windows",
            MF_STRING
                | if autostart {
                    MF_CHECKED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
        );
        if copy {
            add(COPY, "Copiar último resultado", MF_STRING);
        }
        add(0, "", MF_SEPARATOR);
        add(EXIT, "Salir", MF_STRING);
        let mut point = POINT::default();
        let _ = GetCursorPos(&mut point);
        let _ = SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        )
        .0 as u32;
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        chosen
    }
}
