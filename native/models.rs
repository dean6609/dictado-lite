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
        Ok(crate::paths::data_dir()?.join("models").join(format!(
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
/// Download the model to its cache path, verifying size and hash. `progress`
/// receives (done, total) and returns false to cancel. A verified cached file
/// or a verified copy from a previous installation is reused without network.
pub fn download(model: &Model, mut progress: impl FnMut(u64, u64) -> bool) -> Result<PathBuf> {
    let weights = model.weights();
    let total = weights.size_bytes;
    let dest = model.cache_path()?;
    crate::paths::no_link(&dest)?;
    if !progress(0, total) {
        return Err(Error::Cancelled);
    }
    if dest.is_file() && verify(&dest, weights).is_ok() {
        return if progress(total, total) {
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
    crate::paths::no_link(&partial)?;
    let mut output = File::options()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(Error::message)?;
    let outcome = (|| -> Result<()> {
        let mut hash = Sha256::new();
        let mut done = 0;
        let mut sink = |chunk: &[u8]| -> Result<()> {
            if !progress(done, total) {
                return Err(Error::Cancelled);
            }
            done += chunk.len() as u64;
            if done > total {
                return Err(Error::message("Model download exceeds catalog size"));
            }
            hash.update(chunk);
            output.write_all(chunk).map_err(Error::message)
        };
        if let Some(mut legacy) = legacy_copy(model)? {
            // Reuse a verified model from the previous installation before
            // upgrade removes its owned payload.
            let mut buffer = [0; 64 * 1024];
            loop {
                let count = legacy.read(&mut buffer).map_err(Error::message)?;
                if count == 0 {
                    break;
                }
                sink(&buffer[..count])?;
            }
        } else {
            crate::transfer::get(
                &format!(
                    "/{}/resolve/{}/{}",
                    model.id, model.revision, weights.filename
                ),
                &mut sink,
            )?;
        }
        if done != total || format!("{:x}", hash.finalize()) != weights.sha256 {
            return Err(Error::message("Model integrity check failed"));
        }
        output.sync_all().map_err(Error::message)
    })();
    drop(output);
    if let Err(e) = outcome {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    if let Err(e) = publish(&partial, &dest) {
        let _ = std::fs::remove_file(&partial);
        return Err(e);
    }
    Ok(dest)
}

/// A verified model left by an installer that still bundled weights.
#[cfg(windows)]
fn legacy_copy(model: &Model) -> Result<Option<File>> {
    let legacy = PathBuf::from(
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| Error::message("LOCALAPPDATA unavailable"))?,
    )
    .join("Programs/DictadoLite/models")
    .join(&model.weights().filename);
    if !model.ready(&legacy) {
        return Ok(None);
    }
    crate::paths::no_link(&legacy)?;
    if verify(&legacy, model.weights()).is_err() {
        return Ok(None);
    }
    File::open(&legacy).map(Some).map_err(Error::message)
}
#[cfg(not(windows))]
fn legacy_copy(_: &Model) -> Result<Option<File>> {
    Ok(None)
}

/// Atomic replacement; a failed publication leaves the existing model intact.
fn publish(partial: &Path, dest: &Path) -> Result<()> {
    #[cfg(windows)]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use windows::{core::PCWSTR, Win32::Storage::FileSystem::*};
        let from: Vec<_> = partial.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<_> = dest.as_os_str().encode_wide().chain(Some(0)).collect();
        MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
        .map_err(Error::message)
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(partial, dest).map_err(Error::message)
    }
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
