//! User settings: theme choice + fonts + size. Registered as a gpui Global,
//! persisted as TOML in the platform config dir. Bad/missing config falls
//! back to defaults per field so we never fail to launch.

use gpui::{px, Global, Pixels};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const SIZE_BASELINE: f32 = 13.0;
const ROW_RATIO: f32 = 22.0 / 13.0;
const SIZE_MIN: f32 = 8.0;
const SIZE_MAX: f32 = 32.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme_name: String,
    pub ui_font: Option<String>,
    pub code_font: String,
    pub font_size: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme_name: "Catppuccin Mocha".into(),
            ui_font: None,
            code_font: "Menlo".into(),
            font_size: SIZE_BASELINE,
        }
    }
}

impl Global for Settings {}

impl Settings {
    pub fn config_path() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("lgtm").join("config.toml"))
    }

    pub fn from_toml_str(text: &str) -> Self {
        toml::from_str(text).unwrap_or_default()
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::from_toml_str(&text),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match toml::to_string_pretty(self) {
            Ok(text) => {
                if let Err(e) = std::fs::write(&path, text) {
                    eprintln!("lgtm: failed to save settings: {e}");
                }
            }
            Err(e) => eprintln!("lgtm: failed to serialize settings: {e}"),
        }
    }

    pub fn scale(&self) -> f32 {
        self.font_size / SIZE_BASELINE
    }

    pub fn chrome(&self, base: f32) -> Pixels {
        px(base * self.scale())
    }

    pub fn row_height(&self) -> f32 {
        self.font_size * ROW_RATIO
    }

    pub fn set_font_size(&mut self, v: f32) {
        self.font_size = v.clamp(SIZE_MIN, SIZE_MAX);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_current_appearance() {
        let s = Settings::default();
        assert_eq!(s.theme_name, "Catppuccin Mocha");
        assert_eq!(s.code_font, "Menlo");
        assert_eq!(s.ui_font, None);
        assert_eq!(s.font_size, 13.0);
    }

    #[test]
    fn toml_round_trip() {
        let s = Settings { theme_name: "Catppuccin Latte".into(), ui_font: Some("Helvetica".into()), code_font: "Monaco".into(), font_size: 16.0 };
        let text = toml::to_string(&s).unwrap();
        let back = Settings::from_toml_str(&text);
        assert_eq!(back.theme_name, "Catppuccin Latte");
        assert_eq!(back.ui_font, Some("Helvetica".into()));
        assert_eq!(back.code_font, "Monaco");
        assert_eq!(back.font_size, 16.0);
    }

    #[test]
    fn partial_or_garbage_toml_falls_back_to_defaults() {
        // Missing fields -> per-field defaults.
        let partial = Settings::from_toml_str("font_size = 20.0\n");
        assert_eq!(partial.font_size, 20.0);
        assert_eq!(partial.code_font, "Menlo");
        assert_eq!(partial.theme_name, "Catppuccin Mocha");
        // Total garbage -> full defaults.
        let garbage = Settings::from_toml_str("@@@ not toml @@@");
        assert_eq!(garbage.code_font, "Menlo");
        assert_eq!(garbage.font_size, 13.0);
    }

    #[test]
    fn scale_and_row_height_math() {
        let mut s = Settings::default();
        assert_eq!(s.scale(), 1.0);
        assert_eq!(s.row_height(), 22.0);
        s.font_size = 26.0; // 2x baseline
        assert_eq!(s.scale(), 2.0);
        assert_eq!(s.row_height(), 44.0);
    }

    #[test]
    fn font_size_clamps() {
        let mut s = Settings::default();
        s.set_font_size(200.0);
        assert_eq!(s.font_size, 32.0);
        s.set_font_size(1.0);
        assert_eq!(s.font_size, 8.0);
    }
}
