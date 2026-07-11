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
