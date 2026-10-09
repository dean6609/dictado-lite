//! Tiny independent preferences. Never reads or writes Handy's settings.
use crate::engine::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub microphone: Option<String>,
    pub key: u16,
    pub modifiers: u8,
    pub cleanup: bool,
    pub model_id: String,
    pub choose_model: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            microphone: None,
            key: 0x20,
            modifiers: 0x02,
            cleanup: true,
            model_id: "parakeet-q8".into(),
            choose_model: false,
        }
    }
}
fn path() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| Error::message("LOCALAPPDATA unavailable"))?,
    )
    .join("DictadoLite/settings.json"))
}
impl Config {
    pub fn load() -> Result<Self> {
        match std::fs::read(path()?) {
            Ok(bytes) => {
                let config: Self = serde_json::from_slice(&bytes).map_err(Error::message)?;
                if !valid_shortcut(config.key, config.modifiers) {
                    return Err(Error::message("Invalid shortcut configuration"));
                }
                Ok(config)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::message(e)),
        }
    }
    pub fn save(&self) -> Result<()> {
        let path = path()?;
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| Error::message("Invalid settings path"))?,
        )
        .map_err(Error::message)?;
        std::fs::write(
            path,
            serde_json::to_vec_pretty(self).map_err(Error::message)?,
        )
        .map_err(Error::message)
    }
}

pub fn valid_shortcut(key: u16, modifiers: u8) -> bool {
    key > 0
        && key <= 255
        && modifiers <= 7
        && !matches!(key, 0x10..=0x12 | 0x1B | 0x5B..=0x5C | 0xA0..=0xA5)
        && (modifiers & 6 != 0 || (0x70..=0x87).contains(&key))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn space_shortcuts_and_existing_settings() {
        for modifiers in [2, 3, 4, 6, 7] {
            assert!(valid_shortcut(0x20, modifiers));
        }
        for key in [0, 0x1B, 0x11, 0xA2, 0x5B, 256] {
            assert!(!valid_shortcut(key, 2));
        }
        assert!(!valid_shortcut(0x20, 0));
        assert!(valid_shortcut(0x70, 0));
        let old: Config = serde_json::from_str(r#"{"key":32,"modifiers":6}"#).unwrap();
        assert_eq!(old.modifiers, 6);
        assert_eq!(old.model_id, "parakeet-q8");
        assert_eq!(Config::default().modifiers, 2);
    }
}
