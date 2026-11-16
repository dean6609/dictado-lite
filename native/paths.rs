//! Per-user settings and model cache locations. Program files live elsewhere.
use crate::engine::{Error, Result};
use std::path::PathBuf;

/// Settings, the model cache and the optional word list share one folder.
pub fn data_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        Ok(PathBuf::from(
            std::env::var_os("LOCALAPPDATA")
                .ok_or_else(|| Error::message("LOCALAPPDATA unavailable"))?,
        )
        .join("DictadoLite"))
    }
    #[cfg(target_os = "macos")]
    {
        Ok(home()?.join("Library/Application Support/DictadoLite"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(
            match std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
                Some(base) => PathBuf::from(base),
                None => home()?.join(".local/share"),
            }
            .join("dictado-lite"),
        )
    }
}

#[cfg(not(windows))]
fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| Error::message("HOME unavailable"))
}

/// Refuse paths that pass through a symbolic link or Windows reparse point.
pub fn no_link(path: &std::path::Path) -> Result<()> {
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    metadata.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = metadata.file_type().is_symlink();
                if linked {
                    return Err(Error::message("Linked path refused"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::message(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn data_dir_is_absolute_and_named() {
        let dir = super::data_dir().unwrap();
        assert!(dir.is_absolute());
        assert!(dir.to_string_lossy().to_lowercase().contains("dictado"));
    }
}
