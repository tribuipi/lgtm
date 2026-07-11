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
        let bold = raw.and_then(|s| s.font_weight).map(|w| w >= 600.0).unwrap_or(false);
        syntax.insert(token, SyntaxStyle { color, italic, bold });
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
            r##"{ "background": "#101010", "editor.background": "#202020",
                 "text": "#f0f0f0", "text.accent": "#3366ff", "border": "#303030" }"##,
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
        let only_editor = resolve("X", Appearance::Dark, &style(r##"{ "editor.background": "#123456" }"##));
        assert_eq!(only_editor.background, rgb(0x123456));
        let only_bg = resolve("X", Appearance::Dark, &style(r##"{ "background": "#654321" }"##));
        assert_eq!(only_bg.editor_bg, rgb(0x654321));
    }

    #[test]
    fn text_from_editor_foreground_and_muted_tiers_derived() {
        let t = resolve("X", Appearance::Dark, &style(r##"{ "editor.foreground": "#ffffff", "editor.background": "#000000" }"##));
        assert_eq!(t.text, rgb(0xffffff));
        // Muted tiers are between text and background, and distinct from each other.
        assert_ne!(t.text_muted, t.text);
        assert_ne!(t.text_subtle, t.text);
        assert_ne!(t.text_muted, t.text_subtle);
    }

    #[test]
    fn status_roles_cross_fill_then_default() {
        // created present, success absent → success mirrors created.
        let t = resolve("X", Appearance::Dark, &style(r##"{ "created": "#00ff00" }"##));
        assert_eq!(t.success, rgb(0x00ff00));
        // Nothing present → appearance default (non-zero, opaque).
        let empty = resolve("X", Appearance::Dark, &style("{}"));
        assert_eq!(empty.created.a, 1.0);
        assert_ne!(empty.error, empty.created);
    }

    #[test]
    fn accent_falls_back_to_first_player_cursor() {
        let t = resolve("X", Appearance::Dark, &style(r##"{ "players": [ { "cursor": "#abcdef" } ] }"##));
        assert_eq!(t.accent, rgb(0xabcdef));
    }

    #[test]
    fn every_token_resolves_and_comment_defaults_to_subtle() {
        // No syntax map at all → every token filled; comment uses text_subtle.
        let t = resolve("X", Appearance::Dark, &style(r##"{ "text": "#eeeeee" }"##));
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
            &style(r##"{ "syntax": { "comment": { "color": "#777777", "font_style": "italic" },
                                    "keyword": { "color": "#ff00ff" } } }"##),
        );
        assert!(t.syntax(Token::Comment).italic);
        assert_eq!(t.syntax(Token::Comment).color, rgb(0x777777));
        assert!(!t.syntax(Token::Keyword).italic);
    }

    #[test]
    fn syntax_bold_from_font_weight() {
        let t = resolve(
            "X",
            Appearance::Dark,
            &style(r##"{ "syntax": { "keyword": { "color": "#ff00ff", "font_weight": 700 },
                                    "function": { "color": "#00ff00" } } }"##),
        );
        // A weight >= 600 is bold; an omitted weight is not.
        assert!(t.syntax(Token::Keyword).bold);
        assert!(!t.syntax(Token::Function).bold);
    }
}
