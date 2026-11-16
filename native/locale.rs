//! UI language follows the operator's display language; English fallback.
use std::sync::OnceLock;

pub fn spanish_ui(language_id: u16) -> bool {
    language_id & 0x3ff == 0x0a
}
/// POSIX locale names such as `es_ES.UTF-8`, `es` or `C`.
pub fn spanish_locale(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "es" || name.starts_with("es_") || name.starts_with("es-") || name.starts_with("es.")
}
pub fn text(es: &'static str, en: &'static str) -> &'static str {
    static SPANISH: OnceLock<bool> = OnceLock::new();
    let spanish = *SPANISH.get_or_init(|| {
        #[cfg(windows)]
        unsafe {
            // UI language, not region, keyboard layout or recognition language.
            spanish_ui(windows::Win32::Globalization::GetUserDefaultUILanguage())
        }
        #[cfg(not(windows))]
        {
            ["LC_ALL", "LC_MESSAGES", "LANG"]
                .iter()
                .filter_map(|name| std::env::var(name).ok())
                .find(|value| !value.is_empty())
                .is_some_and(|value| spanish_locale(&value))
        }
    });
    if spanish {
        es
    } else {
        en
    }
}

pub fn shortcut(key: u16, mods: u8) -> String {
    let mut parts: Vec<String> = [(2, "Ctrl"), (4, "Alt"), (1, text("Mayús", "Shift"))]
        .into_iter()
        .filter(|(bit, _)| mods & bit != 0)
        .map(|(_, s)| s.into())
        .collect();
    let name = match key {
        0x20 => text("Espacio", "Space").into(),
        0x09 => text("Tabulador", "Tab").into(),
        0x0D => "Enter".into(),
        0x70..=0x87 => format!("F{}", key - 0x70 + 1),
        _ => {
            #[cfg(windows)]
            unsafe {
                use windows::Win32::UI::Input::KeyboardAndMouse::*;
                let scan = MapVirtualKeyW(key as u32, MAPVK_VK_TO_VSC_EX);
                let extended = if scan & 0xff00 != 0 { 1 << 24 } else { 0 };
                let mut buffer = [0; 64];
                let length = GetKeyNameTextW(((scan & 0xff) << 16 | extended) as i32, &mut buffer);
                String::from_utf16_lossy(&buffer[..length.max(0) as usize])
            }
            #[cfg(not(windows))]
            {
                char::from_u32(key as u32).unwrap_or('?').to_string()
            }
        }
    };
    parts.push(name);
    parts.join("+")
}

#[cfg(test)]
mod tests {
    #[test]
    fn spanish_regions_and_fallback() {
        for id in [0x040a, 0x080a, 0x0c0a, 0x240a] {
            assert!(super::spanish_ui(id));
        }
        for id in [0x0409, 0x040c, 0, 0x0425] {
            assert!(!super::spanish_ui(id));
        }
        for name in ["es", "es_ES.UTF-8", "es_MX", "ES_AR"] {
            assert!(super::spanish_locale(name));
        }
        for name in ["en_US.UTF-8", "C", "POSIX", "est_EE", ""] {
            assert!(!super::spanish_locale(name));
        }
    }
}
