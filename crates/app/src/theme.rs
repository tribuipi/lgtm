//! Data-driven themes. A `Theme` is a flat struct of palette colors (UI +
//! syntax accents); built-in constructors produce concrete themes. This
//! struct is the seam a future external-theme loader (e.g. Helix .toml)
//! targets.
//!
//! The bare `base()`/`mantle()`/.../`peach()` accessors below predate this
//! refactor. They now read a thread-local "active theme" cell that
//! `apply_ui_theme` installs, so the many decorative UI colors (badges,
//! chat, titlebar, sidebar, …) that call them follow whatever theme is
//! currently applied. Before the first `apply_ui_theme`/`set_active_theme`
//! call on a thread, the cell defaults to Catppuccin Mocha.

use gpui::{rgb, rgba, App, FontStyle, HighlightStyle, Hsla, Rgba};
use gpui_component::{Theme as UiTheme, ThemeMode};
use std::cell::RefCell;
use syntax::Token;

// Invariant: any change to `settings.theme_name` MUST be followed by
// `apply_ui_theme(&by_name(theme_name))` in the same synchronous step, or
// these bare accessors (backed by `ACTIVE`) will diverge from the
// syntax/tint colors that render code derives inline via `by_name` — the
// two are only kept in sync because every mutation path currently honors
// this ordering (see `settings_ui::apply_and_save`).
thread_local! {
    /// The palette the bare accessors below read from. Set by
    /// `apply_ui_theme` (and directly in tests). Defaults to Mocha so any
    /// read before the first apply matches today's hardcoded behavior.
    static ACTIVE: RefCell<Theme> = RefCell::new(catppuccin_mocha());
}

/// Install `theme` as the palette the bare color accessors return. Called by
/// `apply_ui_theme`; exposed for tests that have no `App`.
///
/// Invariant: callers MUST also ensure `settings.theme_name` is (or already
/// was) updated to match `theme` in the same synchronous step — see the
/// module-level comment above `ACTIVE`.
pub(crate) fn set_active_theme(theme: &Theme) {
    ACTIVE.with(|a| *a.borrow_mut() = theme.clone());
}

/// The palette currently backing the bare color accessors.
fn active() -> Theme {
    ACTIVE.with(|a| a.borrow().clone())
}

pub fn base() -> Rgba {
    rgb(active().base)
}
pub fn mantle() -> Rgba {
    rgb(active().mantle)
}
pub fn crust() -> Rgba {
    rgb(active().crust)
}
pub fn surface0() -> Rgba {
    rgb(active().surface0)
}
pub fn text() -> Rgba {
    rgb(active().text)
}
pub fn subtext() -> Rgba {
    rgb(active().subtext)
}
pub fn overlay0() -> Rgba {
    rgb(active().overlay0)
}
pub fn green() -> Rgba {
    rgb(active().green)
}
pub fn red() -> Rgba {
    rgb(active().red)
}
pub fn blue() -> Rgba {
    rgb(active().blue)
}
pub fn mauve() -> Rgba {
    rgb(active().mauve)
}
pub fn peach() -> Rgba {
    rgb(active().peach)
}

/// A named palette. The first block of fields drives the UI chrome
/// (`apply_ui_theme` + the bare accessors above); the trailing accent fields
/// (`yellow`/`lavender`/`maroon`/`sky`/`overlay2`) exist only to give
/// `token_style` a full syntax palette. Field names follow Catppuccin's
/// vocabulary; for other themes they simply hold that theme's color for the
/// same *role* (e.g. `mauve` is always the keyword color).
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
    pub yellow: u32,
    pub lavender: u32,
    pub maroon: u32,
    pub sky: u32,
    pub overlay2: u32,
}

// ---- Catppuccin (https://catppuccin.com) ----

pub fn catppuccin_latte() -> Theme {
    Theme {
        name: "Catppuccin Latte",
        mode: ThemeMode::Light,
        base: 0xeff1f5,
        mantle: 0xe6e9ef,
        crust: 0xdce0e8,
        surface0: 0xccd0da,
        text: 0x4c4f69,
        subtext: 0x6c6f85,
        overlay0: 0x9ca0b0,
        green: 0x40a02b,
        red: 0xd20f39,
        blue: 0x1e66f5,
        mauve: 0x8839ef,
        peach: 0xfe640b,
        yellow: 0xdf8e1d,
        lavender: 0x7287fd,
        maroon: 0xe64553,
        sky: 0x04a5e5,
        overlay2: 0x6c6f85,
    }
}

