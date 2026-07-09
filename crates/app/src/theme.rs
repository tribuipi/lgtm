//! Catppuccin Mocha, hardcoded for now. Helix-theme loading comes later.

use gpui::{rgb, rgba, Rgba};

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

/// Low-alpha tints: syntax/text must stay readable on top. Never opaque.
pub fn added_row_bg() -> Rgba {
    rgba(0xa6e3a120)
}
pub fn removed_row_bg() -> Rgba {
    rgba(0xf38ba820)
}
pub fn added_word_bg() -> Rgba {
    rgba(0xa6e3a148)
}
pub fn removed_word_bg() -> Rgba {
    rgba(0xf38ba848)
}
