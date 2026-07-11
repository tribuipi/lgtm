# Settings (Themes, Fonts, Size) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an in-app settings UI (opened with `cmd-,`) that lets users pick a built-in theme, choose a UI font and a code font from installed system fonts, and set a single font size that scales the whole app — all persisted across launches.

**Architecture:** A `Settings` struct registered as a gpui `Global` is the single source of truth. `theme.rs` is refactored from hardcoded functions into a data-driven `Theme` struct with built-in constructors (the seam for future external themes). `main.rs` reads fonts/size/theme from the global instead of the `MONO`/`TEXT_SIZE`/`ROW_HEIGHT` constants; chrome sizes scale via a helper. The settings modal follows the existing command-palette pattern: state on `ReviewApp`, rendered inline, driven by a new action.

**Tech Stack:** Rust, gpui 0.2, gpui-component 0.5, serde + toml (persistence), dirs (config path), fuzzy-matcher (already used for the palette).

## Global Constraints

- Rust edition 2021; workspace resolver 2.
- gpui `= "0.2"` (with `runtime_shaders`), gpui-component `= "0.5"`. Do not bump these.
- `serde = { version = "1", features = ["derive"] }` is already a workspace dependency — reuse it.
- The **default** appearance must be byte-for-byte identical to today: theme = Catppuccin Mocha, code font = `Menlo`, UI font = the current default (unset), size = `13.0`, so a fresh install with no config file looks unchanged.
- Config persists as TOML at `<config_dir>/lgtm/config.toml` where `<config_dir>` is `dirs::config_dir()` (macOS: `~/Library/Application Support`).
- Never panic on bad/missing config — fall back to defaults per field.
- Follow the existing in-file patterns: modal state lives on `ReviewApp` and is rendered by a free `render_*` function (mirroring `render_palette`), not a standalone gpui `Entity`.
- Font-size clamp: `8.0..=32.0`.
- Chrome scale baseline is `13.0` (today's `TEXT_SIZE`); `scale = font_size / 13.0`.
- Row-height ratio is `22.0 / 13.0` (today's `ROW_HEIGHT / TEXT_SIZE`).

---

## File Structure

- **Create `crates/app/src/settings.rs`** — `Settings` struct (`Global`), defaults, `load()`/`save()`, derived accessors (`scale`, `chrome`, `row_height`, `theme`). Unit-tested.
- **Rewrite `crates/app/src/theme.rs`** — `Theme` struct + built-in constructors (`catppuccin_mocha`, `catppuccin_latte`, one more dark), `by_name`, `all_names`, `apply_ui_theme(&Theme, cx)`, `token_style(&Theme, Token)`. Unit-tested. Keep all non-theme helpers (`palette_backdrop`, `void_cell_bg`, `selection_bg`, `added_*`/`removed_*`) but source their colors from the active `Theme`.
- **Create `crates/app/src/settings_ui.rs`** — `SettingsUi` modal state struct + `render_settings(entity, window, cx)` free fn + apply/save helpers.
- **Modify `crates/app/src/main.rs`** — declare the new modules; load the `Settings` global at startup; replace `MONO`/`TEXT_SIZE`/`ROW_HEIGHT` reads; scale chrome sizes; add `OpenSettings` action + `cmd-,` binding; hold `settings: Option<SettingsUi>` on `ReviewApp`; render the modal; invalidate `char_width` on font/size change.
- **Modify `crates/app/Cargo.toml`** — add `serde` (workspace), `toml`, `dirs`.
- **Modify `Cargo.toml`** (workspace) — add `toml` and `dirs` to `[workspace.dependencies]`.
- **Modify `README.md`** — add `cmd-,` to the keymap and a Settings note to Features.

---

## Task 1: `Settings` global + persistence

**Files:**
- Create: `crates/app/src/settings.rs`
- Modify: `Cargo.toml` (workspace `[workspace.dependencies]`)
- Modify: `crates/app/Cargo.toml` (`[dependencies]`)
- Test: inline `#[cfg(test)]` module in `crates/app/src/settings.rs`

**Interfaces:**
- Produces:
  - `pub struct Settings { pub theme_name: String, pub ui_font: Option<String>, pub code_font: String, pub font_size: f32 }`
  - `impl Default for Settings` → `{ theme_name: "Catppuccin Mocha", ui_font: None, code_font: "Menlo", font_size: 13.0 }`
  - `impl gpui::Global for Settings {}`
  - `Settings::load() -> Settings` (reads config file, defaults on any error)
  - `Settings::save(&self)` (writes TOML; best-effort, logs on failure)
  - `Settings::config_path() -> Option<std::path::PathBuf>`
  - `Settings::from_toml_str(&str) -> Settings` (parse helper; defaults for missing/invalid fields — this is the unit-testable core of `load`)
  - `Settings::scale(&self) -> f32`
  - `Settings::chrome(&self, base: f32) -> gpui::Pixels`
  - `Settings::row_height(&self) -> f32`
  - `Settings::set_font_size(&mut self, v: f32)` (clamps to `8.0..=32.0`)

- [ ] **Step 1: Add dependencies**

In workspace `Cargo.toml` under `[workspace.dependencies]`, add:

```toml
toml = "0.8"
dirs = "5"
```

In `crates/app/Cargo.toml` under `[dependencies]`, add:

```toml
serde = { workspace = true }
toml = { workspace = true }
dirs = { workspace = true }
```

- [ ] **Step 2: Write the failing tests**

Create `crates/app/src/settings.rs` with the test module first:

```rust
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
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p lgtm settings::`
Expected: FAIL to compile (`Settings` not defined).

- [ ] **Step 4: Write the implementation**

Above the test module in `crates/app/src/settings.rs`:

```rust
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
```

> Note: `#[serde(default)]` on the struct makes every missing field fall back to `Default`, which is what makes partial-TOML parsing safe. `toml` cannot serialize a top-level `Option` that is `None` cleanly in all versions — `Option<String>` is fine here because it's a struct field (serialized as absent when `None`).

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p lgtm settings::`
Expected: PASS (5 tests).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/app/Cargo.toml crates/app/src/settings.rs
git commit -m "feat(settings): Settings global with TOML persistence"
```

---

## Task 2: Data-driven `Theme`

**Files:**
- Rewrite: `crates/app/src/theme.rs`
- Test: inline `#[cfg(test)]` module in `crates/app/src/theme.rs`

**Interfaces:**
- Consumes: `syntax::Token` (unchanged).
- Produces:
  - `pub struct Theme { pub name: &'static str, pub mode: ThemeMode, /* palette fields */ ..., pub syntax: fn(Token) -> (u32, bool) }` — a struct carrying every color `apply_ui_theme` needs plus a syntax color+italic lookup. (Concrete field list below.)
  - `pub fn catppuccin_mocha() -> Theme`, `pub fn catppuccin_latte() -> Theme`, `pub fn tokyo_night() -> Theme`
  - `pub fn all_names() -> &'static [&'static str]`
  - `pub fn by_name(name: &str) -> Theme` (falls back to `catppuccin_mocha()` on unknown)
  - `pub fn apply_ui_theme(theme: &Theme, cx: &mut App)`
  - `pub fn token_style(theme: &Theme, token: Token) -> HighlightStyle`
  - `pub fn palette_backdrop(theme: &Theme) -> Rgba`, `void_cell_bg(theme: &Theme) -> Rgba`, `selection_bg(theme: &Theme) -> Rgba`, `added_row_bg(theme)`, `removed_row_bg(theme)`, `added_word_bg(theme)`, `removed_word_bg(theme)`

**Design note on struct shape.** To keep this tractable, `Theme` stores the raw `u32` palette colors (base, mantle, crust, surface0, text, subtext, overlay0, green, red, blue, mauve, peach) plus `mode`, and a `syntax: fn(Token) -> (u32, bool)` function pointer. `apply_ui_theme` and the tint helpers derive their `Hsla`/`Rgba` from those `u32`s exactly as today. This means each built-in is a small literal + a `match` fn, and the Mocha built-in reproduces the current values verbatim.

- [ ] **Step 1: Write the failing tests**

Add to the bottom of `crates/app/src/theme.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use syntax::Token;

    #[test]
    fn by_name_resolves_builtins() {
        assert_eq!(by_name("Catppuccin Mocha").name, "Catppuccin Mocha");
        assert_eq!(by_name("Catppuccin Latte").name, "Catppuccin Latte");
        assert_eq!(by_name("Tokyo Night").name, "Tokyo Night");
    }

    #[test]
    fn by_name_unknown_falls_back_to_mocha() {
        assert_eq!(by_name("does-not-exist").name, "Catppuccin Mocha");
    }

    #[test]
    fn mocha_preserves_current_values() {
        let t = catppuccin_mocha();
        assert_eq!(t.base, 0x1e1e2e);
        assert_eq!(t.text, 0xcdd6f4);
        assert_eq!(t.blue, 0x89b4fa);
        // syntax: keyword is mauve + not italic; comment is overlay0 + italic.
        assert_eq!((t.syntax)(Token::Keyword), (0xcba6f7, false));
        assert_eq!((t.syntax)(Token::Comment), (0x6c7086, true));
    }

    #[test]
    fn all_names_lists_every_builtin() {
        let names = all_names();
        assert!(names.contains(&"Catppuccin Mocha"));
        assert!(names.contains(&"Catppuccin Latte"));
        assert!(names.contains(&"Tokyo Night"));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn latte_is_light_mode() {
        assert!(matches!(catppuccin_latte().mode, ThemeMode::Light));
        assert!(matches!(catppuccin_mocha().mode, ThemeMode::Dark));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p lgtm theme::`
Expected: FAIL to compile.

- [ ] **Step 3: Rewrite the implementation**

Replace the whole of `crates/app/src/theme.rs` with the data-driven version. The `Theme` struct, the three built-ins, and the rewritten `apply_ui_theme`/`token_style`/tint helpers. Preserve the **exact** current Mocha palette and syntax mapping (copy the `u32`s from the existing file: base `0x1e1e2e`, mantle `0x181825`, crust `0x11111b`, surface0 `0x313244`, text `0xcdd6f4`, subtext `0xa6adc8`, overlay0 `0x6c7086`, green `0xa6e3a1`, red `0xf38ba8`, blue `0x89b4fa`, mauve `0xcba6f7`, peach `0xfab387`; syntax mapping exactly as in the current `token_style`).

```rust
//! Data-driven themes. A `Theme` is a struct of palette colors + a syntax
//! lookup; built-in constructors produce concrete themes. This struct is the
//! seam a future external-theme loader (e.g. Helix .toml) targets.

use gpui::{rgb, rgba, App, FontStyle, HighlightStyle, Hsla, Rgba};
use gpui_component::{Theme as UiTheme, ThemeMode};
use syntax::Token;

#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub mode: ThemeMode,
    pub base: u32,
    pub mantle: u32,
    pub crust: u32,
    pub surface0: u32,
    pub text: u32,
    pub subtext: u32,
    pub overlay0: u32,
    pub green: u32,
    pub red: u32,
    pub blue: u32,
    pub mauve: u32,
    pub peach: u32,
    /// (color, italic) per syntax token.
    pub syntax: fn(Token) -> (u32, bool),
}

fn mocha_syntax(token: Token) -> (u32, bool) {
    match token {
        Token::Keyword => (0xcba6f7, false),
        Token::Function => (0x89b4fa, false),
        Token::Type => (0xf9e2af, false),
        Token::String => (0xa6e3a1, false),
        Token::Number | Token::Constant => (0xfab387, false),
        Token::Comment => (0x6c7086, true),
        Token::Property => (0xb4befe, false),
        Token::Variable | Token::Embedded => (0xcdd6f4, false),
        Token::Parameter => (0xeba0ac, true),
        Token::Operator => (0x89dceb, false),
        Token::Punctuation => (0x9399b2, false),
        Token::Attribute | Token::Label => (0xf9e2af, false),
        Token::Namespace => (0xfab387, true),
    }
}

pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin Mocha",
        mode: ThemeMode::Dark,
        base: 0x1e1e2e, mantle: 0x181825, crust: 0x11111b, surface0: 0x313244,
        text: 0xcdd6f4, subtext: 0xa6adc8, overlay0: 0x6c7086,
        green: 0xa6e3a1, red: 0xf38ba8, blue: 0x89b4fa, mauve: 0xcba6f7, peach: 0xfab387,
        syntax: mocha_syntax,
    }
}
```

Then implement `catppuccin_latte()` (light — Catppuccin Latte palette: base `0xeff1f5`, mantle `0xe6e9ef`, crust `0xdce0e8`, surface0 `0xccd0da`, text `0x4c4f69`, subtext `0x6c6f85`, overlay0 `0x9ca0b0`, green `0x40a02b`, red `0xd20f39`, blue `0x1e66f5`, mauve `0x8839ef`, peach `0xfe640b`, `mode: ThemeMode::Light`, with a `latte_syntax` fn mapping the same tokens to Latte colors) and `tokyo_night()` (dark — base `0x1a1b26`, mantle `0x16161e`, crust `0x13131a`, surface0 `0x292e42`, text `0xc0caf5`, subtext `0xa9b1d6`, overlay0 `0x565f89`, green `0x9ece6a`, red `0xf7768e`, blue `0x7aa2f7`, mauve `0xbb9af7`, peach `0xff9e64`, `mode: ThemeMode::Dark`, with a `tokyo_syntax` fn). Each `*_syntax` fn maps the same `Token` arms; pick sensible palette colors per token (keyword→mauve, function→blue, type→a yellow, string→green, number/constant→peach, comment→overlay0 italic, property→a lavender/blue, variable→text, parameter→red italic, operator→a cyan, punctuation→subtext, attribute/label→yellow, namespace→peach italic).

Then the lookups and application:

```rust
pub fn all_names() -> &'static [&'static str] {
    &["Catppuccin Mocha", "Catppuccin Latte", "Tokyo Night"]
}

pub fn by_name(name: &str) -> Theme {
    match name {
        "Catppuccin Latte" => catppuccin_latte(),
        "Tokyo Night" => tokyo_night(),
        _ => catppuccin_mocha(),
    }
}

pub fn apply_ui_theme(theme: &Theme, cx: &mut App) {
    UiTheme::change(theme.mode, None, cx);

    let base: Hsla = rgb(theme.base).into();
    let mantle: Hsla = rgb(theme.mantle).into();
    let crust: Hsla = rgb(theme.crust).into();
    let surface0: Hsla = rgb(theme.surface0).into();
    let text: Hsla = rgb(theme.text).into();
    let overlay0: Hsla = rgb(theme.overlay0).into();
    let green: Hsla = rgb(theme.green).into();
    let red: Hsla = rgb(theme.red).into();
    let blue: Hsla = rgb(theme.blue).into();
    let peach: Hsla = rgb(theme.peach).into();

    let t = UiTheme::global_mut(cx);
    // ... identical field assignments to the current apply_ui_theme body ...
}

pub fn token_style(theme: &Theme, token: Token) -> HighlightStyle {
    let (color, italic) = (theme.syntax)(token);
    HighlightStyle {
        color: Some(rgb(color).into()),
        font_style: italic.then_some(FontStyle::Italic),
        ..Default::default()
    }
}
```

Rewrite the tint helpers to take `&Theme` and build the `rgba` from the theme's `u32`s + the current alpha bytes, e.g.:

```rust
pub fn palette_backdrop(theme: &Theme) -> Rgba { rgba((theme.crust << 8) | 0xaa) }
pub fn void_cell_bg(theme: &Theme) -> Rgba { rgba((theme.crust << 8) | 0x99) }
pub fn selection_bg(theme: &Theme) -> Rgba { rgba((theme.blue << 8) | 0x4d) }
pub fn added_row_bg(theme: &Theme) -> Rgba { rgba((theme.green << 8) | 0x20) }
pub fn removed_row_bg(theme: &Theme) -> Rgba { rgba((theme.red << 8) | 0x20) }
pub fn added_word_bg(theme: &Theme) -> Rgba { rgba((theme.green << 8) | 0x48) }
pub fn removed_word_bg(theme: &Theme) -> Rgba { rgba((theme.red << 8) | 0x48) }
```

> `rgba(u32)` takes a `0xRRGGBBAA` value; `(color << 8) | alpha` composes the existing 6-hex color with the existing alpha byte, reproducing today's constants for Mocha (e.g. crust `0x11111b` → `0x11111baa`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p lgtm theme::`
Expected: PASS (5 tests). (This task leaves `main.rs` not-yet-compiling because callers pass no `&Theme`; that is fixed in Task 3. Run only the theme tests here — `cargo test -p lgtm theme::` builds the lib target; if the bin fails to build, that is expected and addressed next.)

> If `cargo test -p lgtm theme::` fails because the bin (`main.rs`) doesn't compile, temporarily verify with `cargo test -p lgtm theme:: --lib` is not applicable (single-bin crate). Instead, proceed to Task 3 in the same working session and run the theme tests at the end of Task 3. Do NOT commit a non-compiling tree — combine the Task 2 and Task 3 commits if needed.

- [ ] **Step 5: Commit (only if the tree compiles)**

If `main.rs` still references the old `theme::` signatures and won't build, defer this commit and fold it into Task 3's commit. Otherwise:

```bash
git add crates/app/src/theme.rs
git commit -m "feat(theme): data-driven Theme struct with built-in themes"
```

---

## Task 3: Wire settings into startup, fonts, and diff metrics

**Files:**
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `Settings` (Task 1), `theme::{by_name, apply_ui_theme, token_style, Theme, *_bg}` (Task 2).
- Produces: `ReviewApp` reads all appearance values from the `Settings` global; `char_width` resolves `settings.code_font`; a helper `settings(cx)` accessor pattern.

- [ ] **Step 1: Declare modules and load settings at startup**

At the top of `main.rs`, add module declarations next to `mod theme;`:

```rust
mod settings;
mod settings_ui; // added in Task 5; declare now only if Task 5 is done in the same session, else add in Task 5
```

> Declare `mod settings;` now. Declare `mod settings_ui;` in Task 5 to avoid an unresolved-module build error.

In `main()` inside the `run` closure, right after `gpui_component::init(cx);` replace the unconditional theme application:

```rust
gpui_component::init(cx);
let settings = settings::Settings::load();
theme::apply_ui_theme(&theme::by_name(&settings.theme_name), cx);
cx.set_global(settings);
```

Remove the old `theme::apply_ui_theme(cx);` call.

- [ ] **Step 2: Replace the font/size constants with settings reads**

The constants `MONO`, `TEXT_SIZE`, `ROW_HEIGHT` at the top of `main.rs` are today's sources of truth. Keep `MONO`/`TEXT_SIZE`/`ROW_HEIGHT` **removed as the diff's source** and read from the global instead. Concretely:

- In `ReviewApp::char_width` (around line 3257): resolve the code font and size from the global:

```rust
fn char_width(&mut self, window: &Window, cx: &App) -> Pixels {
    let (code_font, size) = {
        let s = cx.global::<settings::Settings>();
        (s.code_font.clone(), s.font_size)
    };
    *self.char_width.get_or_insert_with(|| {
        let text_system = window.text_system();
        let font_id = text_system.resolve_font(&font(SharedString::from(code_font)));
        text_system.em_advance(font_id, px(size)).unwrap_or(px(size * 0.6))
    })
}
```

> `char_width` now needs `&App`/`&Context` to read the global. Update its call sites to pass `cx`. If threading `cx` is impractical at a call site, read `(code_font, font_size)` into locals earlier in that method and pass them in. Prefer the smallest change that compiles.

- In the diff row render (around line 5808) replace:
  - `.font_family(MONO)` → `.font_family(SharedString::from(cx.global::<settings::Settings>().code_font.clone()))`
  - `.text_size(px(TEXT_SIZE))` → `.text_size(px(cx.global::<settings::Settings>().font_size))`
  - `.line_height(px(ROW_HEIGHT))` → `.line_height(px(cx.global::<settings::Settings>().row_height()))`

- Any other reads of `ROW_HEIGHT`/`TEXT_SIZE`/`MONO` (search the file) → route through the global similarly. Where a raw row-height number is needed for layout math (scroll offset → row), use `settings.row_height()` consistently so mouse math and layout agree.

- [ ] **Step 3: Point `token_style` calls at the active theme**

Find the `theme::token_style(token)` call (around line 1094). Change it to build the theme once per render pass and pass it:

```rust
let active_theme = theme::by_name(&cx.global::<settings::Settings>().theme_name);
// ...
let mut style = token.map(|t| theme::token_style(&active_theme, t)).unwrap_or_default();
```

Do the same for the tint helpers now taking `&Theme` (`added_row_bg(&active_theme)` etc.) and `palette_backdrop`/`void_cell_bg`/`selection_bg`. Thread a single `active_theme: Theme` (cloned; it's cheap — a handful of `u32`s + a fn ptr) into the render helpers that need colors, or recompute it locally where they're called.

- [ ] **Step 4: Build and run the theme tests**

Run: `cargo build -p lgtm` then `cargo test -p lgtm`
Expected: builds clean; all Task 1 + Task 2 tests PASS.

- [ ] **Step 5: Manual smoke test (defaults unchanged)**

Run: `cargo run --release` (or debug). Open a local repo diff.
Expected: the app looks **identical** to before — Catppuccin Mocha, Menlo, size 13, diff columns aligned, mouse selection lands on the right character. (No settings file exists yet, so defaults are in force.)

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/main.rs crates/app/src/theme.rs
git commit -m "feat(settings): read theme + fonts + size from Settings global"
```

---

## Task 4: Scale UI chrome sizes and apply the UI font

**Files:**
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `Settings::chrome`, `Settings::ui_font`.
- Produces: every chrome `text_size` scales with `font_size`; UI text uses the chosen UI font.

- [ ] **Step 1: Replace hardcoded chrome text sizes**

For each `.text_size(px(N.))` call that is **not** the diff pane's `px(TEXT_SIZE)` (i.e. the `px(10.)`, `px(11.)`, `px(12.)`, `px(13.)` chrome sizes at lines ~1193, 1314, 1890, 4690, 4725, 4768, 4789, 4837, 4848, 4919, 4993, 5012, 5121, 5152, 5302, 5334, 5348, 5354, 5416, 5516, 5527, 5719, 5728, 5767 and any others found by grep), replace:

```rust
.text_size(px(11.))
```

with:

```rust
.text_size(cx.global::<settings::Settings>().chrome(11.))
```

Use the matching base number for each site. Verify with:

Run: `grep -nE "\.text_size\(px\(" crates/app/src/main.rs`
Expected: only the diff pane's `px(...font_size...)` line(s) remain; no bare `px(10./11./12./13.)` chrome sizes left.

- [ ] **Step 2: Apply the UI font where text chrome is rendered**

Where the root/window text style is set, apply the UI font if present. If there is a top-level container in `ReviewApp::render`, set `.font_family(...)` on it when `ui_font` is `Some`:

```rust
let ui_font = cx.global::<settings::Settings>().ui_font.clone();
// on the root div:
let root = div(); // existing root
let root = if let Some(f) = ui_font { root.font_family(SharedString::from(f)) } else { root };
```

> The diff pane already sets its own `.font_family(code_font)` per row, so it is unaffected by the root UI font. Chrome inherits the root font.

- [ ] **Step 3: Build**

Run: `cargo build -p lgtm`
Expected: builds clean.

- [ ] **Step 4: Manual smoke test (still unchanged at defaults)**

Run: `cargo run`. With no config, size 13 → scale 1.0, so chrome sizes are unchanged; `ui_font` is `None`, so the root font is the default.
Expected: visually identical to before.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/main.rs
git commit -m "feat(settings): scale chrome sizes and apply UI font"
```

---

## Task 5: Settings modal UI + `cmd-,`

**Files:**
- Create: `crates/app/src/settings_ui.rs`
- Modify: `crates/app/src/main.rs`

**Interfaces:**
- Consumes: `Settings`, `theme::all_names`, `theme::by_name`, `theme::apply_ui_theme`, `cx.text_system().all_font_names()`, `fuzzy_matcher` (already imported).
- Produces:
  - In `main.rs`: an `OpenSettings` action, a `cmd-,` binding, a `settings: Option<settings_ui::SettingsUi>` field on `ReviewApp`, a render call in `ReviewApp::render`, and an `on_action` handler that opens the modal.
  - In `settings_ui.rs`: `pub struct SettingsUi { pub font_filter: gpui::Entity<InputState>, pub focus: SettingsField }`, `pub fn render_settings(app: &gpui::Entity<ReviewApp>, window: &mut Window, cx: &mut Context<ReviewApp>) -> impl IntoElement`, and helpers `apply_and_save(cx)` that re-applies the theme, clears `char_width`, saves, and notifies.

- [ ] **Step 1: Add the action, binding, and field**

In `main.rs` `actions!` macro, add `OpenSettings`. In the `bind_keys` list add:

```rust
KeyBinding::new("cmd-,", OpenSettings, None),
```

Add `mod settings_ui;` at the top. Add `settings: Option<settings_ui::SettingsUi>` to `ReviewApp` (init `None` in `ReviewApp::new`). Register the handler in `render` alongside the other `.on_action` handlers (mirror how `OpenPalette` is wired):

```rust
.on_action(cx.listener(|app, _: &OpenSettings, window, cx| {
    let font_filter = cx.new(|cx| InputState::new(window, cx).placeholder("filter fonts…"));
    app.settings = Some(settings_ui::SettingsUi { font_filter, focus: settings_ui::SettingsField::Theme });
    cx.notify();
}))
```

- [ ] **Step 2: Implement the modal**

Create `crates/app/src/settings_ui.rs`. It renders an overlay (backdrop + centered panel) modeled on `render_palette`. Controls:

- **Theme**: a row of buttons, one per `theme::all_names()`; the active one (matching `settings.theme_name`) is highlighted. Clicking sets `theme_name`, then `apply_and_save`.
- **UI font** and **Code font**: each a scrollable, fuzzy-filtered list of `cx.text_system().all_font_names()` (dedup + sort). Reuse `SkimMatcherV2` against the shared `font_filter` input text. Clicking a name sets `ui_font`/`code_font`. Show a one-line note under Code font: *"Use a monospace font — proportional fonts will misalign the diff."*
- **Font size**: `−` / value / `+` buttons calling `settings.set_font_size(size ± 1.0)`, plus display of the current value.
- **Reset to defaults**: sets the global to `Settings::default()`, then `apply_and_save`.
- **Dismiss**: an `escape`/close affordance that sets `app.settings = None` (bind `escape` within a `"Settings"` key context, or a close button; follow the palette's escape handling).

`apply_and_save` (free fn or method):

```rust
fn apply_and_save(cx: &mut Context<ReviewApp>, app: &mut ReviewApp) {
    let s = cx.global::<settings::Settings>().clone();
    theme::apply_ui_theme(&theme::by_name(&s.theme_name), cx);
    app.char_width = None; // force re-measure at new code font/size
    s.save();
    cx.notify();
}
```

> To mutate the global from a click handler: `cx.update_global::<settings::Settings, _>(|s, _| { s.theme_name = name; })` then call `apply_and_save`.

- [ ] **Step 3: Render the modal from `ReviewApp::render`**

Where the palette is conditionally rendered, add: if `self.settings.is_some()`, render `settings_ui::render_settings(&cx.entity(), window, cx)` as a top overlay (after the main content, like the palette overlay).

- [ ] **Step 4: Build**

Run: `cargo build -p lgtm`
Expected: builds clean.

- [ ] **Step 5: Manual verification (the real test)**

Run: `cargo run`. Then:
1. Press `cmd-,` → settings modal opens.
2. Switch theme Mocha → Latte → Tokyo Night: UI + syntax colors change live.
3. Change font size with `+`: diff text and chrome grow; **diff columns stay aligned and mouse selection still lands on the correct character** (this exercises the `char_width` reset + `row_height`).
4. Change code font to another monospace font (e.g. Monaco): diff re-renders aligned.
5. Change UI font: chrome font changes, diff pane unaffected.
6. Close modal (escape), quit, relaunch → all choices persisted (check `~/Library/Application Support/lgtm/config.toml` exists with the values).
7. Reset to defaults → back to Mocha/Menlo/13.

Expected: all of the above hold. If font size change misaligns the diff, `char_width` is not being reset or `row_height()` isn't used consistently — fix before committing.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/settings_ui.rs crates/app/src/main.rs
git commit -m "feat(settings): in-app settings modal (cmd-,)"
```

---

## Task 6: Docs

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the keymap and features**

Add to the keymap table:

```markdown
| `cmd-,` | open settings (theme / fonts / size) |
```

Add to Features:

```markdown
- settings: built-in themes, UI + code font, font size (`cmd-,`), persisted to config
```

- [ ] **Step 2: Commit**

```bash
git add README.md
git commit -m "docs: document settings (cmd-,)"
```

---

## Self-Review

**Spec coverage:**
- In-app settings UI (`cmd-,`) → Task 5. ✓
- Built-in themes now + external seam → Task 2 (`Theme` struct is the seam). ✓
- Two font families (UI + code) → Tasks 3 (code font in diff), 4 (UI font on chrome), 5 (pickers). ✓
- Single font size scaling everything → Task 1 (`scale`/`chrome`/`row_height`), Task 3 (diff), Task 4 (chrome). ✓
- Enumerate system fonts → Task 5 (`all_font_names()`). ✓
- Non-mono user-beware note → Task 5 note text. ✓
- Persistence as TOML in config dir, defaults on error → Task 1. ✓
- `char_width` invalidation on font/size change → Task 3 (accessor reads global) + Task 5 (`apply_and_save` resets it). ✓
- Default appearance byte-identical → Task 1 defaults + Task 2 Mocha verbatim + Task 3/4 smoke tests. ✓
- Docs → Task 6. ✓

**Placeholder scan:** No TBD/TODO in deliverable steps. The one deferred item (declare `mod settings_ui;` in Task 5) is an explicit ordering instruction, not a placeholder.

**Type consistency:** `Settings` fields (`theme_name`, `ui_font: Option<String>`, `code_font`, `font_size`) are used consistently across Tasks 1/3/4/5. `theme::by_name` / `apply_ui_theme(&Theme, cx)` / `token_style(&Theme, Token)` signatures are consistent between Task 2 (defined) and Task 3 (consumed). `Settings::chrome`/`scale`/`row_height`/`set_font_size` defined in Task 1 and consumed in 3/4/5.

**Known risk flagged in-plan:** `char_width` gaining a `cx`/`&App` parameter ripples to its call sites (Task 3, Step 2) — the plan tells the implementer to pass `cx` or hoist the `(code_font, size)` read, whichever compiles with the smallest change.
