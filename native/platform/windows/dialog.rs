//! Shared native dialog placement and system-color background.
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

pub fn center(hwnd: HWND) {
    unsafe {
        let monitor = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mut rect = RECT::default();
        if GetMonitorInfoW(monitor, &mut info).as_bool() && GetWindowRect(hwnd, &mut rect).is_ok() {
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let x = info.rcWork.left + (info.rcWork.right - info.rcWork.left - width) / 2;
            let y = info.rcWork.top + (info.rcWork.bottom - info.rcWork.top - height) / 2;
            let _ = SetWindowPos(
                hwnd,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }
}
/// Apply the system background to a native read-only/static control.
///
/// # Safety
/// `wp` must contain the valid HDC supplied by WM_CTLCOLORSTATIC.
pub unsafe fn background(wp: WPARAM) -> LRESULT {
    let dc = HDC(wp.0 as *mut _);
    SetBkColor(dc, COLORREF(GetSysColor(COLOR_BTNFACE)));
    SetTextColor(dc, COLORREF(GetSysColor(COLOR_WINDOWTEXT)));
    LRESULT(GetSysColorBrush(COLOR_BTNFACE).0 as isize)
}
