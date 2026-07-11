# Zed-Compatible Theme System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor lgtm's theme system to Zed's semantic-role model and let the app load external Zed theme JSON files at runtime, bundling only Catppuccin Mocha as the guaranteed default.

**Architecture:** A new `crates/app/src/theme/` module tree parses Zed theme families (`serde_json` + gpui's own `Rgba` hex `Deserialize`), runs a pure deterministic *resolver* that turns possibly-incomplete Zed `style` maps into a fully-concrete role-named `Theme`, and exposes the same thread-local `ACTIVE` + bare-accessor consumer model as today — only renamed to semantic roles. Boot applies the embedded Mocha directly (zero disk) or targeted-resolves a named theme from disk; opening Settings kicks off a background filesystem scan into a transient `ThemeRegistry` that is discarded on close.

**Tech Stack:** Rust, gpui 0.2, gpui-component 0.5, serde/serde_json, the workspace `syntax` crate (`Token` enum), tree-sitter (unchanged).

## Global Constraints

- **Bundle only Catppuccin Mocha.** Every other theme comes from disk. The embedded default is real upstream Zed JSON and MUST parse+resolve successfully (guarded by a test) so boot can never fail.
- **Accessors return `gpui::Rgba`** (alpha-preserving), exactly like today, so the ~161 existing call sites keep compiling (`Rgba: Into<Hsla>`, `Rgba: Into<Fill>`).
- **Invariant (unchanged):** any write to `settings.theme_name` MUST be followed, in the same synchronous step, by `apply_ui_theme(&<resolved theme>)` — otherwise the `ACTIVE` cell diverges from render-inline colors.
- **The resolver is the single chokepoint** where "possibly incomplete Zed JSON" becomes "guaranteed-complete app `Theme`". No consumer may ever see an `Option<Color>`.
- **Parsing tolerance:** unknown JSON keys are ignored (never `deny_unknown_fields`); a bad *variant* is skipped while its siblings load; a bad *file* is skipped with a logged warning; a bad *scan directory* does not abort the others.
- **Canonical Zed-syntax-key → app `Token`:** `keyword`→Keyword, `function`→Function, `type`→Type, `string`→String, `number`→Number, `comment`→Comment, `constant`→Constant, `property`→Property, `variable`→Variable, `variable.parameter`→Parameter, `operator`→Operator, `punctuation`→Punctuation, `attribute`→Attribute, `namespace`→Namespace, `label`→Label, `embedded`→Embedded. Missing token → `text()`; missing `comment` → `text_subtle()`.
- **Accent becomes mauve.** The only upstream Catppuccin Zed file is mauve-accented (`text.accent` = `#cba6f7`); per the `accent() ← text.accent` mapping the default's accent/link/primary is now mauve, not the old blue. This is intended and accepted.

**Reference — exact resolved values of the embedded Catppuccin Mocha** (assert these in tests; all are opaque `#rrggbb` unless noted):

| Role | Value | | Role | Value |
|---|---|---|---|---|
| `background` | `#27273b` | | `text` | `#cdd6f4` |
| `editor_bg` | `#1e1e2e` | | `text_muted` | `#bac2de` |
| `surface` | `#181825` | | `text_subtle` | `#585b70` |
| `element_bg` | `#11111b` | | `accent` | `#cba6f7` |
| `border` | `#313244` | | `selection` | `#9399b240` (α≈0.25) |
| `created`/`success` | `#a6e3a1` | | `deleted`/`error` | `#f38ba8` |
| `modified`/`warning` | `#f9e2af` | | `info` | `#94e2d5` |

Syntax (`color`, `italic`): Keyword `#cba6f7` f · Function `#89b4fa` f · Type `#f9e2af` f · String `#a6e3a1` f · Number `#fab387` f · Constant `#fab387` f · Comment `#9399b2` **t** · Property `#89b4fa` f · Variable `#cdd6f4` f · Parameter `#eba0ac` f · Operator `#89dceb` f · Punctuation `#9399b2` f · Attribute `#fab387` f · Namespace `#f9e2af` **t** · Label `#74c7ec` f · Embedded `#eba0ac` f. (`t`=italic, `f`=not.)

---

## File Structure

New module tree (replaces the single `crates/app/src/theme.rs`):

- `crates/app/src/theme/mod.rs` — public surface: bare accessors (semantic roles), the `ACTIVE` thread-local, `active()`, `set_active_theme()`, `apply_ui_theme()`, `token_style()`, the diff-tint helpers, `load_active()`, and `mod` declarations. Re-exports `model::{Theme, SyntaxStyle, Appearance}`.
- `crates/app/src/theme/model.rs` — the resolved `Theme` struct (role-named `Rgba` fields + `syntax: HashMap<Token, SyntaxStyle>`), `SyntaxStyle`, `Appearance`.
- `crates/app/src/theme/zed.rs` — raw deserialization structs (`ThemeFamily`, `ZedThemeDef`, `RawStyle`, `RawPlayer`, `RawSyntaxStyle`, `RawAppearance`) + `parse_variants()` (variant-level tolerance).
- `crates/app/src/theme/resolver.rs` — `resolve()`: `(name, Appearance, &RawStyle) → Theme`, with the deterministic fallback chain and the `mix` helper.
- `crates/app/src/theme/embedded.rs` — `include_str!` of the bundled Mocha + `embedded_mocha() → Theme`.
- `crates/app/src/theme/registry.rs` — `ThemeRegistry`, `theme_dirs()`, `discover()` (blocking scan, called from a background task).
- `crates/app/themes/catppuccin-mocha.json` — the bundled asset (real upstream, Mocha variant only).

Consumers touched: `crates/app/src/main.rs` (~155 sites + boot), `crates/app/src/settings_ui.rs` (picker + preview/commit), `crates/app/src/settings.rs` (persistence fallback comment only). No changes to the `syntax`, `git`, `gh`, `diff-core`, or `claude` crates.

---

## Task 1: Module skeleton + Zed raw-parse structs

Move the existing theme file into a module directory (keeping the old hue-named `Theme` intact so the crate keeps compiling), add `serde_json`, and add the raw deserialization layer for Zed JSON. No consumer behavior changes.

**Files:**
- Move: `crates/app/src/theme.rs` → `crates/app/src/theme/mod.rs`
- Create: `crates/app/src/theme/zed.rs`
- Modify: `crates/app/Cargo.toml` (add `serde_json`)
- Test: unit tests in `crates/app/src/theme/zed.rs`

**Interfaces:**
- Produces:
  - `pub enum RawAppearance { Light, Dark }` (serde `rename_all = "lowercase"`).
  - `pub struct RawSyntaxStyle { pub color: Option<Rgba>, pub font_style: Option<String>, pub font_weight: Option<f32> }`.
  - `pub struct RawPlayer { pub cursor: Option<Rgba>, pub selection: Option<Rgba> }`.
  - `pub struct RawStyle { …one Option<Rgba> per consumed role…, pub players: Vec<RawPlayer>, pub syntax: HashMap<String, RawSyntaxStyle> }`.
  - `pub struct ZedThemeDef { pub name: String, pub appearance: RawAppearance, pub style: RawStyle }`.
  - `pub struct ThemeFamily { pub name: String, pub author: Option<String>, pub themes: Vec<ZedThemeDef> }`.
  - `pub fn parse_variants(text: &str) -> anyhow::Result<Vec<ZedThemeDef>>` — parses the family shell (errors if that fails), then decodes each variant independently, skipping+logging bad ones.

- [ ] **Step 1: Move the file into a module directory**

```bash
mkdir -p crates/app/src/theme
git mv crates/app/src/theme.rs crates/app/src/theme/mod.rs
```

- [ ] **Step 2: Add `serde_json` to the app crate**

In `crates/app/Cargo.toml`, under `[dependencies]`, add (alongside the existing `serde = { workspace = true }`):

```toml
serde_json = { workspace = true }
```

- [ ] **Step 3: Declare the new submodule from `mod.rs`**

At the top of `crates/app/src/theme/mod.rs`, right after the existing `use` block (after `use syntax::Token;`), add:

```rust
mod zed;
```

- [ ] **Step 4: Write the failing test for raw parsing**

Create `crates/app/src/theme/zed.rs` with only the tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_hex_forms_and_null_and_missing() {
        // #rgb, #rrggbb, #rrggbbaa all decode; explicit null and missing → None.
        let style: RawStyle = serde_json::from_str(
            r#"{ "text": "#fff", "border": "#313244", "editor.background": "#1e1e2eff",
                 "text.accent": null }"#,
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
            serde_json::from_str(r#"{ "vim.mode.text": "#11111b", "border.variant": "#abc" }"#)
                .unwrap();
        assert!(style.border.is_none());
    }

    #[test]
    fn parse_variants_skips_bad_variant_keeps_siblings() {
        // First variant is valid; second has a non-string color → skipped, sibling survives.
        let json = r#"{
            "name": "Fam",
            "themes": [
                { "name": "Good", "appearance": "dark", "style": { "text": "#fff" } },
                { "name": "Bad",  "appearance": "dark", "style": { "text": 42 } }
            ]
        }"#;
        let variants = parse_variants(json).unwrap();
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0].name, "Good");
    }

    #[test]
    fn parse_variants_errors_on_broken_shell() {
        assert!(parse_variants("not json").is_err());
    }
}
```

- [ ] **Step 5: Run the test to verify it fails to compile**

Run: `cargo test -p lgtm --lib theme::zed 2>&1 | tail -20`
Expected: FAIL — `RawStyle`, `parse_variants`, etc. are not defined.

- [ ] **Step 6: Implement the raw structs and `parse_variants`**

Prepend to `crates/app/src/theme/zed.rs` (above the `tests` module):

```rust
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