pub fn catppuccin_frappe() -> Theme {
    Theme {
        name: "Catppuccin Frappé",
        mode: ThemeMode::Dark,
        base: 0x303446,
        mantle: 0x292c3c,
        crust: 0x232634,
        surface0: 0x414559,
        text: 0xc6d0f5,
        subtext: 0xa5adce,
        overlay0: 0x737994,
        green: 0xa6d189,
        red: 0xe78284,
        blue: 0x8caaee,
        mauve: 0xca9ee6,
        peach: 0xef9f76,
        yellow: 0xe5c890,
        lavender: 0xbabbf1,
        maroon: 0xea999c,
        sky: 0x99d1db,
        overlay2: 0x949cbb,
    }
}

pub fn catppuccin_macchiato() -> Theme {
    Theme {
        name: "Catppuccin Macchiato",
        mode: ThemeMode::Dark,
        base: 0x24273a,
        mantle: 0x1e2030,
        crust: 0x181926,
        surface0: 0x363a4f,
        text: 0xcad3f5,
        subtext: 0xa5adcb,
        overlay0: 0x6e738d,
        green: 0xa6da95,
        red: 0xed8796,
        blue: 0x8aadf4,
        mauve: 0xc6a0f6,
        peach: 0xf5a97f,
        yellow: 0xeed49f,
        lavender: 0xb7bdf8,
        maroon: 0xee99a0,
        sky: 0x91d7e3,
        overlay2: 0x939ab7,
    }
}

pub fn catppuccin_mocha() -> Theme {
    Theme {
        name: "Catppuccin Mocha",
        mode: ThemeMode::Dark,
        base: 0x1e1e2e,
        mantle: 0x181825,
        crust: 0x11111b,
        surface0: 0x313244,
        text: 0xcdd6f4,
        subtext: 0xa6adc8,
        overlay0: 0x6c7086,
        green: 0xa6e3a1,
        red: 0xf38ba8,
        blue: 0x89b4fa,
        mauve: 0xcba6f7,
        peach: 0xfab387,
        yellow: 0xf9e2af,
        lavender: 0xb4befe,
        maroon: 0xeba0ac,
        sky: 0x89dceb,
        overlay2: 0x9399b2,
    }
}

// ---- GitHub (Primer, https://primer.style) ----

pub fn github_light() -> Theme {
    Theme {
        name: "GitHub Light",
        mode: ThemeMode::Light,
        base: 0xffffff,
        mantle: 0xf6f8fa,
        crust: 0xeaeef2,
        surface0: 0xd0d7de,
        text: 0x1f2328,
        subtext: 0x656d76,
        overlay0: 0x6e7781,
        green: 0x1a7f37,
        red: 0xcf222e,
        blue: 0x0969da,
        mauve: 0xcf222e, // keyword: GitHub red
        peach: 0xbc4c00,
        yellow: 0x953800,
        lavender: 0x0550ae,
        maroon: 0x953800,
        sky: 0x0550ae,
        overlay2: 0x656d76,
    }
}

pub fn github_dark() -> Theme {
    Theme {
        name: "GitHub Dark",
        mode: ThemeMode::Dark,
        base: 0x0d1117,
        mantle: 0x161b22,
        crust: 0x010409,
        surface0: 0x30363d,
        text: 0xe6edf3,
        subtext: 0x7d8590,
        overlay0: 0x6e7681,
        green: 0x3fb950,
        red: 0xf85149,
        blue: 0x58a6ff,
        mauve: 0xff7b72, // keyword: GitHub coral
        peach: 0xffa657,
        yellow: 0xf0883e,
        lavender: 0x79c0ff,
        maroon: 0xffa657,
        sky: 0x79c0ff,
        overlay2: 0x7d8590,
    }
}

pub fn github_dark_dimmed() -> Theme {
    Theme {
        name: "GitHub Dark Dimmed",
        mode: ThemeMode::Dark,
        base: 0x22272e,
        mantle: 0x2d333b,
        crust: 0x1c2128,
        surface0: 0x373e47,
        text: 0xadbac7,
        subtext: 0x768390,
        overlay0: 0x636e7b,
        green: 0x6bc46d,
        red: 0xf47067,
        blue: 0x539bf5,
        mauve: 0xf47067,
        peach: 0xf69d50,
        yellow: 0xdaaa3f,
        lavender: 0x6cb6ff,
        maroon: 0xf69d50,
        sky: 0x6cb6ff,
        overlay2: 0x768390,
    }
}

// ---- Tokyo Night (https://github.com/folke/tokyonight.nvim) ----

