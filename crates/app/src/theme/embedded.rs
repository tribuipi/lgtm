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