#[derive(Debug, Deserialize)]
pub struct ThemeFamily {
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    pub themes: Vec<ZedThemeDef>,
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
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p lgtm --lib theme::zed 2>&1 | tail -20`
Expected: PASS (4 tests). If `cargo` warns about unused fields (`author`, `font_weight`, `cursor`, `editor_foreground`), that is expected — they are consumed by the resolver in Task 2.

- [ ] **Step 8: Commit**

```bash
git add crates/app/src/theme/ crates/app/Cargo.toml Cargo.lock
git commit -m "feat(theme): add Zed theme-family raw parsing layer"
```

---

## Task 2: Resolved `Theme` model + deterministic resolver

Add the role-named resolved model and the pure resolver that fills every hole. This new `Theme` coexists with the old hue-named `Theme` in `mod.rs` (they are different types in different modules) until the Task 5 flip.

**Files:**
- Create: `crates/app/src/theme/model.rs`
- Create: `crates/app/src/theme/resolver.rs`
- Modify: `crates/app/src/theme/mod.rs` (add `mod model; mod resolver;`)
- Test: unit tests in `crates/app/src/theme/resolver.rs`

**Interfaces:**
- Consumes: `zed::{RawStyle, RawAppearance}` (Task 1).
- Produces:
  - `model::Appearance { Light, Dark }` with `pub fn mode(self) -> gpui_component::ThemeMode` and `impl From<zed::RawAppearance>`.
  - `model::SyntaxStyle { pub color: Rgba, pub italic: bool }` (Copy).
  - `model::Theme { pub name: String, pub appearance: Appearance, pub background/editor_bg/surface/element_bg/border/text/text_muted/text_subtle/accent: Rgba, pub created/deleted/modified/error/warning/success/info/selection: Rgba, pub syntax: HashMap<Token, SyntaxStyle> }` with `pub fn mode(&self) -> ThemeMode` and `pub fn syntax(&self, Token) -> SyntaxStyle`.
  - `resolver::resolve(name: impl Into<String>, appearance: Appearance, style: &RawStyle) -> Theme`.

- [ ] **Step 1: Write the model**

Create `crates/app/src/theme/model.rs`:

```rust
//! The resolved, role-named theme every consumer sees. Produced only by the
//! resolver, so every field is concrete — no consumer ever handles an
//! `Option<Color>`. Field names are Zed's semantic roles, not hues.

use gpui::Rgba;
use gpui_component::ThemeMode;
use std::collections::HashMap;
use syntax::Token;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub fn mode(self) -> ThemeMode {
        match self {
            Appearance::Light => ThemeMode::Light,
            Appearance::Dark => ThemeMode::Dark,
        }
    }
}

