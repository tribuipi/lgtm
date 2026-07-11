//! Data-driven themes. A `Theme` is a struct of palette colors + a syntax
//! lookup; built-in constructors produce concrete themes. This struct is the
//! seam a future external-theme loader (e.g. Helix .toml) targets.
//!
//! The bare `base()`/`mantle()`/.../`peach()` accessors below predate this
//! refactor and stay hardcoded to Catppuccin Mocha: they back the many
//! decorative UI colors (badges, chat, titlebar, sidebar, …) that this task
//! did not wire to the active theme. Only `apply_ui_theme`, `token_style`,
//! and the tint helpers below are theme-aware; re-theming the rest of the
//! app's chrome is left for a follow-up task.

use gpui::{rgb, rgba, App, FontStyle, HighlightStyle, Hsla, Rgba};
use gpui_component::{Theme as UiTheme, ThemeMode};
use syntax::Token;

pub fn base() -> Rgba {
    rgb(0x1e1e2e)
}
pub fn mantle() -> Rgba {
    rgb(0x181825)
}
pub fn crust() -> Rgba {
    rgb(0x11111b)
}
pub fn surface0() -> Rgba {
    rgb(0x313244)
}
pub fn text() -> Rgba {
    rgb(0xcdd6f4)
}
pub fn subtext() -> Rgba {
    rgb(0xa6adc8)
}
pub fn overlay0() -> Rgba {
    rgb(0x6c7086)
}
pub fn green() -> Rgba {
    rgb(0xa6e3a1)
}
pub fn red() -> Rgba {
    rgb(0xf38ba8)
}
pub fn blue() -> Rgba {
    rgb(0x89b4fa)
}
pub fn mauve() -> Rgba {
    rgb(0xcba6f7)
}
pub fn peach() -> Rgba {
    rgb(0xfab387)
}

/// A named palette + syntax mapping. Built-ins are small literals; a future
/// external-theme loader (e.g. Helix .toml) would produce more of these at
/// runtime.
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
        Token::Keyword => (0xcba6f7, false),                  // mauve
        Token::Function => (0x89b4fa, false),                 // blue
        Token::Type => (0xf9e2af, false),                     // yellow
        Token::String => (0xa6e3a1, false),                   // green
        Token::Number | Token::Constant => (0xfab387, false), // peach
        Token::Comment => (0x6c7086, true),                   // overlay0
        Token::Property => (0xb4befe, false),                 // lavender
        Token::Variable | Token::Embedded => (0xcdd6f4, false), // text
        Token::Parameter => (0xeba0ac, true),                 // maroon
        Token::Operator => (0x89dceb, false),                 // sky
        Token::Punctuation => (0x9399b2, false),              // overlay2
        Token::Attribute | Token::Label => (0xf9e2af, false), // yellow
        Token::Namespace => (0xfab387, true),                 // peach
    }
}

fn latte_syntax(token: Token) -> (u32, bool) {
    match token {
        Token::Keyword => (0x8839ef, false),                  // mauve
        Token::Function => (0x1e66f5, false),                 // blue
        Token::Type => (0xdf8e1d, false),                     // yellow
        Token::String => (0x40a02b, false),                   // green
        Token::Number | Token::Constant => (0xfe640b, false), // peach
        Token::Comment => (0x9ca0b0, true),                   // overlay0
        Token::Property => (0x7287fd, false),                 // lavender
        Token::Variable | Token::Embedded => (0x4c4f69, false), // text
        Token::Parameter => (0xe64553, true),                 // maroon
        Token::Operator => (0x04a5e5, false),                 // sky
        Token::Punctuation => (0x6c6f85, false),              // subtext
        Token::Attribute | Token::Label => (0xdf8e1d, false), // yellow
        Token::Namespace => (0xfe640b, true),                 // peach
    }
}

fn tokyo_syntax(token: Token) -> (u32, bool) {
    match token {
        Token::Keyword => (0xbb9af7, false),                  // mauve/purple
        Token::Function => (0x7aa2f7, false),                 // blue
        Token::Type => (0xe0af68, false),                     // yellow
        Token::String => (0x9ece6a, false),                   // green
        Token::Number | Token::Constant => (0xff9e64, false), // peach/orange
        Token::Comment => (0x565f89, true),                   // overlay0
        Token::Property => (0x7dcfff, false),                 // lavender/cyan
        Token::Variable | Token::Embedded => (0xc0caf5, false), // text
        Token::Parameter => (0xf7768e, true),                 // maroon/red
        Token::Operator => (0x89ddff, false),                 // sky
        Token::Punctuation => (0xa9b1d6, false),              // subtext
        Token::Attribute | Token::Label => (0xe0af68, false), // yellow
        Token::Namespace => (0xff9e64, true),                 // peach
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
        syntax: mocha_syntax,
    }
}

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
        syntax: latte_syntax,
    }
}

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
        syntax: tokyo_syntax,
    }
}

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

/// Override gpui-component's theme (dark mode, default shadcn palette) with
/// the given `Theme`. Call after `gpui_component::init`.
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

/// Syntax highlight for one tree-sitter token, per the active theme. Variable
/// and Embedded map to the plain text color (syntax spans for them are not
/// emitted, so this is belt-and-braces).
pub fn token_style(theme: &Theme, token: Token) -> HighlightStyle {
    let (color, italic) = (theme.syntax)(token);
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
