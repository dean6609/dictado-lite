use super::{
    integration,
    payload::{self, Manifest, Result, MARKER},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub fn root(test: bool) -> Result<PathBuf> {
    let path = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA unavailable")?)
        .join("Programs")
        .join(if test {
            "DictadoLite-QA"
        } else {
            "DictadoLite"
        });
    payload::no_reparse(&path)?;
    Ok(path)
}
fn installed(root: &Path) -> Result<Option<Manifest>> {
    payload::no_reparse(&root.join(MARKER))?;
    match fs::read(root.join(MARKER)) {
        Ok(json) => {
            let m: Manifest = serde_json::from_slice(&json)?;
            payload::validate(&m)?;
            Ok(Some(m))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if root.exists() && fs::read_dir(root)?.next().is_some() {
                Err("Install directory contains unowned files and no product marker".into())
            } else {
                Ok(None)
            }
        }
        Err(e) => Err(e.into()),
    }
}
pub fn install(test: bool, progress: impl Fn(u64, u64) -> bool) -> Result<PathBuf> {
    let root = root(test)?;
    let (mut file, manifest) = payload::open(&std::env::current_exe()?)?;
    let old = installed(&root)?;
    let previous = old
        .as_ref()
        .map(|m| {
            m.files
                .iter()
                .map(|e| e.name.as_str())
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    for entry in &manifest.files {
        let target = root.join(&entry.name);
        payload::no_reparse(&target)?;
        if target.exists() && !previous.contains(entry.name.as_str()) {
            return Err(format!("Preserving unowned file: {}", entry.name).into());
        }
    }
    // Graceful app exit; never force-terminate a process or touch another application.
    if root.join("dictado-lite.exe").exists() {
        let status = Command::new(root.join("dictado-lite.exe"))
            .arg("--exit")
            .status()?;
        if !status.success() {
            return Err("Close Dictado Lite before upgrading".into());
        }
    }
    let parent = root.parent().ok_or("Invalid install path")?;
    fs::create_dir_all(parent)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let stage = parent.join(format!("DictadoLite-stage-{}-{stamp}", std::process::id()));
    fs::create_dir(&stage)?;
    let result = (|| -> Result<()> {
        payload::extract(&mut file, &manifest, &stage, &progress)?;
        if !progress(1, 1) {
            return Err("Installation cancelled".into());
        }
        // Test write/rename permissions before altering any prior owned file.
        fs::create_dir_all(&root)?;
        let backup = stage.join("previous");
        fs::create_dir(&backup)?;
        let mut moved = Vec::new();
        let mut published = Vec::new();
        let commit = (|| -> Result<()> {
            if let Some(old) = &old {
                for entry in &old.files {
                    let from = root.join(&entry.name);
                    payload::no_reparse(&from)?;
                    if from.exists() {
                        let to = backup.join(&entry.name);
                        fs::create_dir_all(to.parent().ok_or("Invalid backup path")?)?;
                        fs::rename(&from, &to)?;
                        moved.push(entry.name.clone());
                    }
                }
            }
            for entry in &manifest.files {
                let to = root.join(&entry.name);
                payload::no_reparse(&to)?;
                fs::create_dir_all(to.parent().ok_or("Invalid install file")?)?;
                fs::rename(stage.join(&entry.name), &to)?;
                published.push(entry.name.clone());
            }
            fs::write(stage.join(MARKER), serde_json::to_vec_pretty(&manifest)?)?;
            #[cfg(windows)]
            unsafe {
                use std::os::windows::ffi::OsStrExt;
                use windows::{core::PCWSTR, Win32::Storage::FileSystem::*};
                let from: Vec<_> = stage
                    .join(MARKER)
                    .as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                let to: Vec<_> = root
                    .join(MARKER)
                    .as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                MoveFileExW(
                    PCWSTR(from.as_ptr()),
                    PCWSTR(to.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )?;
            }
            Ok(())
        })();
        if let Err(error) = commit {
            for name in published.iter().rev() {
                fs::remove_file(root.join(name))?;
            }
            for name in moved.iter().rev() {
                fs::rename(backup.join(name), root.join(name))?;
            }
            return Err(error);
        }
        if let Some(old) = &old {
            payload::remove_files(&backup, old)?;
        }
        let _ = fs::remove_dir(backup);
        if !test {
            integration::register(
                &root,
                &manifest.version,
                manifest.files.iter().map(|e| e.bytes).sum(),
            )?;
        }
        Ok(())
    })();
    // Fixed manifest entries only. No recursive delete, including on extraction failure.
    let _ = payload::remove_files(&stage, &manifest);
    let _ = fs::remove_file(stage.join(MARKER));
    let _ = fs::remove_dir(&stage);
    result?;
    Ok(root)
}
pub fn uninstall(test: bool) -> Result<()> {
    let root = root(test)?;
    let manifest = installed(&root)?.ok_or("No owned installation found")?;
    let app = root.join("dictado-lite.exe");
    if app.exists() {
        let _ = Command::new(app).arg("--exit").status()?;
    }
    payload::remove_files(&root, &manifest)?;
    if !test {
        integration::remove()?;
    }
    fs::remove_file(root.join(MARKER))?;
    let _ = fs::remove_dir(root);
    Ok(())
}
/// Relocate only the tiny uninstaller stub so its installed image can be removed.
pub fn launch_uninstall(test: bool, quiet: bool) -> Result<()> {
    let temp = std::env::temp_dir().join(format!(
        "dictado-uninstall-{}-{}.exe",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    payload::no_reparse(&temp)?;
    fs::copy(std::env::current_exe()?, &temp)?;
    let mut command = Command::new(temp);
    command.arg("--remove");
    if test {
        command.arg("--test");
    }
    if quiet {
        command.arg("--quiet");
    }
    command.spawn()?;
    Ok(())
}
