//! Theme discovery on disk. `theme_dirs()` enumerates where themes may live;
//! `resolve_named_in` does the boot-time targeted resolve (parse only until the
//! named variant is found). The full background scan (`discover`) is added with
//! the settings picker.

use crate::theme::embedded_mocha;
use crate::theme::model::{Appearance, Theme};
use crate::theme::{resolver, zed};
use std::path::{Path, PathBuf};

/// Directories scanned for external Zed themes, in precedence order (later
/// entries override earlier ones during discovery). The app's own themes dir
/// comes first, then Zed extension-provided themes, then Zed's hand-placed user
/// themes dir last (so a theme a user dropped into `zed/themes` wins over an
/// extension shipping the same name). Missing dirs are simply skipped by
/// callers. Duplicates are removed while preserving this precedence order.
pub fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(config) = dirs::config_dir() {
        dirs.push(config.join("lgtm").join("themes"));
    }
    // Themes shipped by installed Zed extensions live one level deeper, under
    // `<extensions/installed>/<ext>/themes`. Enumerated dynamically.
    for installed in zed_extension_installed_dirs() {
        dirs.extend(extension_theme_dirs_in(&installed));
    }
    // Zed user themes. `config_dir()/zed/themes` covers the XDG case; on macOS
    // `config_dir()` is `~/Library/Application Support` but Zed actually stores
    // user themes under `~/.config/zed/themes` there too, so add that as well.
    if let Some(config) = dirs::config_dir() {
        dirs.push(config.join("zed").join("themes"));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".config").join("zed").join("themes"));
    }
    // De-duplicate (e.g. on Linux `config_dir()` IS `~/.config`) while keeping
    // the first occurrence, so precedence order is preserved.
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|p| seen.insert(p.clone()));
    dirs
}

/// Candidate `extensions/installed` roots where Zed unpacks installed
/// extensions, across platforms. Zed uses a capitalized `Zed` data dir on
/// macOS and a lowercase `zed` dir under XDG data on Linux; probe all and let
/// callers skip the ones that don't exist.
fn zed_extension_installed_dirs() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(data) = dirs::data_dir() {
        roots.push(data.join("Zed").join("extensions").join("installed"));
        roots.push(data.join("zed").join("extensions").join("installed"));
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(
            home.join(".local")
                .join("share")
                .join("zed")
                .join("extensions")
                .join("installed"),
        );
    }
    roots
}

/// Given a Zed `extensions/installed` directory, return the `themes` subdir of
/// every installed extension that has one. Returns empty (no error) when
/// `installed` is absent or unreadable.
fn extension_theme_dirs_in(installed: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(installed) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let themes = entry.path().join("themes");
        if themes.is_dir() {
            out.push(themes);
        }
    }
    out
}

/// Scan `dirs` for a variant named `name`, parsing files lazily and returning
/// the first match resolved. Unreadable/malformed files are skipped. Returns
/// `None` if no directory holds the named variant.
pub fn resolve_named_in(dirs: &[PathBuf], name: &str) -> Option<Theme> {
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(variants) = zed::parse_variants(&text) else {
                eprintln!("lgtm: skipping malformed theme file {path:?}");
                continue;
            };
            if let Some(def) = variants.into_iter().find(|d| d.name == name) {
                return Some(resolver::resolve(
                    def.name,
                    Appearance::from(def.appearance),
                    &def.style,
                ));
            }
        }
    }
    None
}

/// A transient, name-keyed set of resolved themes for the settings picker.
/// Insertion order is preserved for display; inserting an existing name
/// overrides in place (later sources win). Lives only while Settings is open.
pub struct ThemeRegistry {
    order: Vec<String>,
    themes: std::collections::HashMap<String, Theme>,
}

impl ThemeRegistry {
    /// Start with the embedded default plus the currently-active theme, so the
    /// picker is never empty even before discovery completes.
    pub fn seeded(active: Theme) -> Self {
        let mut reg = ThemeRegistry {
            order: Vec::new(),
            themes: std::collections::HashMap::new(),
        };
        reg.insert(embedded_mocha());
        reg.insert(active);
        reg
    }

    pub fn insert(&mut self, theme: Theme) {
        if !self.themes.contains_key(&theme.name) {
            self.order.push(theme.name.clone());
        }
        self.themes.insert(theme.name.clone(), theme);
    }

    pub fn merge(&mut self, themes: Vec<Theme>) {
        for t in themes {
            self.insert(t);
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.order.clone()
    }

    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name)
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.order.len()
    }
}

