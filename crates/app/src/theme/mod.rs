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
pub use registry::{discover, theme_dirs, ThemeRegistry};

mod embedded;
mod model;
mod registry;
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

/// Syntax highlight for one token, read from the resolved theme's syntax map.
pub fn token_style(theme: &Theme, token: Token) -> HighlightStyle {
    let s = theme.syntax(token);
    HighlightStyle {
        color: Some(s.color.into()),
        font_style: s.italic.then_some(FontStyle::Italic),
        ..Default::default()
    }
}

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
