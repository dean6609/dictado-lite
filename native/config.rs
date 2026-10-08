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
}
impl Default for Config {
    fn default() -> Self {
        Self {
            microphone: None,
            key: 0x20,
            modifiers: 0x06,
            cleanup: true,
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
                if config.key == 0
                    || config.key > 255
                    || config.modifiers > 7
                    || config.key == 0x1B
                    || (config.modifiers & 6 == 0 && !(0x70..=0x87).contains(&config.key))
                {
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
