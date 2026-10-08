//! Per-user Windows integration; no administrator or global settings.
use super::payload::{no_reparse, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};
use windows::{
    core::{w, Interface, PCWSTR},
    Win32::{
        Foundation::*,
        System::{Com::*, Registry::*},
        UI::Shell::*,
    },
};
fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
const KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\DictadoLite");
fn shortcut() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("APPDATA").ok_or("APPDATA unavailable")?)
            .join("Microsoft/Windows/Start Menu/Programs/Dictado Lite.lnk"),
    )
}
pub fn register(root: &Path, version: &str, bytes: u64) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            KEY,
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let write = (|| -> Result<()> {
            for (name, value) in [
                ("DisplayName", "Dictado Lite".to_string()),
                ("DisplayVersion", version.into()),
                ("Publisher", "Dictado Lite contributors".into()),
                ("InstallLocation", root.display().to_string()),
                (
                    "DisplayIcon",
                    root.join("dictado-lite.exe").display().to_string(),
                ),
                (
                    "UninstallString",
                    format!(
                        "\"{}\" --uninstall",
                        root.join("dictado-uninstall.exe").display()
                    ),
                ),
            ] {
                let name = wide(name);
                let value = wide(value);
                let data = std::slice::from_raw_parts(value.as_ptr().cast(), value.len() * 2);
                RegSetValueExW(key, PCWSTR(name.as_ptr()), None, REG_SZ, Some(data)).ok()?;
            }
            for (name, value) in [
                (w!("NoModify"), 1u32),
                (w!("NoRepair"), 1),
                (w!("EstimatedSize"), (bytes / 1024) as u32),
            ] {
                RegSetValueExW(key, name, None, REG_DWORD, Some(&value.to_le_bytes())).ok()?;
            }
            Ok(())
        })();
        let _ = RegCloseKey(key);
        write?;
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let result = (|| -> Result<()> {
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
            let exe = wide(root.join("dictado-lite.exe"));
            let cwd = wide(root);
            link.SetPath(PCWSTR(exe.as_ptr()))?;
            link.SetWorkingDirectory(PCWSTR(cwd.as_ptr()))?;
            link.SetIconLocation(PCWSTR(exe.as_ptr()), 0)?;
            link.SetDescription(w!("Dictado local · Ctrl+Alt+Space"))?;
            let path = shortcut()?;
            no_reparse(&path)?;
            fs::create_dir_all(path.parent().ok_or("Invalid shortcut directory")?)?;
            let path = wide(path);
            let persist: IPersistFile = link.cast()?;
            persist.Save(PCWSTR(path.as_ptr()), true)?;
            Ok(())
        })();
        CoUninitialize();
        result
    }
}
pub fn remove() -> Result<()> {
    let path = shortcut()?;
    no_reparse(&path)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    unsafe {
        let error = RegDeleteTreeW(HKEY_CURRENT_USER, KEY);
        if error != ERROR_FILE_NOT_FOUND {
            error.ok()?;
        }
        let mut run = HKEY::default();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run"),
            None,
            KEY_SET_VALUE,
            &mut run,
        )
        .is_ok()
        {
            let error = RegDeleteValueW(run, w!("DictadoLite"));
            let _ = RegCloseKey(run);
            if error != ERROR_FILE_NOT_FOUND {
                error.ok()?;
            }
        }
    }
    Ok(())
}
