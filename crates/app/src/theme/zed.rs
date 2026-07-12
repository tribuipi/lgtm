//! Raw deserialization of Zed theme *family* JSON. These structs mirror the
//! on-disk shape 1:1 and stay deliberately lenient: every consumed role is an
//! `Option` (Zed themes legally omit or `null` any role), unknown keys are
//! ignored, and colors decode through gpui's own `Rgba` hex parser (which
//! accepts #rgb/#rgba/#rrggbb/#rrggbbaa with alpha preserved). Turning this
//! lenient shape into a guaranteed-complete `Theme` is the resolver's job.

use gpui::Rgba;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RawAppearance {
    Light,
    Dark,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawSyntaxStyle {
    #[serde(default)]
    pub color: Option<Rgba>,
    #[serde(default)]
    pub font_style: Option<String>,
    #[serde(default)]
    pub font_weight: Option<f32>,
}

#[derive(Debug, Default, Deserialize)]
pub struct RawPlayer {
    #[serde(default)]
    pub cursor: Option<Rgba>,
    #[serde(default)]
    pub selection: Option<Rgba>,
}

/// One theme's `style` object. Field names use `serde(rename)` for Zed's
/// dotted keys. Only the roles we actually consume are listed; everything else
/// in the file is ignored.
#[derive(Debug, Default, Deserialize)]
pub struct RawStyle {
    #[serde(default)]
    pub background: Option<Rgba>,
    #[serde(rename = "editor.background", default)]
    pub editor_background: Option<Rgba>,
    #[serde(rename = "editor.foreground", default)]
    pub editor_foreground: Option<Rgba>,
    #[serde(rename = "surface.background", default)]
    pub surface_background: Option<Rgba>,
    #[serde(rename = "elevated_surface.background", default)]
    pub elevated_surface_background: Option<Rgba>,
    #[serde(rename = "element.background", default)]
    pub element_background: Option<Rgba>,
    #[serde(default)]
    pub border: Option<Rgba>,
    #[serde(default)]
    pub text: Option<Rgba>,
    #[serde(rename = "text.muted", default)]
    pub text_muted: Option<Rgba>,
    #[serde(rename = "text.placeholder", default)]
    pub text_placeholder: Option<Rgba>,
    #[serde(rename = "text.accent", default)]
    pub text_accent: Option<Rgba>,
    #[serde(default)]
    pub created: Option<Rgba>,
    #[serde(default)]
    pub deleted: Option<Rgba>,
    #[serde(default)]
    pub modified: Option<Rgba>,
    #[serde(default)]
    pub error: Option<Rgba>,
    #[serde(default)]
    pub warning: Option<Rgba>,
    #[serde(default)]
    pub success: Option<Rgba>,
    #[serde(default)]
    pub info: Option<Rgba>,
    #[serde(default)]
    pub players: Vec<RawPlayer>,
    #[serde(default)]
    pub syntax: HashMap<String, RawSyntaxStyle>,
}

#[derive(Debug, Deserialize)]
pub struct ZedThemeDef {
    pub name: String,
    pub appearance: RawAppearance,
    #[serde(default)]
    pub style: RawStyle,
}

/// Decode a family, tolerating a malformed *variant* while keeping its
/// siblings. The family shell (`name` + a `themes` array) must parse or this
/// returns `Err` (whole file unusable); each variant is then decoded on its
/// own and a failing one is logged and skipped.
pub fn parse_variants(text: &str) -> anyhow::Result<Vec<ZedThemeDef>> {
    #[derive(Deserialize)]
    struct Shell {
        #[serde(default)]
        name: String,
        themes: Vec<serde_json::Value>,
    }
    let shell: Shell = serde_json::from_str(text)?;
    let mut out = Vec::new();
    for value in shell.themes {
        match serde_json::from_value::<ZedThemeDef>(value) {
            Ok(def) => out.push(def),
            Err(e) => eprintln!("lgtm: skipping malformed theme variant in {:?}: {e}", shell.name),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_hex_forms_and_null_and_missing() {
        // #rgb, #rrggbb, #rrggbbaa all decode; explicit null and missing → None.
        let style: RawStyle = serde_json::from_str(
            r##"{ "text": "#fff", "border": "#313244", "editor.background": "#1e1e2eff",
                 "text.accent": null }"##,
        )
        .unwrap();
        assert!(style.text.is_some());
        assert!(style.border.is_some());
        assert!(style.editor_background.is_some());
        assert!(style.text_accent.is_none()); // explicit null
        assert!(style.background.is_none()); // missing key
        assert!(style.players.is_empty());
        assert!(style.syntax.is_empty());
    }

    #[test]
    fn ignores_unknown_keys() {
        // Real Zed files carry dozens of keys we don't consume (vim.*, accents, …).
        let style: RawStyle =
            serde_json::from_str(r##"{ "vim.mode.text": "#11111b", "border.variant": "#abc" }"##)
                .unwrap();
        assert!(style.border.is_none());
    }

    #[test]
    fn parse_variants_skips_bad_variant_keeps_siblings() {
        // First variant is valid; second has a non-string color → skipped, sibling survives.
        let json = r##"{
            "name": "Fam",
            "themes": [
                { "name": "Good", "appearance": "dark", "style": { "text": "#fff" } },
                { "name": "Bad",  "appearance": "dark", "style": { "text": 42 } }
            ]
        }"##;
        let variants = parse_variants(json).unwrap();
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "Good");
    }

    #[test]
    fn parse_variants_errors_on_broken_shell() {
        assert!(parse_variants("not json").is_err());
    }
}
