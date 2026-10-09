use super::{pw, runtime::WM_TRAY, wide};
use crate::{
    config::Config,
    locale::{shortcut, text},
};
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
pub const MODELS: u32 = 8;
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
        let tip = wide("Dictado Lite");
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
        let text = if paused {
            format!("Dictado Lite · {}", text("Pausado", "Paused"))
        } else {
            format!(
                "Dictado Lite · {} {} {}",
                text("Pulsa", "Press"),
                shortcut(config.key, config.modifiers),
                text("para iniciar/detener", "to start/stop")
            )
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
                text("Detener dictado", "Stop dictation")
            } else {
                text("Iniciar dictado", "Start dictation")
            },
            MF_STRING
                | if paused {
                    MF_GRAYED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
        );
        add(
            PAUSE,
            if paused {
                text("Reanudar", "Resume")
            } else {
                text("Pausar", "Pause")
            },
            MF_STRING,
        );
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
                windows::core::PCWSTR(
                    super::wide(text("Predeterminado de Windows", "Windows default")).as_ptr(),
                ),
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
            let _ = AppendMenuW(
                menu,
                MF_POPUP,
                microphone.0 as usize,
                windows::core::PCWSTR(super::wide(text("Micrófono", "Microphone")).as_ptr()),
            );
        }
        add(
            SHORTCUT,
            text("Cambiar atajo…", "Change shortcut…"),
            MF_STRING,
        );
        add(MODELS, text("Modelos…", "Models…"), MF_STRING);
        add(
            CLEANUP,
            text("Limpieza básica", "Basic cleanup"),
            MF_STRING
                | if config.cleanup {
                    MF_CHECKED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
        );
        add(
            AUTOSTART,
            text("Iniciar con Windows", "Start with Windows"),
            MF_STRING
                | if autostart {
                    MF_CHECKED
                } else {
                    MENU_ITEM_FLAGS(0)
                },
        );
        if copy {
            add(
                COPY,
                text("Copiar último resultado", "Copy last result"),
                MF_STRING,
            );
        }
        add(0, "", MF_SEPARATOR);
        add(EXIT, text("Salir", "Quit"), MF_STRING);
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
