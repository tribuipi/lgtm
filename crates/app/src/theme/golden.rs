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
        // font_weight 700 in the fixture resolves to bold; function omits it.
        assert!(t.syntax(Token::Keyword).bold);
        assert!(!t.syntax(Token::Function).bold);
        assert_eq!(t.syntax(Token::Function).color, rgb(0x7aa2f7));
        assert_eq!(t.syntax(Token::Comment).color, rgb(0x565f89));
        assert!(t.syntax(Token::Comment).italic);
        assert_eq!(t.syntax(Token::Parameter).color, rgb(0xe0af68));
        assert!(t.syntax(Token::Parameter).italic);
        // A token absent from the fixture falls back to plain text.
        assert_eq!(t.syntax(Token::Operator).color, t.text);
    }
}
