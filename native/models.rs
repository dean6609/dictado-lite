//! Handy's vendored GGUF catalog and atomic, verified model downloads.
use crate::engine::{Error, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
};

#[derive(Clone, Deserialize)]
pub struct Model {
    pub id: String,
    pub revision: String,
    pub name: String,
    pub description: String,
    pub architecture: String,
    pub license: String,
    pub languages: Vec<String>,
    pub files: Vec<Weights>,
    pub default_quant: String,
    pub capabilities: Capabilities,
    #[serde(default)]
    pub recommended: bool,
}
#[derive(Clone, Deserialize)]
pub struct Capabilities {
    pub lang_detect: bool,
}
#[derive(Clone, Deserialize)]
pub struct Weights {
    pub filename: String,
    pub quant: String,
    pub size_bytes: u64,
    pub sha256: String,
}
#[derive(Deserialize)]
struct Catalog {
    models: Vec<Model>,
}
pub fn catalog() -> &'static [Model] {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    &CATALOG
        .get_or_init(|| {
            serde_json::from_str(include_str!("../models/catalog.json"))
                .expect("validated bundled catalog")
        })
        .models
}
impl Model {
    pub fn language_hint(&self) -> Option<&str> {
        if self.capabilities.lang_detect {
            return None;
        }
        let preferred = crate::locale::text("es", "en");
        self.languages
            .iter()
            .find(|s| s.as_str() == preferred)
            .or_else(|| self.languages.iter().find(|s| s.as_str() == "en"))
            .or_else(|| self.languages.first())
            .map(String::as_str)
    }
    pub fn weights(&self) -> &Weights {
        self.files
            .iter()
            .find(|f| f.quant == self.default_quant)
            .expect("default quantization exists")
    }
    pub fn key(&self) -> String {
        format!("{}/{}", self.id, self.weights().filename)
    }
    pub fn cache_path(&self) -> Result<PathBuf> {
        let base = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| Error::message("LOCALAPPDATA unavailable"))?;
        Ok(PathBuf::from(base).join("DictadoLite/models").join(format!(
            "{}-{}",
            &self.weights().sha256[..16],
            self.weights().filename
        )))
    }
    pub fn ready(&self, path: &Path) -> bool {
        std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == self.weights().size_bytes)
    }
}
pub fn selected(id: &str) -> &'static Model {
    catalog()
        .iter()
        .find(|m| m.key() == id)
        .unwrap_or_else(recommended)
}
pub fn recommended() -> &'static Model {
    catalog()
        .iter()
        .find(|m| m.id == "handy-computer/parakeet-tdt-0.6b-v3-gguf")
        .expect("recommended model in catalog")
}
pub fn recommendations() -> Vec<usize> {
    let preferred = crate::locale::text("es", "en");
    let first = catalog()
        .iter()
        .position(|m| m.key() == recommended().key())
        .expect("recommended model");
    let mut indices = vec![first];
    let alternatives: Vec<String> =
        serde_json::from_str(include_str!("../models/recommended.json"))
            .expect("validated recommendations");
    indices.extend(alternatives.iter().filter_map(|id| {
        catalog()
            .iter()
            .position(|m| m.id == *id && m.languages.iter().any(|s| s == preferred))
    }));
    indices
}
pub fn for_path(path: &Path) -> Result<&'static Model> {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::message("Invalid model path"))?;
    catalog()
        .iter()
        .find(|m| name.ends_with(&m.weights().filename))
        .ok_or_else(|| Error::message("Model is not in the supported catalog"))
}
pub fn verify(path: &Path, weights: &Weights) -> Result<()> {
    let mut file = File::open(path).map_err(Error::message)?;
    if file.metadata().map_err(Error::message)?.len() != weights.size_bytes {
        return Err(Error::message("Model size mismatch"));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(Error::message)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != weights.sha256 {
        return Err(Error::message("Model integrity check failed"));
    }
    Ok(())
}
#[cfg(windows)]
pub fn download(model: &Model, mut progress: impl FnMut(u64, u64) -> bool) -> Result<PathBuf> {
    use std::ffi::c_void;
    use windows::{
        core::{w, PCWSTR},
        Win32::Networking::WinHttp::*,
    };
    struct Internet(*mut c_void);
    impl Internet {
        fn new(ptr: *mut c_void) -> Result<Self> {
            if ptr.is_null() {
                Err(Error::message(windows::core::Error::from_win32()))
            } else {
                Ok(Self(ptr))
            }
        }
    }
    impl Drop for Internet {
        fn drop(&mut self) {
            unsafe {
                let _ = WinHttpCloseHandle(self.0);
            }
        }
    }
    fn safe_path(path: &Path) -> Result<()> {
        use std::os::windows::fs::MetadataExt;
        for p in path.ancestors() {
            match std::fs::symlink_metadata(p) {
                Ok(m) if m.file_attributes() & 0x400 != 0 => {
                    return Err(Error::message("Reparse model path refused"))
                }
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(Error::message(e))
                }
                _ => {}
            }
        }
        Ok(())
    }
    let dest = model.cache_path()?;
    safe_path(&dest)?;
    if !progress(0, model.weights().size_bytes) {
        return Err(Error::Cancelled);
    }
    if dest.is_file() && verify(&dest, model.weights()).is_ok() {
        return if progress(model.weights().size_bytes, model.weights().size_bytes) {
            Ok(dest)
        } else {
            Err(Error::Cancelled)
        };
    }
    std::fs::create_dir_all(
        dest.parent()
            .ok_or_else(|| Error::message("Invalid model path"))?,
    )
    .map_err(Error::message)?;
    // A unique partial file is never offered to inference. No interrupted download becomes a model.
    let partial = dest.with_extension(format!(
        "{}-{}.part",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(Error::message)?
            .as_nanos()
    ));
    safe_path(&partial)?;
    let mut output = File::options()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(Error::message)?;
    let outcome = (|| -> Result<()> {
        if !progress(0, model.weights().size_bytes) {
            return Err(Error::Cancelled);
        }
        // Reuse a verified model from the previous installation before upgrade
        // removes its owned payload. Model data moves to the independent cache.
        let legacy = PathBuf::from(
            std::env::var_os("LOCALAPPDATA")
                .ok_or_else(|| Error::message("LOCALAPPDATA unavailable"))?,
        )
        .join("Programs/DictadoLite/models")
        .join(&model.weights().filename);
        if model.ready(&legacy) {
            safe_path(&legacy)?;
            if verify(&legacy, model.weights()).is_ok() {
                let mut input = File::open(&legacy).map_err(Error::message)?;
                let mut buffer = [0; 64 * 1024];
                let mut hash = Sha256::new();
                let mut done = 0;
                loop {
                    if !progress(done, model.weights().size_bytes) {
                        return Err(Error::Cancelled);
                    }
                    let count = input.read(&mut buffer).map_err(Error::message)?;
                    if count == 0 {
                        break;
                    }
                    done += count as u64;
                    if done > model.weights().size_bytes {
                        return Err(Error::message("Model size mismatch"));
                    }
                    hash.update(&buffer[..count]);
                    output.write_all(&buffer[..count]).map_err(Error::message)?;
                }
                if done != model.weights().size_bytes
                    || format!("{:x}", hash.finalize()) != model.weights().sha256
                {
                    return Err(Error::message("Model integrity check failed"));
                }
                output.sync_all().map_err(Error::message)?;
                return Ok(());
            }
        }
        unsafe {
            let session = Internet::new(WinHttpOpen(
                w!("DictadoLite"),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            ))?;
            WinHttpSetTimeouts(session.0, 15_000, 15_000, 15_000, 15_000)
                .map_err(Error::message)?;
            let connection =
                Internet::new(WinHttpConnect(session.0, w!("huggingface.co"), 443, 0))?;
            let url: Vec<_> = format!(
                "/{}/resolve/{}/{}",
                model.id,
                model.revision,
                model.weights().filename
            )
            .encode_utf16()
            .chain(Some(0))
            .collect();
            let request = Internet::new(WinHttpOpenRequest(
                connection.0,
                w!("GET"),
                PCWSTR(url.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            ))?;
            WinHttpSendRequest(request.0, None, None, 0, 0, 0).map_err(Error::message)?;
            WinHttpReceiveResponse(request.0, std::ptr::null_mut()).map_err(Error::message)?;
            let mut status = 0u32;
            let mut bytes = 4;
            let mut index = 0;
            WinHttpQueryHeaders(
                request.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some((&mut status as *mut u32).cast()),
                &mut bytes,
                &mut index,
            )
            .map_err(Error::message)?;
            if status != 200 {
                return Err(Error::message(format!("Model download HTTP {status}")));
            }
            let mut buffer = [0u8; 64 * 1024];
            let mut done = 0;
            let mut hash = Sha256::new();
            loop {
                if !progress(done, model.weights().size_bytes) {
                    return Err(Error::Cancelled);
                }
                let mut read = 0;
                WinHttpReadData(
                    request.0,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut read,
                )
                .map_err(Error::message)?;
                if read == 0 {
                    break;
                }
                done += read as u64;
                if done > model.weights().size_bytes {
                    return Err(Error::message("Model download exceeds catalog size"));
                }
                hash.update(&buffer[..read as usize]);
                output
                    .write_all(&buffer[..read as usize])
                    .map_err(Error::message)?;
            }
            if done != model.weights().size_bytes
                || format!("{:x}", hash.finalize()) != model.weights().sha256
            {
                return Err(Error::message("Model integrity check failed"));
            }
            output.sync_all().map_err(Error::message)?;
        }
        Ok(())
    })();
    drop(output);
    if let Err(e) = outcome {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    // Windows replacement is atomic; a failed publication leaves the existing model intact.
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::*;
        let from: Vec<_> = partial.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = dest.as_os_str().encode_wide().chain(Some(0)).collect();
        if let Err(e) = MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        ) {
            let _ = std::fs::remove_file(&partial);
            return Err(Error::message(e));
        }
    }
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_is_complete_and_pinned() {
        assert!(catalog().len() > 60);
        for model in catalog() {
            let weights = model.weights();
            assert!(model.id.starts_with("handy-computer/") && !model.id.contains(".."));
            assert_eq!(model.revision.len(), 40);
            assert!(model.revision.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(!weights.filename.contains(['/', '\\', ':']));
            assert!(weights.size_bytes > 0);
            assert_eq!(weights.sha256.len(), 64);
            assert!(weights.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(!model.license.is_empty());
        }
        assert_eq!(
            recommended().weights().size_bytes,
            crate::engine::MODEL_BYTES
        );
        assert_eq!(recommended().weights().sha256, crate::engine::MODEL_SHA256);
        let recommendations = recommendations();
        assert!((1..=4).contains(&recommendations.len()));
        assert_eq!(catalog()[recommendations[0]].key(), recommended().key());
        let unique: std::collections::HashSet<_> = recommendations.iter().collect();
        assert_eq!(unique.len(), recommendations.len());
    }
    #[test]
    fn rejects_incomplete_and_corrupt_downloads() {
        let path =
            std::env::temp_dir().join(format!("dictado-download-{}.gguf", std::process::id()));
        let weights = Weights {
            filename: "fixture.gguf".into(),
            quant: "test".into(),
            size_bytes: 3,
            sha256: format!("{:x}", Sha256::digest(b"abc")),
        };
        std::fs::write(&path, b"ab").unwrap();
        assert!(verify(&path, &weights).is_err());
        std::fs::write(&path, b"abd").unwrap();
        assert!(verify(&path, &weights).is_err());
        std::fs::write(&path, b"abc").unwrap();
        assert!(verify(&path, &weights).is_ok());
        std::fs::remove_file(path).unwrap();
    }
}