impl From<super::zed::RawAppearance> for Appearance {
    fn from(a: super::zed::RawAppearance) -> Self {
        match a {
            super::zed::RawAppearance::Light => Appearance::Light,
            super::zed::RawAppearance::Dark => Appearance::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SyntaxStyle {
    pub color: Rgba,
    pub italic: bool,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub appearance: Appearance,
    // Chrome roles.
    pub background: Rgba,
    pub editor_bg: Rgba,
    pub surface: Rgba,
    pub element_bg: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_subtle: Rgba,
    pub accent: Rgba,
    // Status roles.
    pub created: Rgba,
    pub deleted: Rgba,
    pub modified: Rgba,
    pub error: Rgba,
    pub warning: Rgba,
    pub success: Rgba,
    pub info: Rgba,
    // Diff-pane text selection (Zed players[0].selection, or accent@opacity).
    pub selection: Rgba,
    // Fully populated for all 16 Tokens by the resolver.
    pub syntax: HashMap<Token, SyntaxStyle>,
}

impl Theme {
    pub fn mode(&self) -> ThemeMode {
        self.appearance.mode()
    }

    /// Belt-and-braces: the resolver fills all 16 tokens, but fall back to the
    /// plain text color if a token is ever missing.
    pub fn syntax(&self, token: Token) -> SyntaxStyle {
        self.syntax
            .get(&token)
            .copied()
            .unwrap_or(SyntaxStyle { color: self.text, italic: false })
    }
}
```

- [ ] **Step 2: Declare the modules**

In `crates/app/src/theme/mod.rs`, extend the module declarations added in Task 1 to:

```rust
mod model;
mod resolver;
mod zed;
```

- [ ] **Step 3: Write the failing resolver tests**

Create `crates/app/src/theme/resolver.rs` with the tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::model::Appearance;
    use crate::theme::zed::RawStyle;
    use gpui::rgb;
    use syntax::Token;

    fn style(json: &str) -> RawStyle {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn concrete_roles_pass_through() {
        let s = style(
            r#"{ "background": "#101010", "editor.background": "#202020",
                 "text": "#f0f0f0", "text.accent": "#3366ff", "border": "#303030" }"#,
        );
        let t = resolve("X", Appearance::Dark, &s);
        assert_eq!(t.background, rgb(0x101010));
        assert_eq!(t.editor_bg, rgb(0x202020));
        assert_eq!(t.text, rgb(0xf0f0f0));
        assert_eq!(t.accent, rgb(0x3366ff));
        assert_eq!(t.border, rgb(0x303030));
    }

    #[test]
    fn background_and_editor_bg_fall_back_to_each_other() {
        let only_editor = resolve("X", Appearance::Dark, &style(r#"{ "editor.background": "#123456" }"#));
        assert_eq!(only_editor.background, rgb(0x123456));
        let only_bg = resolve("X", Appearance::Dark, &style(r#"{ "background": "#654321" }"#));
        assert_eq!(only_bg.editor_bg, rgb(0x654321));
    }

    #[test]
    fn text_from_editor_foreground_and_muted_tiers_derived() {
        let t = resolve("X", Appearance::Dark, &style(r#"{ "editor.foreground": "#ffffff", "editor.background": "#000000" }"#));
        assert_eq!(t.text, rgb(0xffffff));
        // Muted tiers are between text and background, and distinct from each other.
        assert_ne!(t.text_muted, t.text);
        assert_ne!(t.text_subtle, t.text);
        assert_ne!(t.text_muted, t.text_subtle);
    }

    #[test]
    fn status_roles_cross_fill_then_default() {
        // created present, success absent → success mirrors created.
        let t = resolve("X", Appearance::Dark, &style(r#"{ "created": "#00ff00" }"#));
        assert_eq!(t.success, rgb(0x00ff00));
        // Nothing present → appearance default (non-zero, opaque).
        let empty = resolve("X", Appearance::Dark, &style("{}"));
        assert_eq!(empty.created.a, 1.0);
        assert_ne!(empty.error, empty.created);
    }

    #[test]
    fn accent_falls_back_to_first_player_cursor() {
        let t = resolve("X", Appearance::Dark, &style(r#"{ "players": [ { "cursor": "#abcdef" } ] }"#));
        assert_eq!(t.accent, rgb(0xabcdef));
    }

    #[test]
    fn every_token_resolves_and_comment_defaults_to_subtle() {
        // No syntax map at all → every token filled; comment uses text_subtle.
        let t = resolve("X", Appearance::Dark, &style(r#"{ "text": "#eeeeee" }"#));
        for token in ALL_TOKENS {
            let s = t.syntax(token);
            assert_eq!(s.color.a, 1.0, "{token:?} unresolved");
        }
        assert_eq!(t.syntax(Token::Variable).color, rgb(0xeeeeee));
        assert_eq!(t.syntax(Token::Comment).color, t.text_subtle);
    }

    #[test]
    fn syntax_italic_from_font_style() {
        let t = resolve(
            "X",
            Appearance::Dark,
            &style(r#"{ "syntax": { "comment": { "color": "#777777", "font_style": "italic" },
                                    "keyword": { "color": "#ff00ff" } } }"#),
        );
        assert!(t.syntax(Token::Comment).italic);
        assert_eq!(t.syntax(Token::Comment).color, rgb(0x777777));
        assert!(!t.syntax(Token::Keyword).italic);
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test -p lgtm --lib theme::resolver 2>&1 | tail -20`
Expected: FAIL — `resolve`, `ALL_TOKENS` not defined.

- [ ] **Step 5: Implement the resolver**

Prepend to `crates/app/src/theme/resolver.rs` (above `tests`):

```rust
//! The single chokepoint that turns a lenient `RawStyle` (any role may be
//! absent) plus an `Appearance` into a fully-concrete `Theme`. Pure and
//! deterministic: same input → same output, no I/O, no globals. Every fallback
//! is documented inline; the anchor roles (background/editor_bg/text) bottom
//! out at appearance-keyed constants so no field is ever left unset.

use crate::theme::model::{Appearance, SyntaxStyle, Theme};
use crate::theme::zed::RawStyle;
use gpui::Rgba;
use std::collections::HashMap;
use syntax::Token;

/// All 16 semantic tokens, each paired with the canonical Zed syntax key it
/// reads. See the Global Constraints table.
const TOKEN_KEYS: &[(Token, &str)] = &[
    (Token::Keyword, "keyword"),
    (Token::Function, "function"),
    (Token::Type, "type"),
    (Token::String, "string"),
    (Token::Number, "number"),
    (Token::Comment, "comment"),
    (Token::Constant, "constant"),
    (Token::Property, "property"),
    (Token::Variable, "variable"),
    (Token::Parameter, "variable.parameter"),
    (Token::Operator, "operator"),
    (Token::Punctuation, "punctuation"),
    (Token::Attribute, "attribute"),
    (Token::Namespace, "namespace"),
    (Token::Label, "label"),
    (Token::Embedded, "embedded"),
];

#[cfg(test)]
pub(crate) const ALL_TOKENS: [Token; 16] = [
    Token::Keyword, Token::Function, Token::Type, Token::String, Token::Number,
    Token::Comment, Token::Constant, Token::Property, Token::Variable, Token::Parameter,
    Token::Operator, Token::Punctuation, Token::Attribute, Token::Namespace, Token::Label,
    Token::Embedded,
];

/// Component-wise linear interpolation from `a` (t=0) to `b` (t=1); alpha
/// tracks `a`. Used to derive muted/border tiers when a theme omits them.
fn mix(a: Rgba, b: Rgba, t: f32) -> Rgba {
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a,
    }
}

fn with_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a, ..c }
}

/// Appearance-keyed anchors for themes that omit even the basics.
struct Defaults {
    editor_bg: Rgba,
    text: Rgba,
    accent: Rgba,
    created: Rgba,
    deleted: Rgba,
    modified: Rgba,
}

fn defaults(appearance: Appearance) -> Defaults {
    match appearance {
        Appearance::Dark => Defaults {
            editor_bg: gpui::rgb(0x1a1b26),
            text: gpui::rgb(0xc0caf5),
            accent: gpui::rgb(0x7aa2f7),
            created: gpui::rgb(0x3fb950),
            deleted: gpui::rgb(0xf85149),
            modified: gpui::rgb(0xd29922),
        },
        Appearance::Light => Defaults {
            editor_bg: gpui::rgb(0xffffff),
            text: gpui::rgb(0x24292f),
            accent: gpui::rgb(0x0969da),
            created: gpui::rgb(0x1a7f37),
            deleted: gpui::rgb(0xcf222e),
            modified: gpui::rgb(0x9a6700),
        },
    }
}

pub fn resolve(name: impl Into<String>, appearance: Appearance, style: &RawStyle) -> Theme {
    let d = defaults(appearance);

    // Anchors first — everything else can lean on these.
    let editor_bg = style.editor_background.or(style.background).unwrap_or(d.editor_bg);
    let background = style.background.unwrap_or(editor_bg);
    let text = style.text.or(style.editor_foreground).unwrap_or(d.text);

    let surface = style
        .surface_background
        .or(style.elevated_surface_background)
        .unwrap_or(editor_bg);
    let element_bg = style.element_background.unwrap_or_else(|| mix(surface, text, 0.05));
    let border = style.border.unwrap_or_else(|| mix(surface, text, 0.15));

    // Muted tiers drift text toward background; keep the two tiers distinct.
    let text_muted = style.text_muted.unwrap_or_else(|| mix(text, background, 0.35));
    let text_subtle = style
        .text_placeholder
        .unwrap_or_else(|| mix(text, background, 0.55));

    // Accent: explicit → first player cursor → appearance default.
    let accent = style
        .text_accent
        .or_else(|| style.players.first().and_then(|p| p.cursor))
        .unwrap_or(d.accent);

    // Status roles cross-fill (created↔success, deleted↔error, modified↔warning)
    // then bottom out at appearance defaults.
    let created = style.created.or(style.success).unwrap_or(d.created);
    let success = style.success.or(style.created).unwrap_or(d.created);
    let deleted = style.deleted.or(style.error).unwrap_or(d.deleted);
    let error = style.error.or(style.deleted).unwrap_or(d.deleted);
    let modified = style.modified.or(style.warning).unwrap_or(d.modified);
    let warning = style.warning.or(style.modified).unwrap_or(d.modified);
    let info = style.info.unwrap_or(accent);

    let selection = style
        .players
        .first()
        .and_then(|p| p.selection)
        .unwrap_or_else(|| with_alpha(accent, 0.3));

    // Syntax: read the canonical key per token; missing → text (comment → subtle).
    let mut syntax = HashMap::with_capacity(TOKEN_KEYS.len());
    for &(token, key) in TOKEN_KEYS {
        let raw = style.syntax.get(key);
        let color = raw.and_then(|s| s.color).unwrap_or_else(|| {
            if token == Token::Comment {
                text_subtle
            } else {
                text
            }
        });
        let italic = raw
            .and_then(|s| s.font_style.as_deref())
            .map(|fs| fs.eq_ignore_ascii_case("italic"))
            .unwrap_or(false);
        syntax.insert(token, SyntaxStyle { color, italic });
    }

    Theme {
        name: name.into(),
        appearance,
        background,
        editor_bg,
        surface,
        element_bg,
        border,
        text,
        text_muted,
        text_subtle,
        accent,
        created,
        deleted,
        modified,
        error,
        warning,
        success,
        info,
        selection,
        syntax,
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p lgtm --lib theme::resolver 2>&1 | tail -20`
Expected: PASS (7 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/app/src/theme/model.rs crates/app/src/theme/resolver.rs crates/app/src/theme/mod.rs
git commit -m "feat(theme): add resolved role model and deterministic resolver"
```

---

## Task 3: Bundle Catppuccin Mocha + embedded loader

Vendor the real upstream Catppuccin Mocha (Mocha variant only) and expose `embedded_mocha()`, with a test that parses+resolves it and asserts the reference values — making a broken default impossible.

**Files:**
- Create: `crates/app/themes/catppuccin-mocha.json`
- Create: `crates/app/src/theme/embedded.rs`
- Modify: `crates/app/src/theme/mod.rs` (add `mod embedded;`)
- Test: unit tests in `crates/app/src/theme/embedded.rs`

**Interfaces:**
- Consumes: `zed::parse_variants`, `resolver::resolve`, `model::{Theme, Appearance}`.
- Produces: `pub fn embedded_mocha() -> model::Theme`.

- [ ] **Step 1: Vendor the bundled asset**

Reproduce the real upstream file, keeping only the Mocha variant (deterministic; requires network + `gh` once). Run from the repo root:

```bash
mkdir -p crates/app/themes
gh api repos/catppuccin/zed/contents/themes/catppuccin-mauve.json --jq '.content' \
  | base64 -d \
  | python3 -c "import json,sys; d=json.load(sys.stdin); m=[t for t in d['themes'] if t['name']=='Catppuccin Mocha'][0]; json.dump({'\$schema':d['\$schema'],'name':'Catppuccin Mocha','author':d['author'],'themes':[m]}, open('crates/app/themes/catppuccin-mocha.json','w'), indent=2, ensure_ascii=False)"
```

Verify: the file exists, is ~23 KB, and its `themes[0]` has `"name": "Catppuccin Mocha"`, `"appearance": "dark"`, `style.background = "#27273b"`, `style.text.accent = "#cba6f7"`, and a `syntax` object.

```bash
python3 -c "import json; d=json.load(open('crates/app/themes/catppuccin-mocha.json')); t=d['themes'][0]; print(t['name'], t['appearance'], t['style']['background'], t['style']['text.accent'], 'syntax' in t['style'])"
```
Expected: `Catppuccin Mocha dark #27273b #cba6f7 True`

- [ ] **Step 2: Write the failing embedded test**

Create `crates/app/src/theme/embedded.rs` with tests only:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::model::Appearance;
    use gpui::{rgb, rgba};
    use syntax::Token;

    #[test]
    fn embedded_mocha_parses_and_resolves_key_roles() {
        let t = embedded_mocha();
        assert_eq!(t.name, "Catppuccin Mocha");
        assert_eq!(t.appearance, Appearance::Dark);
        assert_eq!(t.background, rgb(0x27273b));
        assert_eq!(t.editor_bg, rgb(0x1e1e2e));
        assert_eq!(t.surface, rgb(0x181825));
        assert_eq!(t.element_bg, rgb(0x11111b));
        assert_eq!(t.border, rgb(0x313244));
        assert_eq!(t.text, rgb(0xcdd6f4));
        assert_eq!(t.text_muted, rgb(0xbac2de));
        assert_eq!(t.text_subtle, rgb(0x585b70));
        assert_eq!(t.accent, rgb(0xcba6f7));
        assert_eq!(t.created, rgb(0xa6e3a1));
        assert_eq!(t.deleted, rgb(0xf38ba8));
        assert_eq!(t.modified, rgb(0xf9e2af));
        assert_eq!(t.error, rgb(0xf38ba8));
        assert_eq!(t.warning, rgb(0xf9e2af));
        assert_eq!(t.success, rgb(0xa6e3a1));
        assert_eq!(t.info, rgb(0x94e2d5));
        assert_eq!(t.selection, rgba(0x9399b240));
    }

    #[test]
    fn embedded_mocha_syntax_matches_upstream() {
        let t = embedded_mocha();
        assert_eq!(t.syntax(Token::Keyword).color, rgb(0xcba6f7));
        assert!(!t.syntax(Token::Keyword).italic);
        assert_eq!(t.syntax(Token::Function).color, rgb(0x89b4fa));
        assert_eq!(t.syntax(Token::Type).color, rgb(0xf9e2af));
        assert_eq!(t.syntax(Token::String).color, rgb(0xa6e3a1));
        assert_eq!(t.syntax(Token::Number).color, rgb(0xfab387));
        assert_eq!(t.syntax(Token::Comment).color, rgb(0x9399b2));
        assert!(t.syntax(Token::Comment).italic);
        assert_eq!(t.syntax(Token::Property).color, rgb(0x89b4fa));
        assert_eq!(t.syntax(Token::Parameter).color, rgb(0xeba0ac));
        assert_eq!(t.syntax(Token::Operator).color, rgb(0x89dceb));
        assert_eq!(t.syntax(Token::Punctuation).color, rgb(0x9399b2));
        assert_eq!(t.syntax(Token::Namespace).color, rgb(0xf9e2af));
        assert!(t.syntax(Token::Namespace).italic);
        assert_eq!(t.syntax(Token::Label).color, rgb(0x74c7ec));
        assert_eq!(t.syntax(Token::Attribute).color, rgb(0xfab387));
        assert_eq!(t.syntax(Token::Variable).color, rgb(0xcdd6f4));
        assert_eq!(t.syntax(Token::Embedded).color, rgb(0xeba0ac));
        assert_eq!(t.syntax(Token::Constant).color, rgb(0xfab387));
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test -p lgtm --lib theme::embedded 2>&1 | tail -20`
Expected: FAIL — `embedded_mocha` not defined.

- [ ] **Step 4: Implement the embedded loader**

Prepend to `crates/app/src/theme/embedded.rs`:

```rust
//! The one bundled theme. Compiled into the binary via `include_str!` and
//! parsed+resolved through the exact same path as on-disk themes (dogfooding
//! the parser). The accompanying test asserts it resolves, so shipping a
//! broken default is impossible.

use crate::theme::model::{Appearance, Theme};
use crate::theme::{resolver, zed};

const MOCHA_JSON: &str = include_str!("../../themes/catppuccin-mocha.json");

/// Parse + resolve the bundled Catppuccin Mocha. Panics only if the vendored
/// asset is malformed — which the unit test in this module prevents from ever
/// reaching a release.
pub fn embedded_mocha() -> Theme {
    let variants = zed::parse_variants(MOCHA_JSON).expect("bundled Mocha JSON must parse");
    let def = variants
        .into_iter()
        .find(|d| d.name == "Catppuccin Mocha")
        .expect("bundled JSON must contain the Catppuccin Mocha variant");
    resolver::resolve(def.name, Appearance::from(def.appearance), &def.style)
}
```

- [ ] **Step 5: Declare the module**

In `crates/app/src/theme/mod.rs`, update the module declarations to include `embedded`:

```rust
mod embedded;
mod model;
mod resolver;
mod zed;
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p lgtm --lib theme::embedded 2>&1 | tail -20`
Expected: PASS (2 tests).

- [ ] **Step 7: Commit**

```bash
git add crates/app/themes/catppuccin-mocha.json crates/app/src/theme/embedded.rs crates/app/src/theme/mod.rs
git commit -m "feat(theme): bundle upstream Catppuccin Mocha and embed loader"
```

---

## Task 4: Consumer flip — semantic accessors, apply_ui_theme, token_style, tints, all sites

Replace the entire consumer surface in one atomic switch: the new role accessors read a new `ACTIVE: RefCell<model::Theme>`, `apply_ui_theme`/`token_style`/tints are rewritten against roles, all ~161 call sites are repointed, boot + the picker use the new loader, and the old hue struct/constructors/`by_name`/`all_names` are deleted. Steps are ordered so the crate compiles after each one (new accessors are added *alongside* the old, sites migrated in groups, old surface deleted last).

This is the large task. Work through the steps in order; do not skip the intermediate `cargo build` checks.

**Files:**
- Modify: `crates/app/src/theme/mod.rs` (rewrite accessors/apply/token_style/tints; delete old surface)
- Modify: `crates/app/src/main.rs` (~155 sites + boot + render-site `by_name`)
- Modify: `crates/app/src/settings_ui.rs` (picker + preview/commit interim)
- Test: existing `theme` tests rewritten; existing `main.rs` diff/highlight tests updated

**Interfaces:**
- Consumes: `embedded::embedded_mocha`, `model::{Theme, SyntaxStyle}`, `resolver`.
- Produces (new public accessors on `crate::theme`, all `-> Rgba` unless noted):
  - Chrome: `background()`, `editor_bg()`, `surface()`, `element_bg()`, `border()`, `text()`, `text_muted()`, `text_subtle()`, `accent()`.
  - Status: `created()`, `deleted()`, `modified()`, `error()`, `warning()`, `success()`, `info()`.
  - `active() -> Theme`, `set_active_theme(&Theme)`, `apply_ui_theme(&Theme, &mut App)`, `token_style(&Theme, Token) -> HighlightStyle`.
  - Tints (all `(&Theme) -> Rgba`): `added_row_bg`, `removed_row_bg`, `added_word_bg`, `removed_word_bg`, `selection_bg`, `void_cell_bg`, `palette_backdrop`.
  - `load_active(name: &str) -> Theme` (interim: returns `embedded_mocha()`; extended in Task 5).

### Accessor migration map

**Mechanical 1:1 global renames** (apply across `main.rs` and `settings_ui.rs`):

| Old | New | Notes |
|---|---|---|
| `theme::crust()` | `theme::background()` | 4 sites |
| `theme::base()` | `theme::editor_bg()` | 1 site |
| `theme::mantle()` | `theme::surface()` | 8 main + 1 settings |
| `theme::subtext()` | `theme::text_muted()` | 8 main + 2 settings |
| `theme::overlay0()` | `theme::text_subtle()` | 43 main + 2 settings |
| `theme::text()` | `theme::text()` | unchanged (no edit) |

**`theme::surface0()` — split by role** (from the site inventory):

- → `theme::border()` (strokes/dividers): `main.rs` lines 1489, 4727, 4754, 4869, 5043, 5126, 5173, 5252, 5586, 5605, 5796, 5817, 5846; `settings_ui.rs` line 355.
- → `theme::element_bg()` (fills/hovers): `main.rs` lines 1275, 1936, 1938, 2763, 2765, 4796, 5128, 5428, 5430, 5664, 5667; `settings_ui.rs` lines 261, 265.

**`theme::green()` — split** (from inventory):

- → `theme::created()`: `main.rs` 1049, 1343, 1880, 1917, 2597, 2665, 5294, 5397, and test 6863.
- → `theme::success()`: `main.rs` 2177, 2529, 2537, 2751, 5191; `settings_ui.rs` 280.

**`theme::red()` — split** (from inventory):

- → `theme::deleted()`: `main.rs` 1055, 1348, 1881, 1922, 2602, 2670, 5295, 5402, and test 6864.
- → `theme::error()`: `main.rs` 2179, 2182, 2531, 2538, 4821, 5059, 5195, 5201, 5415, 5599, 5684, 5707, 5883.

**`theme::blue()` — split** (from inventory):

- → `theme::accent()`: `main.rs` 1161, 1238, 1363, 2174, 2628, 4877, 5197, 5299.
- → `theme::modified()`: `main.rs` 1882 (file-status "modified"), and test 6867.

**`theme::mauve()` → `theme::accent()`:** `main.rs` 1883 (status "renamed"), 2178 (MERGED dot), 2530 (merged badge), and test 6871. (Accent is now mauve, so "renamed"/"merged" stay purple.)

**`theme::peach()` → `theme::warning()`:** `main.rs` 1884 (status "binary"), and test 6873.

> Line numbers are the pre-edit positions from the inventory; they drift as you edit. Migrate top-to-bottom, or re-grep each old accessor name after a group to catch stragglers. The final Step 12 greps for *zero* remaining old-name call sites.

- [ ] **Step 1: Rewrite `mod.rs` header and `ACTIVE` to the resolved model**

In `crates/app/src/theme/mod.rs`, replace the module doc comment, imports, `ACTIVE`, `set_active_theme`, and `active()` (the block from the top of the file through the current `fn active()`), keeping the `mod` declarations from Tasks 1–3, with:

```rust
//! Data-driven themes in Zed's semantic-role vocabulary. A resolved `Theme`
//! (see `model`) is a flat struct of role-named colors plus a per-`Token`
//! syntax map; the resolver guarantees every field is concrete. The bare
//! `background()`/`text()`/`accent()`/… accessors read a thread-local "active
//! theme" that `apply_ui_theme` installs, so every decorative UI color follows
//! the applied theme. Before the first apply on a thread the cell defaults to
//! the embedded Catppuccin Mocha.

use gpui::{App, FontStyle, HighlightStyle, Hsla, Rgba};
use gpui_component::Theme as UiTheme;
use std::cell::RefCell;
use syntax::Token;

pub use embedded::embedded_mocha;
pub use model::{Appearance, SyntaxStyle, Theme};

mod embedded;
mod model;
// mod registry; — DEFERRED: registry.rs does not exist until Task 5. Leave this
// commented out now; uncomment it in Task 5 Step 4 when the file is created.
mod resolver;
mod zed;

// Invariant: any change to `settings.theme_name` MUST be followed by
// `apply_ui_theme(&<resolved theme>)` in the same synchronous step, or these
// bare accessors (backed by `ACTIVE`) diverge from the syntax/tint colors that
// render code derives inline via `active()`.
thread_local! {
    static ACTIVE: RefCell<Theme> = RefCell::new(embedded_mocha());
}

/// Install `theme` as the palette the bare accessors return. Called by
/// `apply_ui_theme`; exposed for tests that have no `App`.
pub(crate) fn set_active_theme(theme: &Theme) {
    ACTIVE.with(|a| *a.borrow_mut() = theme.clone());
}

/// A clone of the palette currently backing the bare accessors. Render code
/// that needs the whole theme (diff tints, `token_style`) calls this instead
/// of re-resolving.
pub fn active() -> Theme {
    ACTIVE.with(|a| a.borrow().clone())
}
```

> If you prefer not to touch `registry` before Task 6, omit the `mod registry;` line here and add it in Task 6 Step 1.

- [ ] **Step 2: Replace the old accessors with role accessors (add new; old removed here)**

In `mod.rs`, replace the entire old accessor block (`pub fn base()` … through `pub fn peach()`) with:

```rust
pub fn background() -> Rgba {
    active().background
}
pub fn editor_bg() -> Rgba {
    active().editor_bg
}
pub fn surface() -> Rgba {
    active().surface
}
pub fn element_bg() -> Rgba {
    active().element_bg
}
pub fn border() -> Rgba {
    active().border
}
pub fn text() -> Rgba {
    active().text
}
pub fn text_muted() -> Rgba {
    active().text_muted
}
pub fn text_subtle() -> Rgba {
    active().text_subtle
}
pub fn accent() -> Rgba {
    active().accent
}
pub fn created() -> Rgba {
    active().created
}
pub fn deleted() -> Rgba {
    active().deleted
}
pub fn modified() -> Rgba {
    active().modified
}
pub fn error() -> Rgba {
    active().error
}
pub fn warning() -> Rgba {
    active().warning
}
pub fn success() -> Rgba {
    active().success
}
pub fn info() -> Rgba {
    active().info
}
```

- [ ] **Step 3: Delete the old hue `Theme`, all Rust constructors, `all_names`, `by_name`**

In `mod.rs`, delete: the `pub struct Theme { … }` (the hue one, ~line 89–110 of the original), every `pub fn catppuccin_*/github_*/tokyo_*/solarized_*() -> Theme` constructor, `pub fn all_names()`, and `pub fn by_name()`. (The resolved `Theme` now comes from `pub use model::Theme`.) The crate will not compile again until Steps 4–11 repoint consumers; that is expected within this task.

- [ ] **Step 4: Rewrite `apply_ui_theme` against roles**

Replace the old `apply_ui_theme` body in `mod.rs` with:

```rust
/// Override gpui-component's widget theme with the given resolved `Theme`.
/// Call after `gpui_component::init`.
pub fn apply_ui_theme(theme: &Theme, cx: &mut App) {
    set_active_theme(theme);
    UiTheme::change(theme.mode(), None, cx);

    let editor_bg: Hsla = theme.editor_bg.into();
    let surface: Hsla = theme.surface.into();
    let element_bg: Hsla = theme.element_bg.into();
    let border: Hsla = theme.border.into();
    let text: Hsla = theme.text.into();
    let text_subtle: Hsla = theme.text_subtle.into();
    let accent: Hsla = theme.accent.into();
    let background: Hsla = theme.background.into();
    let error: Hsla = theme.error.into();
    let success: Hsla = theme.success.into();
    let warning: Hsla = theme.warning.into();
    let info: Hsla = theme.info.into();

    let t = UiTheme::global_mut(cx);
    t.background = editor_bg;
    t.foreground = text;
    t.muted = element_bg;
    t.muted_foreground = text_subtle;
    t.border = border;
    t.input = border;
    t.ring = accent;
    t.primary = accent;
    t.primary_hover = accent.opacity(0.9);
    t.primary_active = accent.opacity(0.8);
    t.primary_foreground = background;
    t.secondary = element_bg;
    t.secondary_hover = element_bg.opacity(0.8);
    t.secondary_active = element_bg.opacity(0.6);
    t.secondary_foreground = text;
    t.accent = element_bg;
    t.accent_foreground = text;
    t.danger = error;
    t.danger_hover = error.opacity(0.9);
    t.danger_active = error.opacity(0.8);
    t.danger_foreground = background;
    t.success = success;
    t.success_hover = success.opacity(0.9);
    t.success_active = success.opacity(0.8);
    t.success_foreground = background;
    t.warning = warning;
    t.warning_hover = warning.opacity(0.9);
    t.warning_active = warning.opacity(0.8);
    t.warning_foreground = background;
    t.info = info;
    t.info_hover = info.opacity(0.9);
    t.info_active = info.opacity(0.8);
    t.info_foreground = background;
    t.link = accent;
    t.link_hover = accent.opacity(0.9);
    t.link_active = accent.opacity(0.8);
    t.popover = surface;
    t.popover_foreground = text;
    t.title_bar = surface;
    t.title_bar_border = border;
    t.sidebar = surface;
    t.sidebar_foreground = text;
    t.sidebar_border = border;
    t.caret = text;
    t.selection = theme.selection.into();
    t.scrollbar = background.opacity(0.6);
    t.scrollbar_thumb = text_subtle.opacity(0.5);
    t.scrollbar_thumb_hover = text_subtle;
    t.window_border = border;
}
```

- [ ] **Step 5: Rewrite `token_style` against the syntax map**

Replace the old `token_style` in `mod.rs` with:

```rust
/// Syntax highlight for one token, read from the resolved theme's syntax map.
pub fn token_style(theme: &Theme, token: Token) -> HighlightStyle {
    let s = theme.syntax(token);
    HighlightStyle {
        color: Some(s.color.into()),
        font_style: s.italic.then_some(FontStyle::Italic),
        ..Default::default()
    }
}
```

- [ ] **Step 6: Rewrite the diff-tint helpers against roles**

Replace the old tint helpers (`palette_backdrop` through `removed_word_bg`) in `mod.rs` with:

```rust
fn with_alpha(c: Rgba, a: f32) -> Rgba {
    Rgba { a, ..c }
}

/// Dimming layer behind the command palette / modals.
pub fn palette_backdrop(theme: &Theme) -> Rgba {
    with_alpha(theme.background, 0.667)
}

/// Split view: the absent side of a one-sided row — darker than any content row.
pub fn void_cell_bg(theme: &Theme) -> Rgba {
    with_alpha(theme.background, 0.6)
}

/// Text selection in the diff pane — matches `selection` injected into the
/// gpui-component theme in `apply_ui_theme`.
pub fn selection_bg(theme: &Theme) -> Rgba {
    theme.selection
}

/// Low-alpha tints: syntax/text must stay readable on top. Never opaque.
pub fn added_row_bg(theme: &Theme) -> Rgba {
    with_alpha(theme.created, 0.125)
}
pub fn removed_row_bg(theme: &Theme) -> Rgba {
    with_alpha(theme.deleted, 0.125)
}
pub fn added_word_bg(theme: &Theme) -> Rgba {
    with_alpha(theme.created, 0.282)
}
pub fn removed_word_bg(theme: &Theme) -> Rgba {
    with_alpha(theme.deleted, 0.282)
}
```

- [ ] **Step 7: Add the interim `load_active`**

Append to `mod.rs`:

```rust
/// Resolve a persisted theme name to a concrete `Theme` for boot/preview. The
/// embedded default needs zero disk access; any other name currently falls
/// back to the embedded default (disk targeted-resolve is added in a later
/// task). Never fails.
pub fn load_active(name: &str) -> Theme {
    if name == "Catppuccin Mocha" {
        return embedded_mocha();
    }
    embedded_mocha()
}
```

- [ ] **Step 8: Rewrite the `theme` module's own tests**

Replace the entire `#[cfg(test)] mod tests { … }` at the bottom of `mod.rs` with:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use gpui::rgb;
    use syntax::Token;

    #[test]
    fn accessors_default_to_embedded_mocha_without_apply() {
        // No set_active_theme on this thread → embedded Mocha default.
        assert_eq!(editor_bg(), rgb(0x1e1e2e));
        assert_eq!(text(), rgb(0xcdd6f4));
        assert_eq!(accent(), rgb(0xcba6f7));
        assert_eq!(created(), rgb(0xa6e3a1));
        assert_eq!(deleted(), rgb(0xf38ba8));
    }

    #[test]
    fn accessors_follow_active_theme() {
        set_active_theme(&embedded_mocha());
        assert_eq!(background(), rgb(0x27273b));
        assert_eq!(border(), rgb(0x313244));
    }

    #[test]
    fn token_style_reads_syntax_map() {
        let t = embedded_mocha();
        let kw = token_style(&t, Token::Keyword);
        assert_eq!(kw.color, Some(rgb(0xcba6f7).into()));
        assert_eq!(kw.font_style, None);
        let comment = token_style(&t, Token::Comment);
        assert_eq!(comment.color, Some(rgb(0x9399b2).into()));
        assert_eq!(comment.font_style, Some(FontStyle::Italic));
    }

    #[test]
    fn diff_tints_are_low_alpha_role_colors() {
        let t = embedded_mocha();
        assert_eq!(added_row_bg(&t).a, 0.125);
        assert_eq!(added_word_bg(&t).a, 0.282);
        // Row tint carries the created hue.
        assert_eq!(added_row_bg(&t).r, t.created.r);
    }
}
```

- [ ] **Step 9: Migrate the 1:1 renames in `main.rs` and `settings_ui.rs`**

Apply the mechanical renames from the **Accessor migration map** (`crust→background`, `base→editor_bg`, `mantle→surface`, `subtext→text_muted`, `overlay0→text_subtle`). These names are unambiguous, so a global replace of each `theme::<old>()` token is safe. After this step, run `cargo build -p lgtm 2>&1 | tail -30` — remaining errors should only reference `surface0`, `green`, `red`, `blue`, `mauve`, `peach`, `by_name`, or `all_names`.

- [ ] **Step 10: Migrate the split accessors (`surface0`, `green`, `red`, `blue`, `mauve`, `peach`) per the map**

Using the per-line lists in the **Accessor migration map**, repoint each site to `border()`/`element_bg()`/`created()`/`success()`/`deleted()`/`error()`/`accent()`/`modified()`/`warning()` as listed. Include the mirrored **test assertions** (6863/6864/6867/6871/6873) so unit tests match production. After this step, `cargo build -p lgtm 2>&1 | tail -30` should only error on `by_name`/`all_names` at the render sites, boot, and the picker.

- [ ] **Step 11: Repoint boot, render-site `by_name`, and the interim picker**

1. **Boot** — `main.rs:83`:

```rust
            theme::apply_ui_theme(&theme::load_active(&settings.theme_name), cx);
```

2. **Render-site `by_name`** — replace each `theme::by_name(&…theme_name)` with `theme::active()`:
   - `main.rs:5152-5154` and `main.rs:5764-5766` → `.bg(theme::palette_backdrop(&theme::active()))`.
   - `main.rs:5971` → `let active_theme = theme::active();`.
   - `main.rs:6107` (`OpenSettings`, cursor start) → `let active_theme = cx.global::<settings::Settings>().theme_name.clone();` then use `active_theme` as the name string (see Step 11.4). Since there is no `by_name(...).name` anymore, use the stored name directly.
   - `settings_ui.rs:314` → `.bg(theme::palette_backdrop(&theme::active()))`.

3. **`preview_theme` / `apply_and_save`** — in `settings_ui.rs`, replace both `theme::apply_ui_theme(&theme::by_name(&name), cx)` / `(&theme::by_name(&s.theme_name), cx)` calls with the loader:
   - `preview_theme` (line ~102): `theme::apply_ui_theme(&theme::load_active(&name), cx);`
   - `apply_and_save` (line ~84): `theme::apply_ui_theme(&theme::load_active(&s.theme_name), cx);`

4. **Interim picker** — the picker still calls `theme::all_names()` (deleted). Replace it with a temporary single-entry list so the modal compiles and shows Mocha (Task 6 makes it dynamic). Add to `mod.rs`:

```rust
/// Interim: names available for the picker. Replaced by the registry in the
/// discovery task. Only the embedded default exists until then.
pub fn available_names() -> Vec<String> {
    vec!["Catppuccin Mocha".to_string()]
}
```

   Then in `settings_ui.rs`, replace the three `theme::all_names()` uses:
   - `preview_theme_at` (line ~110): `let names = theme::available_names();` and index with `names[ix].as_str()` (adjust the `&'static str` binding to `&str`).
   - the hover-revert `position` lookup (line ~235) and the `for (i, name) in …enumerate()` loop (line ~243): iterate `theme::available_names()`. Change the row `name: &'static str` binding to an owned `String` (clone into the `on_click`/`on_hover` closures as needed).
   - `SettingsThemeConfirm` handler (line ~348): `let names = theme::available_names();` then `let name = names[ix.min(names.len().saturating_sub(1))].clone();`.

   Adjust `commit_theme`/`preview_theme`/`preview_theme_at`/`commit_theme` signatures that took `&'static str` to take `&str` where the borrow is no longer `'static`.

- [ ] **Step 12: Build, verify no old names remain, run the full suite**

Run:

```bash
cargo build -p lgtm 2>&1 | tail -20
grep -rnE "theme::(base|mantle|crust|surface0|subtext|overlay0|green|red|blue|mauve|peach|by_name|all_names)\b" crates/app/src && echo "STILL PRESENT" || echo "clean"
cargo test -p lgtm 2>&1 | tail -30
```
Expected: build succeeds; grep prints `clean`; all tests pass.

- [ ] **Step 13: Commit**

```bash
git add crates/app/src/theme/mod.rs crates/app/src/main.rs crates/app/src/settings_ui.rs
git commit -m "refactor(theme): migrate all consumers to Zed semantic roles"
```

---

## Task 5: Boot targeted disk-resolve + persistence fallback

Extend `load_active` so a persisted non-Mocha name is resolved from disk (scan theme dirs, parse only until the named variant is found), falling back to embedded Mocha when absent. Boot still does zero disk work for the default.

**Files:**
- Modify: `crates/app/src/theme/mod.rs` (`load_active`)
- Create: `crates/app/src/theme/registry.rs` (add `theme_dirs()` here; `discover()` comes in Task 6)
- Modify: `crates/app/src/settings.rs` (persistence-fallback doc comment)
- Test: unit tests in `crates/app/src/theme/registry.rs`

**Interfaces:**
- Consumes: `zed::parse_variants`, `resolver::resolve`, `embedded::embedded_mocha`.
- Produces: `pub fn theme_dirs() -> Vec<std::path::PathBuf>`; extended `load_active`.

- [ ] **Step 1: Write the failing `theme_dirs` / targeted-resolve test**

Create `crates/app/src/theme/registry.rs` with tests only:

```rust
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
            r#"{ "name": "Fam", "themes": [
                { "name": "My Dark", "appearance": "dark",
                  "style": { "editor.background": "#123456", "text": "#ffffff" } } ] }"#,
        )
        .unwrap();

        let found = resolve_named_in(&[dir.clone()], "My Dark").expect("variant found");
        assert_eq!(found.name, "My Dark");
        assert_eq!(found.editor_bg, gpui::rgb(0x123456));

        let missing = resolve_named_in(&[dir.clone()], "Nope");
        assert!(missing.is_none());

        std::fs::remove_dir_all(&dir).ok();
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p lgtm --lib theme::registry 2>&1 | tail -20`
Expected: FAIL — `theme_dirs`, `resolve_named_in` not defined.

- [ ] **Step 3: Implement `theme_dirs` and `resolve_named_in`**

Prepend to `crates/app/src/theme/registry.rs`:

```rust
//! Theme discovery on disk. `theme_dirs()` enumerates where themes may live;
//! `resolve_named_in` does the boot-time targeted resolve (parse only until the
//! named variant is found). The full background scan (`discover`) is added with
//! the settings picker.

use crate::theme::model::{Appearance, Theme};
use crate::theme::{resolver, zed};
use std::path::PathBuf;

/// Directories scanned for external Zed themes, in precedence order (later
/// entries override earlier ones during discovery). App dir first, then Zed's
/// user themes dir. Missing dirs are simply skipped by callers.
pub fn theme_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(config) = dirs::config_dir() {
        dirs.push(config.join("lgtm").join("themes"));
        dirs.push(config.join("zed").join("themes"));
    }
    dirs
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
```

- [ ] **Step 4: Extend `load_active` to targeted-resolve from disk**

Replace the interim `load_active` in `mod.rs` with:

```rust
/// Resolve a persisted theme name to a concrete `Theme` for boot/preview. The
/// embedded default needs zero disk access; any other name triggers a targeted
/// disk scan (parse only until the variant is found). A name that is nowhere on
/// disk falls back to the embedded default, so boot never fails.
pub fn load_active(name: &str) -> Theme {
    if name == "Catppuccin Mocha" {
        return embedded_mocha();
    }
    registry::resolve_named_in(&registry::theme_dirs(), name).unwrap_or_else(embedded_mocha)
}
```

> Uncomment (or add) `mod registry;` in `mod.rs` now — this is the deferred declaration from Task 4 Step 1, and `registry.rs` exists as of this task. Add `pub use` only if other modules need `theme_dirs`; the accessors reach it via the `registry::` path.

- [ ] **Step 5: Document the persistence fallback**

In `crates/app/src/settings.rs`, update the doc comment on the `theme_name` field (struct `Settings`, around line 16–21) to note the new fallback. Change the field to:

```rust
    /// Persisted theme name. Resolved at boot via `theme::load_active`: the
    /// embedded Catppuccin Mocha applies with zero disk access; any other name
    /// is targeted-resolved from the theme dirs, falling back to Mocha when the
    /// named theme is absent (e.g. a config naming a no-longer-bundled family).
    pub theme_name: String,
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p lgtm --lib theme:: 2>&1 | tail -20`
Expected: PASS (all theme tests, including the two new registry tests).

- [ ] **Step 7: Commit**

```bash
git add crates/app/src/theme/registry.rs crates/app/src/theme/mod.rs crates/app/src/settings.rs
git commit -m "feat(theme): targeted disk-resolve of persisted themes at boot"
```

---

## Task 6: Background discovery + transient registry + dynamic picker

Add the full `ThemeRegistry`, a blocking `discover()` scan run on a background executor when Settings opens, seeded so the picker is never empty, and rewire the picker to iterate resolved themes (no re-resolve). The registry is dropped when the modal closes.

**Files:**
- Modify: `crates/app/src/theme/registry.rs` (`ThemeRegistry`, `discover`)
- Modify: `crates/app/src/theme/mod.rs` (`pub use` registry types)
- Modify: `crates/app/src/settings_ui.rs` (`Discovery` state, background task, picker)
- Modify: `crates/app/src/main.rs` (`OpenSettings`: seed + spawn discovery)
- Test: unit tests in `crates/app/src/theme/registry.rs`

**Interfaces:**
- Consumes: `resolver`, `zed::parse_variants`, `embedded_mocha`, `theme_dirs`.
- Produces:
  - `pub struct ThemeRegistry { … }` with `pub fn seeded(active: Theme) -> Self`, `pub fn insert(&mut self, Theme)` (override by name), `pub fn merge(&mut self, Vec<Theme>)`, `pub fn names(&self) -> Vec<String>`, `pub fn get(&self, name: &str) -> Option<&Theme>`, `pub fn len(&self)`.
  - `pub fn discover() -> Vec<Theme>` (blocking; safe to call off-thread).
  - `settings_ui`: `enum Discovery { Loading, Ready(ThemeRegistry) }` on `SettingsUi`.

- [ ] **Step 1: Write the failing registry tests**

Add to the `tests` module in `crates/app/src/theme/registry.rs`:

```rust
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
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p lgtm --lib theme::registry 2>&1 | tail -20`
Expected: FAIL — `ThemeRegistry` not defined.

- [ ] **Step 3: Implement `ThemeRegistry` and `discover`**

Add to `crates/app/src/theme/registry.rs` (above the `tests` module):

```rust
use crate::theme::embedded::embedded_mocha;

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
        let mut reg = ThemeRegistry { order: Vec::new(), themes: std::collections::HashMap::new() };
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

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
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
```

- [ ] **Step 4: Re-export registry types from `mod.rs`**

In `crates/app/src/theme/mod.rs`, add near the other `pub use`:

```rust
pub use registry::{discover, theme_dirs, ThemeRegistry};
```

Remove the interim `available_names()` function added in Task 4 (the picker no longer uses it after Step 7 below); confirm no other caller references it via `grep -rn "available_names" crates/app/src`.

- [ ] **Step 5: Run registry tests to verify pass**

Run: `cargo test -p lgtm --lib theme::registry 2>&1 | tail -20`
Expected: PASS (4 registry tests).

- [ ] **Step 6: Add `Discovery` state to `SettingsUi`**

In `crates/app/src/settings_ui.rs`, add the enum and a field:

```rust
/// Theme-discovery state for the open settings modal. Seeded synchronously so
/// the picker always has at least the embedded default + active theme; a
/// background task fills in disk themes and flips this to `Ready`.
pub enum Discovery {
    Loading(theme::ThemeRegistry),
    Ready(theme::ThemeRegistry),
}

impl Discovery {
    pub fn registry(&self) -> &theme::ThemeRegistry {
        match self {
            Discovery::Loading(r) | Discovery::Ready(r) => r,
        }
    }
    pub fn is_loading(&self) -> bool {
        matches!(self, Discovery::Loading(_))
    }
}
```

Add to `struct SettingsUi`:

```rust
    /// Theme registry + discovery status. Seeded on open, filled by a
    /// background scan, dropped when the modal closes.
    pub discovery: Discovery,
    /// The background discovery task; dropping it (with the modal) cancels an
    /// in-flight scan.
    pub _discovery_task: gpui::Task<()>,
```

- [ ] **Step 7: Rewrite the picker to iterate the registry**

In `settings_ui.rs`, replace the interim `theme::available_names()` uses (from Task 4 Step 11.4) with registry iteration read from `ui.discovery.registry().names()`. Specifically:

- In `render_settings`, before building the list: `let names = ui.discovery.registry().names();` and `let discovering = ui.discovery.is_loading();`.
- The `for (i, name) in names.iter().enumerate()` loop builds rows exactly as before (name is now `&String`; clone into closures).
- The hover-revert `position` lookup uses `names`.
- When `discovering`, append a subtle affordance row after the list, e.g.:

```rust
        if discovering {
            theme_list = theme_list.child(
                div()
                    .px_2()
                    .py_1()
                    .text_color(theme::text_subtle())
                    .child(SharedString::from("Discovering themes…")),
            );
        }
```

- `preview_theme_at` uses `theme::available_names()` → change to accept the names from the registry: read `let names = app.settings.as_ref().map(|ui| ui.discovery.registry().names()).unwrap_or_default();` (guard empty), clamp `ix`, and preview `names[ix]`.
- `SettingsThemeConfirm` similarly reads names from the registry.

- [ ] **Step 8: Apply previews/commits from the registry, not `load_active`**

The registry already holds resolved `Theme`s, so preview/commit apply them directly (no re-resolve). Update `preview_theme` in `settings_ui.rs`:

```rust
fn preview_theme(app: &mut ReviewApp, name: &str, cx: &mut Context<ReviewApp>) {
    let name = name.to_string();
    let resolved = app
        .settings
        .as_ref()
        .and_then(|ui| ui.discovery.registry().get(&name).cloned())
        .unwrap_or_else(|| theme::load_active(&name));
    cx.update_global::<settings::Settings, _>(|s, _| s.theme_name = name.clone());
    theme::apply_ui_theme(&resolved, cx);
    app.char_width = None;
    cx.notify();
}
```

Update `apply_and_save` the same way — resolve the committed name from the registry, falling back to `theme::load_active`:

```rust
    let s = cx.global::<settings::Settings>().clone();
    let resolved = app
        .settings
        .as_ref()
        .and_then(|ui| ui.discovery.registry().get(&s.theme_name).cloned())
        .unwrap_or_else(|| theme::load_active(&s.theme_name));
    theme::apply_ui_theme(&resolved, cx);
```

- [ ] **Step 9: Seed the registry and spawn discovery in `OpenSettings`**

In `crates/app/src/main.rs`, in the `OpenSettings` handler where `SettingsUi { … }` is constructed (~line 6112), seed the registry from the active theme and spawn a background scan that merges results back:

```rust
                let seed = theme::ThemeRegistry::seeded(theme::active());
                let discovery_task = cx.spawn_in(window, async move |this, cx| {
                    let found = cx
                        .background_executor()
                        .spawn(async move { theme::discover() })
                        .await;
                    this.update(cx, |app, cx| {
                        if let Some(ui) = &mut app.settings {
                            if let theme::Discovery::Loading(reg) | theme::Discovery::Ready(reg) =
                                &mut ui.discovery
                            {
                                reg.merge(found);
                            }
                            // Flip to Ready.
                            if let Some(ui) = &mut app.settings {
                                let reg = match std::mem::replace(
                                    &mut ui.discovery,
                                    theme::Discovery::Ready(theme::ThemeRegistry::seeded(
                                        theme::active(),
                                    )),
                                ) {
                                    theme::Discovery::Loading(r) | theme::Discovery::Ready(r) => r,
                                };
                                ui.discovery = theme::Discovery::Ready(reg);
                            }
                            cx.notify();
                        }
                    })
                    .ok();
                });
```

Then include `discovery: theme::Discovery::Loading(seed), _discovery_task: discovery_task,` in the `SettingsUi { … }` initializer.

> Note: `Discovery` is defined in `settings_ui` but referenced from `main.rs`. Either re-export it (`pub use settings_ui::Discovery;`) or qualify as `settings_ui::Discovery`. Match whichever the codebase uses for `SettingsUi`. Verify the exact `cx.spawn_in` / `background_executor().spawn` signatures against the installed gpui 0.2 (`cargo doc -p gpui --open`, or grep existing `spawn` uses in `main.rs`); adjust the closure shape to compile. The merge-then-flip is written defensively; simplify to a single `mem::replace` if the borrow checker allows.

- [ ] **Step 10: Update the `OpenSettings` cursor-start to a registry name**

Where `theme_cursor` is computed (~line 6107), set it from the active theme's name position in the seeded registry names (fallback 0):

```rust
                let active_name = s.theme_name.clone();
                let names = seed.names();
                let theme_cursor = names.iter().position(|n| *n == active_name).unwrap_or(0);
```

(Compute `seed` before this so its `names()` is available; reorder as needed.)

- [ ] **Step 11: Build and run the full suite**

Run:

```bash
cargo build -p lgtm 2>&1 | tail -30
cargo test -p lgtm 2>&1 | tail -30
```
Expected: build succeeds; all tests pass.

- [ ] **Step 12: Manual smoke test (verification)**

Run the app, open Settings (`cmd-,`), and confirm: the theme list shows "Catppuccin Mocha" (plus any themes present in `~/.config/lgtm/themes` or `~/.config/zed/themes`), a brief "Discovering themes…" affordance may appear, hovering previews live, clicking commits + persists, and closing the modal keeps the applied theme. To exercise external loading, drop any Zed theme JSON into `~/.config/lgtm/themes/` first.

```bash
cargo run -p lgtm
```

- [ ] **Step 13: Commit**

```bash
git add crates/app/src/theme/registry.rs crates/app/src/theme/mod.rs crates/app/src/settings_ui.rs crates/app/src/main.rs
git commit -m "feat(theme): background discovery, transient registry, dynamic picker"
```

---

## Task 7: Golden/regression fixtures for the Zed-key and syntax-key mappings

Guard the Zed-role→app-role and syntax-key→`Token` mappings with checked-in fixtures: one Catppuccin family (already bundled) and one non-Catppuccin family. Snapshot the resolved roles so a mapping regression fails loudly.

**Files:**
- Create: `crates/app/src/theme/fixtures/tokyo-night.json` (a minimal non-Catppuccin Zed family)
- Create: `crates/app/src/theme/golden.rs` (or add to `registry.rs` tests)
- Modify: `crates/app/src/theme/mod.rs` (`#[cfg(test)] mod golden;`)

**Interfaces:**
- Consumes: `zed::parse_variants`, `resolver::resolve`.

- [ ] **Step 1: Create the non-Catppuccin fixture**

Create `crates/app/src/theme/fixtures/tokyo-night.json` — a compact but valid Zed family exercising distinct role values and a few syntax keys (values chosen so each resolved role is unambiguous):

```json
{
  "$schema": "https://zed.dev/schema/themes/v0.2.0.json",
  "name": "Tokyo Night Test",
  "author": "fixture",
  "themes": [
    {
      "name": "Tokyo Night Test",
      "appearance": "dark",
      "style": {
        "background": "#1a1b26",
        "editor.background": "#16161e",
        "surface.background": "#1f2335",
        "element.background": "#292e42",
        "border": "#292e42",
        "text": "#c0caf5",
        "text.muted": "#a9b1d6",
        "text.placeholder": "#565f89",
        "text.accent": "#7aa2f7",
        "created": "#9ece6a",
        "deleted": "#f7768e",
        "modified": "#e0af68",
        "players": [{ "cursor": "#c0caf5", "selection": "#7aa2f733" }],
        "syntax": {
          "keyword": { "color": "#bb9af7" },
          "function": { "color": "#7aa2f7" },
          "comment": { "color": "#565f89", "font_style": "italic" },
          "variable.parameter": { "color": "#e0af68", "font_style": "italic" }
        }
      }
    }
  ]
}
```

- [ ] **Step 2: Write the golden test**

Create `crates/app/src/theme/golden.rs`:

```rust
//! Regression fixtures pinning the Zed-role→app-role and syntax-key→Token
//! mappings. A change to either mapping table changes these snapshots and
//! fails the test — the guardrail against silent remapping.

#[cfg(test)]
mod tests {
    use crate::theme::model::Appearance;
    use crate::theme::{resolver, zed};
    use gpui::{rgb, rgba};
    use syntax::Token;

    const TOKYO: &str = include_str!("fixtures/tokyo-night.json");

    fn resolve_first(text: &str) -> crate::theme::model::Theme {
        let def = zed::parse_variants(text).unwrap().into_iter().next().unwrap();
        resolver::resolve(def.name, Appearance::from(def.appearance), &def.style)
    }

    #[test]
    fn tokyo_night_roles_snapshot() {
        let t = resolve_first(TOKYO);
        assert_eq!(t.background, rgb(0x1a1b26));
        assert_eq!(t.editor_bg, rgb(0x16161e));
        assert_eq!(t.surface, rgb(0x1f2335));
        assert_eq!(t.element_bg, rgb(0x292e42));
        assert_eq!(t.border, rgb(0x292e42));
        assert_eq!(t.text, rgb(0xc0caf5));
        assert_eq!(t.text_muted, rgb(0xa9b1d6));
        assert_eq!(t.text_subtle, rgb(0x565f89));
        assert_eq!(t.accent, rgb(0x7aa2f7));
        assert_eq!(t.created, rgb(0x9ece6a));
        assert_eq!(t.deleted, rgb(0xf7768e));
        assert_eq!(t.modified, rgb(0xe0af68));
        // success/error/warning cross-fill from created/deleted/modified.
        assert_eq!(t.success, rgb(0x9ece6a));
        assert_eq!(t.error, rgb(0xf7768e));
        assert_eq!(t.warning, rgb(0xe0af68));
        assert_eq!(t.selection, rgba(0x7aa2f733));
    }

    #[test]
    fn tokyo_night_syntax_snapshot() {
        let t = resolve_first(TOKYO);
        assert_eq!(t.syntax(Token::Keyword).color, rgb(0xbb9af7));
        assert_eq!(t.syntax(Token::Function).color, rgb(0x7aa2f7));
        assert_eq!(t.syntax(Token::Comment).color, rgb(0x565f89));
        assert!(t.syntax(Token::Comment).italic);
        assert_eq!(t.syntax(Token::Parameter).color, rgb(0xe0af68));
        assert!(t.syntax(Token::Parameter).italic);
        // A token absent from the fixture falls back to plain text.
        assert_eq!(t.syntax(Token::Operator).color, t.text);
    }
}
```

- [ ] **Step 3: Declare the golden module**

In `crates/app/src/theme/mod.rs`, add:

```rust
#[cfg(test)]
mod golden;
```

- [ ] **Step 4: Run the golden tests**

Run: `cargo test -p lgtm --lib theme::golden 2>&1 | tail -20`
Expected: PASS (2 tests).

- [ ] **Step 5: Full suite + commit**

```bash
cargo test -p lgtm 2>&1 | tail -20
git add crates/app/src/theme/fixtures/ crates/app/src/theme/golden.rs crates/app/src/theme/mod.rs
git commit -m "test(theme): golden fixtures pin role and syntax-key mappings"
```

---

## Self-Review

**Spec coverage:**

- §1 Data model & Zed JSON mapping → Tasks 1 (raw parse), 2 (resolved model + role table), with the mapping table encoded in `RawStyle` renames + resolver + the Global Constraints syntax-key table. ✓
- §1 Non-1:1 accessor cases (`surface0`, `subtext`/`overlay0`, `mauve`) → Task 4 migration map, per-line. ✓
- §2 Boot no-discovery / targeted resolve → Task 5 (`load_active`, `resolve_named_in`). ✓
- §2 On-open background discovery / transient registry / drop-on-close → Task 6. ✓
- §2 Sources & precedence (embedded → app dir → zed dir, later wins) → Task 5 `theme_dirs`, Task 6 `discover`/`insert`. ✓
- §2 Persistence unchanged + fallback to Mocha → Task 5 Step 5, `load_active` fallback. ✓
- §2 Picker dynamic → Task 6 Step 7. ✓
- §3 Consumer migration (accessors, token_style, tints, apply_ui_theme) → Task 4. ✓
- §4 Fallback resolver → Task 2 resolver. ✓
- §4 Error handling (per-file/variant/dir containment, embedded validated) → Task 1 `parse_variants`, Task 3 test, Tasks 5/6 skip-and-log. ✓
- §4 Testing (parser/resolver units, embedded default, golden, registry) → Tasks 1–3, 6, 7. ✓

**Placeholder scan:** No "TBD"/"add error handling"/"similar to". Every code step carries full code. The only non-literal spots are the deliberate line-number references in Task 4 (drift-prone; mitigated by the Step 12 zero-remaining grep) and the gpui-`spawn` signature note in Task 6 Step 9 (flagged for local verification against the installed gpui — unavoidable without the exact 0.2 async API in front of us).

**Type consistency:** `Theme` = `model::Theme` throughout after Task 4; accessors all `-> Rgba`; tints all `(&Theme) -> Rgba`; `resolve(name, Appearance, &RawStyle) -> Theme` used identically in embedded/registry/golden; `Appearance::from(RawAppearance)` used at every resolve call; `ThemeRegistry` methods (`seeded`/`insert`/`merge`/`names`/`get`/`len`) consistent between definition (Task 6 Step 3) and uses (Steps 6–10, tests). `parse_variants` signature stable across Tasks 1/3/5/6/7.

**Known accepted behavior shifts** (all spec-sanctioned): default accent/link/primary is now mauve; comment color is `#9399b2` (was `#6c7086`); parameter is no longer italic; `background`/backdrop sites now use Zed `background` (`#27273b`) rather than the old crust (`#11111b`); non-bundled persisted themes fall back to Mocha until present on disk.
