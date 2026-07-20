use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::tidal::Quality;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub quality: Quality,
    pub volume: u8,
}

fn path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tuidal")
        .join("settings.json")
}

impl Settings {
    pub fn load() -> Option<Self> {
        let p = path();
        let json = std::fs::read_to_string(&p).ok()?;
        serde_json::from_str(&json).ok()
    }

    pub fn save(&self) {
        let p = path();
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&p, json);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_roundtrip() {
        let s = Settings {
            lang: Lang::Es,
            quality: Quality::HiResLossless,
            volume: 75,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.lang, Lang::Es);
        assert_eq!(back.quality, Quality::HiResLossless);
        assert_eq!(back.volume, 75);
    }
}