pub fn tokyo_night() -> Theme {
    Theme {
        name: "Tokyo Night",
        mode: ThemeMode::Dark,
        base: 0x1a1b26,
        mantle: 0x16161e,
        crust: 0x13131a,
        surface0: 0x292e42,
        text: 0xc0caf5,
        subtext: 0xa9b1d6,
        overlay0: 0x565f89,
        green: 0x9ece6a,
        red: 0xf7768e,
        blue: 0x7aa2f7,
        mauve: 0xbb9af7,
        peach: 0xff9e64,
        yellow: 0xe0af68,
        lavender: 0x7dcfff,
        maroon: 0xf7768e,
        sky: 0x89ddff,
        overlay2: 0xa9b1d6,
    }
}

pub fn tokyo_night_storm() -> Theme {
    Theme {
        name: "Tokyo Night Storm",
        mode: ThemeMode::Dark,
        base: 0x24283b,
        mantle: 0x1f2335,
        crust: 0x1b1e2e,
        surface0: 0x2f334d,
        text: 0xc0caf5,
        subtext: 0xa9b1d6,
        overlay0: 0x565f89,
        green: 0x9ece6a,
        red: 0xf7768e,
        blue: 0x7aa2f7,
        mauve: 0xbb9af7,
        peach: 0xff9e64,
        yellow: 0xe0af68,
        lavender: 0x7dcfff,
        maroon: 0xf7768e,
        sky: 0x89ddff,
        overlay2: 0xa9b1d6,
    }
}

pub fn tokyo_night_moon() -> Theme {
    Theme {
        name: "Tokyo Night Moon",
        mode: ThemeMode::Dark,
        base: 0x222436,
        mantle: 0x1e2030,
        crust: 0x191a2a,
        surface0: 0x2f334d,
        text: 0xc8d3f5,
        subtext: 0xa9b8e8,
        overlay0: 0x636da6,
        green: 0xc3e88d,
        red: 0xff757f,
        blue: 0x82aaff,
        mauve: 0xc099ff,
        peach: 0xff966c,
        yellow: 0xffc777,
        lavender: 0x86e1fc,
        maroon: 0xff757f,
        sky: 0x86e1fc,
        overlay2: 0x828bb8,
    }
}

pub fn tokyo_night_light() -> Theme {
    Theme {
        name: "Tokyo Night Light",
        mode: ThemeMode::Light,
        base: 0xe1e2e7,
        mantle: 0xd6d8df,
        crust: 0xc9ccd6,
        surface0: 0xc8cede,
        text: 0x3760bf,
        subtext: 0x6172b0,
        overlay0: 0x848cb5,
        green: 0x587539,
        red: 0xf52a65,
        blue: 0x2e7de9,
        mauve: 0x9854f1,
        peach: 0xb15c00,
        yellow: 0x8c6c3e,
        lavender: 0x7847bd,
        maroon: 0xf52a65,
        sky: 0x007197,
        overlay2: 0x848cb5,
    }
}

// ---- Solarized (https://ethanschoonover.com/solarized) ----

pub fn solarized_light() -> Theme {
    Theme {
        name: "Solarized Light",
        mode: ThemeMode::Light,
        base: 0xfdf6e3,   // base3
        mantle: 0xeee8d5, // base2
        crust: 0xe3dcc6,
        surface0: 0xcfc8b0,
        text: 0x657b83,     // base00
        subtext: 0x586e75,  // base01
        overlay0: 0x93a1a1, // base1 (comments)
        green: 0x859900,
        red: 0xdc322f,
        blue: 0x268bd2,
        mauve: 0x6c71c4, // violet (keyword)
        peach: 0xcb4b16, // orange
        yellow: 0xb58900,
        lavender: 0x6c71c4,
        maroon: 0xcb4b16,
        sky: 0x2aa198, // cyan
        overlay2: 0x93a1a1,
    }
}

pub fn solarized_dark() -> Theme {
    Theme {
        name: "Solarized Dark",
        mode: ThemeMode::Dark,
        base: 0x002b36,   // base03
        mantle: 0x073642, // base02
        crust: 0x00212b,
        surface0: 0x073642,
        text: 0x839496,     // base0
        subtext: 0x93a1a1,  // base1
        overlay0: 0x586e75, // base01 (comments)
        green: 0x859900,
        red: 0xdc322f,
        blue: 0x268bd2,
        mauve: 0x6c71c4,
        peach: 0xcb4b16,
        yellow: 0xb58900,
        lavender: 0x6c71c4,
        maroon: 0xcb4b16,
        sky: 0x2aa198,
        overlay2: 0x586e75,
    }
}

