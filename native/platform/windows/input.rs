use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

pub fn modifiers_released() -> bool {
    [VK_CONTROL, VK_MENU, VK_SHIFT]
        .iter()
        .all(|key| unsafe { GetAsyncKeyState(key.0 as i32) } >= 0)
}
pub fn paste(target: HWND) -> Result<(), String> {
    if unsafe { GetForegroundWindow() } != target {
        return Err("Destination focus changed".into());
    }
    if !modifiers_released() {
        return Err("Release shortcut modifiers".into());
    }
    let key = |vk, flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: flags,
                dwExtraInfo: super::hotkey::input_marker(),
                ..Default::default()
            },
        },
    };
    let inputs = [
        key(VK_CONTROL, KEYBD_EVENT_FLAGS(0)),
        key(VIRTUAL_KEY(0x56), KEYBD_EVENT_FLAGS(0)),
        key(VIRTUAL_KEY(0x56), KEYEVENTF_KEYUP),
        key(VK_CONTROL, KEYEVENTF_KEYUP),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent != inputs.len() as u32 {
        // A partial batch must never leave our synthesized modifier held.
        if sent > 0 {
            let releases = [
                key(VIRTUAL_KEY(0x56), KEYEVENTF_KEYUP),
                key(VK_CONTROL, KEYEVENTF_KEYUP),
            ];
            unsafe {
                SendInput(&releases, std::mem::size_of::<INPUT>() as i32);
            }
        }
        return Err("Windows rejected paste input".into());
    }
    Ok(())
}
