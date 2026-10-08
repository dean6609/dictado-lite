//! Nonactivating, per-monitor native pill. Hidden releases drawing resources.
mod geometry;
mod render;
use super::{pw, wide};
use geometry::Frame;
use render::{Content, Painter};
use std::cell::Cell;
use std::time::{Duration, Instant};
use windows::core::w;
use windows::core::BOOL;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::{Accessibility::*, HiDpi::*, WindowsAndMessaging::*};
pub const WM_OVERLAY: u32 = WM_APP + 8;
pub const WM_LAYOUT: u32 = WM_APP + 9;
pub const COPY_OR_OPTIONS: usize = 1;
pub const RETRY: usize = 2;
pub const DISMISS: usize = 3;
struct Route {
    controller: HWND,
    scale: Cell<f32>,
    error: Cell<bool>,
    buttons: Cell<[HWND; 3]>,
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_MOUSEACTIVATE {
        return LRESULT(MA_NOACTIVATE as isize);
    }
    let route = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Route;
    if !route.is_null() {
        let route = &*route;
        if msg == WM_DRAWITEM {
            return LRESULT(1);
        }
        if msg == WM_COMMAND && route.error.get() && lp.0 != 0 {
            let action = wp.0 & 0xFFFF;
            if (1..=3).contains(&action) {
                let _ = PostMessageW(
                    Some(route.controller),
                    WM_OVERLAY,
                    WPARAM(action),
                    LPARAM(0),
                );
            }
            return LRESULT(0);
        }
        if msg == WM_LBUTTONUP && route.error.get() {
            let x = (lp.0 as u16 as i16) as f32 / route.scale.get();
            let y = ((lp.0 >> 16) as u16 as i16) as f32 / route.scale.get();
            if (63.0..=87.0).contains(&y) {
                let action = if (24.0..=107.0).contains(&x) {
                    COPY_OR_OPTIONS
                } else if (117.0..=210.0).contains(&x) {
                    RETRY
                } else if (220.0..=278.0).contains(&x) {
                    DISMISS
                } else {
                    0
                };
                if action != 0 {
                    let _ = PostMessageW(
                        Some(route.controller),
                        WM_OVERLAY,
                        WPARAM(action),
                        LPARAM(0),
                    );
                }
            }
            return LRESULT(0);
        }
        if [WM_DPICHANGED, WM_DISPLAYCHANGE, WM_SETTINGCHANGE].contains(&msg) {
            let _ = PostMessageW(Some(route.controller), WM_LAYOUT, WPARAM(0), LPARAM(0));
        }
    }
    if msg == WM_PAINT {
        let mut paint = PAINTSTRUCT::default();
        BeginPaint(hwnd, &mut paint);
        let _ = EndPaint(hwnd, &paint);
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
enum View {
    Hidden,
    Listening,
    Processing,
    Success,
    Error(String, bool),
}
pub struct Overlay {
    hwnd: HWND,
    route: Box<Route>,
    view: View,
    painter: Option<Painter>,
    target: HWND,
    started: Instant,
    levels: [f32; 9],
    reduced_motion: bool,
    high_contrast: bool,
    inspect: bool,
    focus_preserved: bool,
}
impl Overlay {
    pub fn create(controller: HWND, inspect: bool) -> windows::core::Result<Self> {
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None)?.0);
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: instance,
                lpszClassName: w!("DictadoMicroPill"),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                ..Default::default()
            });
            let hwnd = CreateWindowExW(
                (if inspect {
                    WS_EX_APPWINDOW
                } else {
                    WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
                }) | WS_EX_TOPMOST
                    | WS_EX_LAYERED
                    | WS_EX_TRANSPARENT,
                w!("DictadoMicroPill"),
                w!("Dictado Lite"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                if inspect { None } else { Some(controller) },
                None,
                Some(instance),
                None,
            )?;
            let route = Box::new(Route {
                controller,
                scale: Cell::new(1.0),
                error: Cell::new(false),
                buttons: Cell::new([HWND::default(); 3]),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*route) as *const Route as isize);
            Ok(Self {
                hwnd,
                route,
                view: View::Hidden,
                painter: None,
                target: HWND::default(),
                started: Instant::now(),
                levels: [0.0; 9],
                reduced_motion: false,
                high_contrast: false,
                inspect,
                focus_preserved: true,
            })
        }
    }
    fn name(&self, name: &str) {
        let title = wide(name);
        unsafe {
            let _ = SetWindowTextW(self.hwnd, pw(&title));
            NotifyWinEvent(
                EVENT_OBJECT_NAMECHANGE,
                self.hwnd,
                OBJID_WINDOW.0,
                CHILDID_SELF as i32,
            );
        }
    }
    pub fn listening(&mut self, target: HWND) -> windows::core::Result<()> {
        self.target = target;
        self.view = View::Listening;
        self.levels = [0.0; 9];
        self.started = Instant::now();
        self.name("Dictado Lite: escuchando");
        self.layout()?;
        self.advance([0.0; 9])?;
        Ok(())
    }
    pub fn processing(&mut self) -> windows::core::Result<()> {
        self.view = View::Processing;
        self.started = Instant::now();
        self.name("Dictado Lite: reconociendo");
        self.advance([0.0; 9])?;
        Ok(())
    }
    pub fn processing_at(&mut self, target: HWND) -> windows::core::Result<()> {
        self.target = target;
        self.view = View::Processing;
        self.started = Instant::now();
        self.name("Dictado Lite: reconociendo");
        self.layout()?;
        self.advance([0.0; 9])?;
        Ok(())
    }
    pub fn success(&mut self) -> windows::core::Result<()> {
        self.view = View::Success;
        self.started = Instant::now();
        self.name("Dictado Lite: texto insertado");
        self.advance([0.0; 9])?;
        Ok(())
    }
    pub fn error(
        &mut self,
        target: HWND,
        message: &str,
        recoverable: bool,
    ) -> windows::core::Result<()> {
        self.target = target;
        self.view = View::Error(message.into(), recoverable);
        self.name(&format!(
            "Dictado Lite: {message} Acciones disponibles también en la bandeja."
        ));
        self.layout()?;
        self.advance([0.0; 9])?;
        unsafe {
            NotifyWinEvent(
                EVENT_SYSTEM_ALERT,
                self.hwnd,
                OBJID_WINDOW.0,
                CHILDID_SELF as i32,
            );
        }
        Ok(())
    }
    pub fn hide(&mut self) {
        unsafe {
            for button in self.route.buttons.replace([HWND::default(); 3]) {
                if !button.0.is_null() {
                    let _ = DestroyWindow(button);
                }
            }
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
        self.view = View::Hidden;
        self.painter = None;
        self.levels = [0.0; 9];
        self.route.error.set(false);
    }
    pub fn layout(&mut self) -> windows::core::Result<()> {
        if matches!(self.view, View::Hidden) {
            return Ok(());
        }
        unsafe {
            let foreground = GetForegroundWindow();
            let mut animations = BOOL(1);
            let _ = SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                Some((&mut animations as *mut BOOL).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
            self.reduced_motion = !animations.as_bool();
            let mut contrast = HIGHCONTRASTW {
                cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
                ..Default::default()
            };
            let _ = SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                contrast.cbSize,
                Some((&mut contrast as *mut HIGHCONTRASTW).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            );
            self.high_contrast = contrast.dwFlags.contains(HCF_HIGHCONTRASTON);
            let monitor = MonitorFromWindow(self.target, MONITOR_DEFAULTTONEAREST);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return Err(windows::core::Error::from_win32());
            }
            let mut dpi = 96;
            let mut dpi_y = 96;
            let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y);
            let frame = Frame::place(
                [
                    info.rcWork.left,
                    info.rcWork.top,
                    info.rcWork.right,
                    info.rcWork.bottom,
                ],
                dpi,
                matches!(self.view, View::Error(..)),
            );
            self.route.scale.set(frame.dpi as f32 / 96.0);
            self.route.error.set(frame.error);
            let mut buttons = self.route.buttons.get();
            if let View::Error(_, recoverable) = &self.view {
                for (i, label, left, right) in [
                    (
                        0,
                        if *recoverable { "Copiar" } else { "Opciones" },
                        24.0,
                        107.0,
                    ),
                    (1, "Reintentar", 117.0, 210.0),
                    (2, "Cerrar", 220.0, 278.0),
                ] {
                    let text = wide(label);
                    let scale = self.route.scale.get();
                    if buttons[i].0.is_null() {
                        buttons[i] = CreateWindowExW(
                            WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                            w!("BUTTON"),
                            pw(&text),
                            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_OWNERDRAW as u32),
                            (left * scale).round() as i32,
                            (63.0 * scale).round() as i32,
                            ((right - left) * scale).round() as i32,
                            (24.0 * scale).round() as i32,
                            Some(self.hwnd),
                            Some(HMENU((i + 1) as *mut _)),
                            Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                            None,
                        )?;
                    } else {
                        let _ = SetWindowTextW(buttons[i], pw(&text));
                        SetWindowPos(
                            buttons[i],
                            None,
                            (left * scale).round() as i32,
                            (63.0 * scale).round() as i32,
                            ((right - left) * scale).round() as i32,
                            (24.0 * scale).round() as i32,
                            SWP_NOZORDER | SWP_NOACTIVATE,
                        )?;
                    }
                }
                self.route.buttons.set(buttons);
            } else {
                for button in self.route.buttons.replace([HWND::default(); 3]) {
                    if !button.0.is_null() {
                        let _ = DestroyWindow(button);
                    }
                }
            }
            let mut style = (if self.inspect {
                WS_EX_APPWINDOW
            } else {
                WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW
            }) | WS_EX_TOPMOST
                | WS_EX_LAYERED;
            if !frame.error {
                style |= WS_EX_TRANSPARENT;
            }
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style.0 as isize);
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                frame.x,
                frame.y,
                frame.width,
                frame.height,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            )?;
            self.painter = Some(Painter::new(frame, self.high_contrast)?);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            self.focus_preserved = GetForegroundWindow() == foreground;
            Ok(())
        }
    }
    pub fn metrics(&self) -> serde_json::Value {
        let frame = self.painter.as_ref().map(|painter| serde_json::json!({"x":painter.frame.x,"y":painter.frame.y,"width":painter.frame.width,"height":painter.frame.height,"dpi":painter.frame.dpi,"error":painter.frame.error}));
        serde_json::json!({"frame":frame,"focus_preserved_on_show":self.focus_preserved,"reduced_motion":self.reduced_motion,"high_contrast":self.high_contrast,"inspection":self.inspect})
    }
    pub fn menu_owner(&self) -> Option<HWND> {
        if matches!(self.view, View::Hidden) {
            None
        } else {
            Some(self.hwnd)
        }
    }
    pub fn advance(&mut self, levels: [f32; 9]) -> windows::core::Result<bool> {
        if matches!(self.view, View::Hidden) {
            return Ok(false);
        }
        if matches!(self.view, View::Success)
            && self.started.elapsed() >= Duration::from_millis(220)
        {
            self.hide();
            return Ok(false);
        }
        let Some(painter) = &self.painter else {
            return Ok(false);
        };
        let opacity = if self.reduced_motion || matches!(self.view, View::Error(..)) {
            1.0
        } else if matches!(self.view, View::Success) {
            1.0 - self.started.elapsed().as_secs_f32() / 0.22
        } else {
            (self.started.elapsed().as_secs_f32() / 0.13).min(1.0)
        };
        let content = match &self.view {
            View::Listening => {
                for (current, next) in self.levels.iter_mut().zip(levels) {
                    *current += (next - *current) * if next > *current { 0.65 } else { 0.20 };
                }
                Content::Listening(&self.levels)
            }
            View::Processing => Content::Processing(if self.reduced_motion {
                1.0
            } else {
                0.88 + 0.12 * (self.started.elapsed().as_secs_f32() * 3.4).cos()
            }),
            View::Success => Content::Success,
            View::Error(message, recoverable) => Content::Error(message, *recoverable),
            View::Hidden => return Ok(false),
        };
        painter.draw(self.hwnd, content, opacity)?;
        Ok(true)
    }
}
impl Drop for Overlay {
    fn drop(&mut self) {
        self.hide();
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
