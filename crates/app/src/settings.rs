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

    /// Parses each field independently so a type error in one field (e.g. a
    /// hand-edited `font_size = "big"`) falls back to that field's default
    /// instead of discarding the whole document — `#[serde(default)]` only
    /// covers MISSING fields, not invalid ones, since a single type error
    /// fails `toml::from_str` for the entire struct.
    pub fn from_toml_str(text: &str) -> Self {
        let defaults = Self::default();
        let Ok(toml::Value::Table(table)) = text.parse::<toml::Value>() else {
            return defaults;
        };
        let field = |name: &str| table.get(name).cloned();
        let theme_name = field("theme_name")
            .and_then(|v| v.try_into::<String>().ok())
            .unwrap_or(defaults.theme_name);
        let ui_font = field("ui_font")
            .and_then(|v| v.try_into::<Option<String>>().ok())
            .unwrap_or(defaults.ui_font);
        let code_font = field("code_font")
            .and_then(|v| v.try_into::<String>().ok())
            .unwrap_or(defaults.code_font);
        let font_size = field("font_size")
            .and_then(|v| v.try_into::<f32>().ok())
            .unwrap_or(defaults.font_size);
        let mut settings = Self { theme_name, ui_font, code_font, font_size };
        // A hand-edited out-of-range size is clamped, not honored verbatim.
        settings.set_font_size(settings.font_size);
        settings
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
    fn type_error_in_one_field_preserves_other_valid_fields() {
        // font_size has a type error; the other fields are valid and must survive.
        let s = Settings::from_toml_str(
            "theme_name = \"Tokyo Night\"\ncode_font = \"Monaco\"\nfont_size = \"big\"\n",
        );
        assert_eq!(s.theme_name, "Tokyo Night");
        assert_eq!(s.code_font, "Monaco");
        assert_eq!(s.font_size, 13.0); // bad field fell back to default
    }

    #[test]
    fn font_size_clamps() {
        let mut s = Settings::default();
        s.set_font_size(200.0);
        assert_eq!(s.font_size, 32.0);
        s.set_font_size(1.0);
        assert_eq!(s.font_size, 8.0);
    }

    #[test]
    fn out_of_range_font_size_in_config_is_clamped_on_load() {
        assert_eq!(Settings::from_toml_str("font_size = 100.0\n").font_size, 32.0);
        assert_eq!(Settings::from_toml_str("font_size = 2.0\n").font_size, 8.0);
    }
}
