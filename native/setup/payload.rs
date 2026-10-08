//! Bounded, hash-verified appended payload. No ZIP extraction or executable script.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path},
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const PRODUCT: &str = "dean6609/dictado-lite";
pub const MAGIC: &[u8; 16] = b"DICTADO-PACK-v1!";
pub const MARKER: &str = "installed-files.json";
#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
    pub offset: u64,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub product: String,
    pub version: String,
    pub files: Vec<Entry>,
}
fn reserved(part: &str) -> bool {
    let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.len() == 4
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}
pub fn relative(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 180
        && !name.contains(['\\', ':'])
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !reserved(part)
                && !part.ends_with(['.', ' '])
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_ .+".contains(c))
        })
        && Path::new(name)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}
pub fn validate(manifest: &Manifest) -> Result<()> {
    if manifest.product != PRODUCT || manifest.files.is_empty() || manifest.files.len() > 1024 {
        return Err("Invalid product manifest".into());
    }
    let mut names = HashSet::new();
    for file in &manifest.files {
        if !relative(&file.name)
            || file.name.eq_ignore_ascii_case(MARKER)
            || !names.insert(file.name.to_ascii_lowercase())
            || file.bytes > 800_000_000
            || file.sha256.len() != 64
            || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid payload file".into());
        }
    }
    Ok(())
}
pub fn open(path: &Path) -> Result<(File, Manifest)> {
    let mut file = File::open(path)?;
    let total = file.metadata()?.len();
    if total < 24 {
        return Err("Missing installer payload".into());
    }
    file.seek(SeekFrom::End(-24))?;
    let mut footer = [0; 24];
    file.read_exact(&mut footer)?;
    if &footer[8..] != MAGIC {
        return Err("Missing installer payload".into());
    }
    let length = u64::from_le_bytes(footer[..8].try_into()?);
    if !(1..=2_000_000).contains(&length) || length > total - 24 {
        return Err("Invalid payload footer".into());
    }
    let start = total - 24 - length;
    file.seek(SeekFrom::Start(start))?;
    let mut json = vec![0; length as usize];
    file.read_exact(&mut json)?;
    let manifest: Manifest = serde_json::from_slice(&json)?;
    validate(&manifest)?;
    let pin: serde_json::Value = serde_json::from_str(include_str!("../../models/manifest.json"))?;
    let model = format!(
        "models/{}",
        pin["filename"]
            .as_str()
            .ok_or("Invalid compiled model pin")?
    );
    if manifest
        .files
        .iter()
        .filter(|e| e.name.ends_with(".gguf"))
        .count()
        != 1
        || !manifest.files.iter().any(|e| {
            e.name == model
                && Some(e.bytes) == pin["bytes"].as_u64()
                && Some(e.sha256.as_str()) == pin["sha256"].as_str()
        })
        || [
            "dictado-lite.exe",
            "dictado-uninstall.exe",
            "LICENSE",
            "licenses/CC-BY-4.0.txt",
        ]
        .iter()
        .any(|name| !manifest.files.iter().any(|e| e.name == *name))
    {
        return Err("Incomplete pinned installer payload".into());
    }
    let mut end = 0;
    for entry in &manifest.files {
        if entry.offset < end
            || entry
                .offset
                .checked_add(entry.bytes)
                .is_none_or(|e| e > start)
        {
            return Err("Invalid payload extent".into());
        }
        end = entry.offset + entry.bytes;
    }
    Ok((file, manifest))
}
pub fn no_reparse(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        for ancestor in path.ancestors() {
            match fs::symlink_metadata(ancestor) {
                Ok(meta) if meta.file_attributes() & 0x400 != 0 => {
                    return Err("Reparse paths are not supported".into())
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(())
}
pub fn extract(
    file: &mut File,
    manifest: &Manifest,
    stage: &Path,
    progress: impl Fn(u64, u64) -> bool,
) -> Result<()> {
    let total: u64 = manifest.files.iter().map(|e| e.bytes).sum();
    let mut done = 0;
    let mut buffer = vec![0; 1_048_576];
    for entry in &manifest.files {
        let target = stage.join(&entry.name);
        no_reparse(&target)?;
        fs::create_dir_all(target.parent().ok_or("Invalid payload path")?)?;
        let mut output = File::options().write(true).create_new(true).open(&target)?;
        file.seek(SeekFrom::Start(entry.offset))?;
        let mut remaining = entry.bytes;
        let mut hash = Sha256::new();
        while remaining != 0 {
            let count = (remaining as usize).min(buffer.len());
            file.read_exact(&mut buffer[..count])?;
            hash.update(&buffer[..count]);
            output.write_all(&buffer[..count])?;
            remaining -= count as u64;
            done += count as u64;
            if !progress(done, total) {
                return Err("Installation cancelled".into());
            }
        }
        output.sync_all()?;
        if format!("{:x}", hash.finalize()) != entry.sha256.to_ascii_lowercase() {
            return Err(format!("Payload hash mismatch: {}", entry.name).into());
        }
    }
    Ok(())
}
/// Remove only manifest-owned files and empty directories; preserve unexpected content.
pub fn remove_files(root: &Path, manifest: &Manifest) -> Result<()> {
    validate(manifest)?;
    no_reparse(root)?;
    for entry in &manifest.files {
        let path = root.join(&entry.name);
        no_reparse(&path)?;
        if path.exists() {
            let mut attempts = 0;
            loop {
                match fs::remove_file(&path) {
                    Ok(()) => break,
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && attempts < 20 => {
                        attempts += 1;
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
    let mut dirs: Vec<_> = manifest
        .files
        .iter()
        .flat_map(|e| {
            Path::new(&e.name)
                .parent()
                .into_iter()
                .flat_map(|p| p.ancestors())
        })
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| root.join(p))
        .collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    dirs.dedup();
    for dir in dirs {
        let _ = fs::remove_dir(dir);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_escape_duplicate_and_reserved_marker() {
        for path in [
            "../Handy/x",
            "C:/x",
            "\\x",
            "models//x",
            "x/../y",
            "x.",
            "./x",
            "NUL.txt",
            "COM1.bin",
            "licenses/LPT9",
        ] {
            assert!(!relative(path));
        }
        assert!(relative("models/parakeet-Q8_0.gguf"));
        assert!(relative("libc++.dll"));
        let e = Entry {
            name: "DICTADO.EXE".into(),
            bytes: 1,
            sha256: "0".repeat(64),
            offset: 0,
        };
        let mut m = Manifest {
            product: PRODUCT.into(),
            version: "0.1.0".into(),
            files: vec![e.clone()],
        };
        assert!(validate(&m).is_ok());
        m.files.push(Entry {
            name: "dictado.exe".into(),
            ..e
        });
        assert!(validate(&m).is_err());
        m.files.truncate(1);
        m.files[0].name = MARKER.into();
        assert!(validate(&m).is_err());
        m.files[0].name = MARKER.to_ascii_uppercase();
        assert!(validate(&m).is_err());
    }
    #[test]
    fn extraction_rejects_tampering_and_cancel_and_preserves_unowned_files() {
        let root = std::env::temp_dir().join(format!(
            "dictado-payload-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("fixture");
        fs::write(&source, b"abc").unwrap();
        let m = Manifest {
            product: PRODUCT.into(),
            version: "test".into(),
            files: vec![Entry {
                name: "owned/file.txt".into(),
                bytes: 3,
                sha256: format!("{:x}", Sha256::digest(b"abc")),
                offset: 0,
            }],
        };
        let stage = root.join("stage");
        fs::create_dir(&stage).unwrap();
        let mut file = File::open(&source).unwrap();
        extract(&mut file, &m, &stage, |_, _| true).unwrap();
        assert_eq!(fs::read(stage.join("owned/file.txt")).unwrap(), b"abc");
        fs::write(stage.join("keep.txt"), b"user-owned").unwrap();
        remove_files(&stage, &m).unwrap();
        assert_eq!(fs::read(stage.join("keep.txt")).unwrap(), b"user-owned");
        fs::write(&source, b"abd").unwrap();
        let mut file = File::open(&source).unwrap();
        assert!(extract(&mut file, &m, &stage, |_, _| true).is_err());
        remove_files(&stage, &m).unwrap();
        fs::write(&source, b"abc").unwrap();
        let mut file = File::open(&source).unwrap();
        assert!(extract(&mut file, &m, &stage, |_, _| false).is_err());
        drop(file);
        remove_files(&stage, &m).unwrap();
        fs::remove_file(stage.join("keep.txt")).unwrap();
        fs::remove_dir(stage).unwrap();
        fs::remove_file(source).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