/// Built-in theme names, in picker display order (grouped by family).
pub fn all_names() -> &'static [&'static str] {
    &[
        "Catppuccin Latte",
        "Catppuccin Frappé",
        "Catppuccin Macchiato",
        "Catppuccin Mocha",
        "GitHub Light",
        "GitHub Dark",
        "GitHub Dark Dimmed",
        "Tokyo Night",
        "Tokyo Night Storm",
        "Tokyo Night Moon",
        "Tokyo Night Light",
        "Solarized Light",
        "Solarized Dark",
    ]
}

/// Resolve a stored theme name to its palette; unknown names fall back to the
/// default (Catppuccin Mocha).
pub fn by_name(name: &str) -> Theme {
    match name {
        "Catppuccin Mocha" => catppuccin_mocha(),
        "Catppuccin Latte" => catppuccin_latte(),
        "Catppuccin Frappé" => catppuccin_frappe(),
        "Catppuccin Macchiato" => catppuccin_macchiato(),
        "GitHub Light" => github_light(),
        "GitHub Dark" => github_dark(),
        "GitHub Dark Dimmed" => github_dark_dimmed(),
        "Tokyo Night" => tokyo_night(),
        "Tokyo Night Storm" => tokyo_night_storm(),
        "Tokyo Night Moon" => tokyo_night_moon(),
        "Tokyo Night Light" => tokyo_night_light(),
        "Solarized Light" => solarized_light(),
        "Solarized Dark" => solarized_dark(),
        _ => catppuccin_mocha(),
    }
}

/// Override gpui-component's theme (dark mode, default shadcn palette) with
/// the given `Theme`. Call after `gpui_component::init`.
pub fn apply_ui_theme(theme: &Theme, cx: &mut App) {
    set_active_theme(theme);
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
    t.background = base;
    t.foreground = text;
    t.muted = surface0;
    t.muted_foreground = overlay0;
    t.border = surface0;
    t.input = surface0;
    t.ring = blue;
    t.primary = blue;
    t.primary_hover = blue.opacity(0.9);
    t.primary_active = blue.opacity(0.8);
    t.primary_foreground = crust;
    t.secondary = surface0;
    t.secondary_hover = surface0.opacity(0.8);
    t.secondary_active = surface0.opacity(0.6);
    t.secondary_foreground = text;
    t.accent = surface0;
    t.accent_foreground = text;
    t.danger = red;
    t.danger_hover = red.opacity(0.9);
    t.danger_active = red.opacity(0.8);
    t.danger_foreground = crust;
    t.success = green;
    t.success_hover = green.opacity(0.9);
    t.success_active = green.opacity(0.8);
    t.success_foreground = crust;
    t.warning = peach;
    t.warning_hover = peach.opacity(0.9);
    t.warning_active = peach.opacity(0.8);
    t.warning_foreground = crust;
    t.info = blue;
    t.info_hover = blue.opacity(0.9);
    t.info_active = blue.opacity(0.8);
    t.info_foreground = crust;
    t.link = blue;
    t.link_hover = blue.opacity(0.9);
    t.link_active = blue.opacity(0.8);
    t.popover = mantle;
    t.popover_foreground = text;
    t.title_bar = mantle;
    t.title_bar_border = surface0;
    t.sidebar = mantle;
    t.sidebar_foreground = text;
    t.sidebar_border = surface0;
    t.caret = text;
    t.selection = blue.opacity(0.3);
    t.scrollbar = crust.opacity(0.6);
    t.scrollbar_thumb = overlay0.opacity(0.5);
    t.scrollbar_thumb_hover = overlay0;
    t.window_border = surface0;
}

/// Syntax highlight for one tree-sitter token, per the given theme. The
/// token→color mapping is shared across all themes; each theme supplies the
/// colors via its palette fields. Variable and Embedded map to the plain text
/// color (syntax spans for them are not emitted, so this is belt-and-braces).
pub fn token_style(theme: &Theme, token: Token) -> HighlightStyle {
    let (color, italic) = match token {
        Token::Keyword => (theme.mauve, false),
        Token::Function => (theme.blue, false),
        Token::Type => (theme.yellow, false),
        Token::String => (theme.green, false),
        Token::Number | Token::Constant => (theme.peach, false),
        Token::Comment => (theme.overlay0, true),
        Token::Property => (theme.lavender, false),
        Token::Variable | Token::Embedded => (theme.text, false),
        Token::Parameter => (theme.maroon, true),
        Token::Operator => (theme.sky, false),
        Token::Punctuation => (theme.overlay2, false),
        Token::Attribute | Token::Label => (theme.yellow, false),
        Token::Namespace => (theme.peach, true),
    };
    HighlightStyle {
        color: Some(rgb(color).into()),
        font_style: italic.then_some(FontStyle::Italic),
        ..Default::default()
    }
}