/// Blocking full scan of every theme dir: parse each `*.json`, resolve every
/// variant, and return them in precedence order (app dir before zed dir, so a
/// later duplicate name overrides). Errors are contained per file/dir. Safe to
/// run on a background executor — no gpui state touched.
pub fn discover() -> Vec<Theme> {
    let mut out = Vec::new();
    for dir in theme_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let Ok(variants) = zed::parse_variants(&text) else {
                eprintln!("lgtm: skipping malformed theme file {path:?}");
                continue;
            };
            for def in variants {
                out.push(resolver::resolve(
                    def.name,
                    Appearance::from(def.appearance),
                    &def.style,
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_dirs_includes_app_and_zed_locations() {
        let dirs = theme_dirs();
        // Both the app themes dir and a zed themes dir should be represented
        // when a config dir exists; the exact paths are platform-specific, so
        // just assert the tail segments.
        let tails: Vec<String> = dirs
            .iter()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        assert!(tails.iter().any(|p| p.ends_with("lgtm/themes")));
        assert!(tails.iter().any(|p| p.contains("zed/themes")));
    }

    #[test]
    fn resolve_named_from_dir_finds_variant_and_falls_back() {
        // Write a tiny family to a temp dir; the loader should resolve the
        // named variant and, for an unknown name, fall back to embedded Mocha.
        let dir = std::env::temp_dir().join(format!("lgtm-theme-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fam.json");
        std::fs::write(
            &path,
            r##"{ "name": "Fam", "themes": [
                { "name": "My Dark", "appearance": "dark",
                  "style": { "editor.background": "#123456", "text": "#ffffff" } } ] }"##,
        )
        .unwrap();

        let found = resolve_named_in(&[dir.clone()], "My Dark").expect("variant found");
        assert_eq!(found.name, "My Dark");
        assert_eq!(found.editor_bg, gpui::rgb(0x123456));

        let missing = resolve_named_in(&[dir.clone()], "Nope");
        assert!(missing.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn registry_dedupes_by_name_later_wins() {
        let mut reg = ThemeRegistry::seeded(embedded_mocha());
        assert_eq!(reg.len(), 1);
        // Insert a second theme, then override it.
        let mut a = embedded_mocha();
        a.name = "Custom".into();
        a.editor_bg = gpui::rgb(0x111111);
        reg.insert(a);
        assert_eq!(reg.len(), 2);
        let mut b = embedded_mocha();
        b.name = "Custom".into();
        b.editor_bg = gpui::rgb(0x222222);
        reg.insert(b);
        assert_eq!(reg.len(), 2, "override must not add a row");
        assert_eq!(reg.get("Custom").unwrap().editor_bg, gpui::rgb(0x222222));
    }

    #[test]
    fn registry_names_lists_every_entry() {
        let mut reg = ThemeRegistry::seeded(embedded_mocha());
        let mut a = embedded_mocha();
        a.name = "Z".into();
        reg.insert(a);
        let names = reg.names();
        assert!(names.contains(&"Catppuccin Mocha".to_string()));
        assert!(names.contains(&"Z".to_string()));
    }

    #[test]
    fn registry_names_preserve_insertion_order_across_override() {
        // Seeded with "Catppuccin Mocha"; then A, then Z. Re-inserting Mocha
        // (an override) must NOT move it to the end — display order is stable.
        let mut reg = ThemeRegistry::seeded(embedded_mocha());
        let mut a = embedded_mocha();
        a.name = "Aaa".into();
        reg.insert(a);
        let mut z = embedded_mocha();
        z.name = "Zzz".into();
        reg.insert(z);
        reg.insert(embedded_mocha()); // override of the first entry
        assert_eq!(
            reg.names(),
            vec![
                "Catppuccin Mocha".to_string(),
                "Aaa".to_string(),
                "Zzz".to_string()
            ]
        );
    }

    #[test]
    fn extension_theme_dirs_finds_installed_theme_folders() {
        // Mimic `<installed>/<ext>/themes`: one extension ships themes, another
        // does not; a missing root yields empty without panicking.
        let root = std::env::temp_dir().join(format!("lgtm-ext-test-{}", std::process::id()));
        let ext_themes = root.join("some-theme-ext").join("themes");
        std::fs::create_dir_all(&ext_themes).unwrap();
        std::fs::create_dir_all(root.join("no-themes-ext")).unwrap();

        let found = extension_theme_dirs_in(&root);
        assert_eq!(found, vec![ext_themes]);
        assert!(extension_theme_dirs_in(&root.join("does-not-exist")).is_empty());

        std::fs::remove_dir_all(&root).ok();
    }
}