/// Dimming layer behind the command palette.
pub fn palette_backdrop(theme: &Theme) -> Rgba {
    rgba((theme.crust << 8) | 0xaa)
}

/// Split view: background for the absent side of a one-sided row — darker
/// than any content row so it clearly reads as "nothing here".
pub fn void_cell_bg(theme: &Theme) -> Rgba {
    rgba((theme.crust << 8) | 0x99)
}

/// Text selection in the diff pane — same blue.opacity(0.3) injected into the
/// gpui-component theme as `selection` in `apply_ui_theme`.
pub fn selection_bg(theme: &Theme) -> Rgba {
    rgba((theme.blue << 8) | 0x4d)
}

/// Low-alpha tints: syntax/text must stay readable on top. Never opaque.
pub fn added_row_bg(theme: &Theme) -> Rgba {
    rgba((theme.green << 8) | 0x20)
}
pub fn removed_row_bg(theme: &Theme) -> Rgba {
    rgba((theme.red << 8) | 0x20)
}
pub fn added_word_bg(theme: &Theme) -> Rgba {
    rgba((theme.green << 8) | 0x48)
}
pub fn removed_word_bg(theme: &Theme) -> Rgba {
    rgba((theme.red << 8) | 0x48)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syntax::Token;

    #[test]
    fn by_name_resolves_builtins() {
        // Every advertised name resolves to a theme carrying that same name.
        for &name in all_names() {
            assert_eq!(by_name(name).name, name, "by_name({name:?}) mismatch");
        }
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
        // Shared mapping must reproduce the old hardcoded Mocha syntax: keyword
        // is mauve + not italic; comment is overlay0 + italic.
        let kw = token_style(&t, Token::Keyword);
        assert_eq!(kw.color, Some(rgb(0xcba6f7).into()));
        assert_eq!(kw.font_style, None);
        let comment = token_style(&t, Token::Comment);
        assert_eq!(comment.color, Some(rgb(0x6c7086).into()));
        assert_eq!(comment.font_style, Some(FontStyle::Italic));
    }

    #[test]
    fn all_names_lists_every_builtin() {
        let names = all_names();
        assert_eq!(names.len(), 13);
        for expected in [
            "Catppuccin Mocha",
            "Catppuccin Latte",
            "Catppuccin Frappé",
            "Catppuccin Macchiato",
            "GitHub Light",
            "GitHub Dark",
            "GitHub Dark Dimmed",
            "Tokyo Night",
            "Tokyo Night Storm",
            "Tokyo Night Moon",
            "Tokyo Night Light",
            "Solarized Light",
            "Solarized Dark",
        ] {
            assert!(names.contains(&expected), "missing {expected:?}");
        }
    }

    #[test]
    fn theme_modes_are_correct() {
        assert!(matches!(catppuccin_mocha().mode, ThemeMode::Dark));
        assert!(matches!(catppuccin_latte().mode, ThemeMode::Light));
        assert!(matches!(github_light().mode, ThemeMode::Light));
        assert!(matches!(github_dark().mode, ThemeMode::Dark));
        assert!(matches!(tokyo_night_light().mode, ThemeMode::Light));
        assert!(matches!(solarized_dark().mode, ThemeMode::Dark));
        assert!(matches!(solarized_light().mode, ThemeMode::Light));
    }

    #[test]
    fn muted_bg_differs_from_muted_foreground() {
        // muted = surface0, muted_foreground = overlay0 (see apply_ui_theme).
        // If they were equal, muted text would be invisible.
        for &name in all_names() {
            let t = by_name(name);
            assert_ne!(t.surface0, t.overlay0, "{name}: surface0 == overlay0");
        }
    }

    #[test]
    fn accessors_default_to_mocha_without_apply() {
        // No set_active_theme call on this test's thread → Mocha default.
        assert_eq!(base(), rgb(0x1e1e2e));
        assert_eq!(text(), rgb(0xcdd6f4));
        assert_eq!(green(), rgb(0xa6e3a1));
    }

    #[test]
    fn accessors_follow_active_theme() {
        set_active_theme(&catppuccin_latte());
        assert_eq!(base(), rgb(0xeff1f5)); // latte base
        assert_eq!(text(), rgb(0x4c4f69)); // latte text
        set_active_theme(&tokyo_night());
        assert_eq!(green(), rgb(0x9ece6a)); // tokyo green
        assert_eq!(base(), rgb(0x1a1b26)); // tokyo base
    }
}
