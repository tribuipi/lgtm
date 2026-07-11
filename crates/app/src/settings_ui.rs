//! In-app settings modal, opened with `cmd-,` (see `OpenSettings` in
//! main.rs). Lets the user swap theme, UI font, code font, and font size;
//! every change is applied live and persisted immediately.
//!
//! Kept out of main.rs to bound that file's growth. `render_settings` is
//! implemented as a method on `ReviewApp` below (in an `impl` block, same
//! type as `render_review`/`render_palette`) so it can reach the private
//! fields (`char_width`, `focus_handle`, `settings`, …) those methods do —
//! this submodule is a descendant of the crate root where `ReviewApp` is
//! defined, so private-to-crate-root items are visible here.

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use gpui::{div, prelude::*, px, Context, Entity, Hsla, MouseButton, SharedString, Window};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{Escape as InputEscape, Input, InputState},
    Sizable as _,
};

use crate::{settings, theme, ReviewApp};

/// Only a handful of font-name rows are ever painted at once (see
/// `filtered_fonts`), so a plain scrolling `div` is plenty; no need for
/// `uniform_list`.
const MAX_FONT_ROWS: usize = 30;
const ROW_H: f32 = 26.0;

/// Which section the shared `font_filter` input currently narrows. Theme
/// has only three options, so it ignores the filter text entirely.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsField {
    Theme,
    UiFont,
    CodeFont,
}

pub struct SettingsUi {
    pub font_filter: Entity<InputState>,
    pub focus: SettingsField,
}

/// Every font name the text system knows about, deduped and sorted for
/// stable, readable display.
pub fn all_font_names(cx: &gpui::App) -> Vec<String> {
    let mut names = cx.text_system().all_font_names();
    names.sort();
    names.dedup();
    names
}

/// Font names fuzzy-matched against `query`, best score first. An empty
/// query keeps every name in its given (already sorted) order. Mirrors
/// `fuzzy_file_matches` in main.rs.
pub fn filtered_fonts<'a>(names: &'a [String], query: &str) -> Vec<&'a str> {
    let query = query.trim();
    if query.is_empty() {
        return names.iter().map(String::as_str).collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &str)> = names
        .iter()
        .filter_map(|name| {
            matcher
                .fuzzy_match(name, query)
                .map(|score| (score, name.as_str()))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    scored.into_iter().map(|(_, name)| name).collect()
}

/// Theme names fuzzy-matched against `query`; same behavior as
/// `filtered_fonts`, specialized for the tiny static theme list so the
/// shared filter input does something sensible on every tab.
fn filtered_themes(query: &str) -> Vec<&'static str> {
    let names = theme::all_names();
    let query = query.trim();
    if query.is_empty() {
        return names.to_vec();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &'static str)> = names
        .iter()
        .filter_map(|&name| matcher.fuzzy_match(name, query).map(|score| (score, name)))
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(b.1)));
    scored.into_iter().map(|(_, name)| name).collect()
}

/// After any settings mutation (theme/font/size change or reset): re-apply
/// the (possibly new) theme so the UI + syntax colors follow it live, force
/// a re-measure of the monospace cell (code font or size may have changed —
/// stale `char_width` would misalign diff columns and mouse selection),
/// persist to disk, and repaint. Safe to call even when nothing changed.
pub fn apply_and_save(app: &mut ReviewApp, cx: &mut Context<ReviewApp>) {
    let s = cx.global::<settings::Settings>().clone();
    theme::apply_ui_theme(&theme::by_name(&s.theme_name), cx);
    app.char_width = None;
    s.save();
    cx.notify();
}

/// One clickable tab in the section switcher (Theme / UI Font / Code Font).
fn tab(
    label: &'static str,
    active: bool,
    cx: &mut Context<ReviewApp>,
    on_click: impl Fn(&mut ReviewApp, &mut Window, &mut Context<ReviewApp>) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .when(active, |el| {
            el.bg(Hsla::from(theme::surface0()))
                .text_color(theme::text())
        })
        .when(!active, |el| {
            el.text_color(theme::subtext())
                .hover(|style| style.bg(Hsla::from(theme::surface0()).opacity(0.5)))
        })
        .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
        .child(SharedString::from(label))
}

/// One row in a fuzzy font list: highlighted when it matches `active`.
fn font_row(
    name: &str,
    active: bool,
    cx: &mut Context<ReviewApp>,
    on_click: impl Fn(&mut ReviewApp, &mut Window, &mut Context<ReviewApp>) + 'static,
) -> impl IntoElement {
    let owned = name.to_string();
    div()
        .id(SharedString::from(format!("font-row-{name}")))
        .h(px(ROW_H))
        .px_2()
        .flex()
        .items_center()
        .rounded_md()
        .cursor_pointer()
        .when(active, |row| {
            row.bg(Hsla::from(theme::surface0()))
                .text_color(theme::text())
        })
        .when(!active, |row| {
            row.text_color(theme::subtext())
                .hover(|style| style.bg(Hsla::from(theme::surface0()).opacity(0.5)))
        })
        .on_click(cx.listener(move |this, _, window, cx| on_click(this, window, cx)))
        .child(SharedString::from(owned))
}

impl ReviewApp {
    /// Switch which section the shared font-filter input narrows, clearing
    /// and refocusing it so Escape keeps routing through it (see the
    /// `InputEscape` handler on the modal card below).
    fn switch_settings_focus(
        &mut self,
        focus: SettingsField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(ui) = &mut self.settings else { return };
        ui.focus = focus;
        let input = ui.font_filter.clone();
        input.update(cx, |state, cx| {
            state.set_value("", window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// The settings modal: a dimming backdrop (click closes) over a centered
    /// card, modeled on `render_review`. Root-level so its input escapes the
    /// "ReviewApp" key context, same reasoning as the composer/review/palette.
    pub(crate) fn render_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(ui) = &self.settings else {
            return div().into_any_element();
        };
        let focus = ui.focus;
        let font_filter = ui.font_filter.clone();
        let query = font_filter.read(cx).value().to_string();
        let s = cx.global::<settings::Settings>().clone();
        let active_theme_name = theme::by_name(&s.theme_name).name;

        let close = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            this.settings = None;
            window.focus(&this.focus_handle);
            cx.notify();
        };

        // Section switcher.
        let tabs = div()
            .flex()
            .items_center()
            .gap_1()
            .child(tab(
                "Theme",
                focus == SettingsField::Theme,
                cx,
                |this, window, cx| {
                    this.switch_settings_focus(SettingsField::Theme, window, cx);
                },
            ))
            .child(tab(
                "UI Font",
                focus == SettingsField::UiFont,
                cx,
                |this, window, cx| {
                    this.switch_settings_focus(SettingsField::UiFont, window, cx);
                },
            ))
            .child(tab(
                "Code Font",
                focus == SettingsField::CodeFont,
                cx,
                |this, window, cx| {
                    this.switch_settings_focus(SettingsField::CodeFont, window, cx);
                },
            ));

        // Section body: theme swatches, or a fuzzy font-name list. The
        // filter input itself is rendered unconditionally below (outside
        // this match) so it — and its focus — persist across tab switches;
        // that's what lets a stray Escape keep bubbling to `InputEscape`
        // below no matter which tab is showing.
        let body: gpui::AnyElement = match focus {
            SettingsField::Theme => {
                let mut row = div().flex().flex_wrap().gap_2();
                for name in filtered_themes(&query) {
                    let active = name == active_theme_name;
                    row = row.child(
                        div()
                            .id(name)
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .cursor_pointer()
                            .when(active, |el| {
                                el.bg(Hsla::from(theme::mauve()).opacity(0.15))
                                    .border_color(theme::mauve())
                                    .text_color(theme::mauve())
                            })
                            .when(!active, |el| {
                                el.border_color(theme::surface0())
                                    .text_color(theme::subtext())
                                    .hover(|style| {
                                        style.bg(Hsla::from(theme::surface0()).opacity(0.5))
                                    })
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.update_global::<settings::Settings, _>(|s, _| {
                                    s.theme_name = name.to_string();
                                });
                                apply_and_save(this, cx);
                            }))
                            .child(SharedString::from(name)),
                    );
                }
                row.into_any_element()
            }
            SettingsField::UiFont | SettingsField::CodeFont => {
                let names = all_font_names(cx);
                let matches = filtered_fonts(&names, &query);
                let current = if focus == SettingsField::UiFont {
                    s.ui_font.clone().unwrap_or_default()
                } else {
                    s.code_font.clone()
                };
                let mut list = div()
                    .id("font-list")
                    .flex()
                    .flex_col()
                    .max_h(px(ROW_H * 6.))
                    .overflow_y_scroll();
                if matches.is_empty() {
                    list = list.child(
                        div()
                            .px_2()
                            .py_1()
                            .text_color(theme::overlay0())
                            .child(SharedString::from("no matching fonts")),
                    );
                }
                for &name in matches.iter().take(MAX_FONT_ROWS) {
                    let active = name == current;
                    let owned: String = name.to_string();
                    let is_ui = focus == SettingsField::UiFont;
                    list = list.child(font_row(name, active, cx, move |this, _, cx| {
                        cx.update_global::<settings::Settings, _>(|s, _| {
                            if is_ui {
                                s.ui_font = Some(owned.clone());
                            } else {
                                s.code_font = owned.clone();
                            }
                        });
                        apply_and_save(this, cx);
                    }));
                }
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(list)
                    .when(focus == SettingsField::CodeFont, |col| {
                        col.child(
                            div()
                                .text_color(theme::overlay0())
                                .child(SharedString::from(
                                "Use a monospace font — proportional fonts will misalign the diff.",
                            )),
                        )
                    })
                    .into_any_element()
            }
        };

        // Font size stepper.
        let size_row = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("settings-size-down")
                    .label("−")
                    .ghost()
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let size = cx.global::<settings::Settings>().font_size;
                        cx.update_global::<settings::Settings, _>(|s, _| {
                            s.set_font_size(size - 1.0)
                        });
                        apply_and_save(this, cx);
                    })),
            )
            .child(
                div()
                    .min_w(px(48.))
                    .flex()
                    .justify_center()
                    .text_color(theme::text())
                    .child(SharedString::from(format!("{:.0}px", s.font_size))),
            )
            .child(
                Button::new("settings-size-up")
                    .label("+")
                    .ghost()
                    .small()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let size = cx.global::<settings::Settings>().font_size;
                        cx.update_global::<settings::Settings, _>(|s, _| {
                            s.set_font_size(size + 1.0)
                        });
                        apply_and_save(this, cx);
                    })),
            );

        div()
            .absolute()
            .inset_0()
            .occlude()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(100.))
            .bg(theme::palette_backdrop(&theme::by_name(&s.theme_name)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    cx.stop_propagation();
                    close(this, window, cx);
                }),
            )
            .child(
                div()
                    .w(px(520.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    // The font-filter input propagates Escape when it has
                    // nothing of its own to dismiss; catch it here to close
                    // the modal (same pattern as render_review).
                    .on_action(cx.listener(move |this, _: &InputEscape, window, cx| {
                        close(this, window, cx);
                    }))
                    .rounded_lg()
                    .border_1()
                    .border_color(theme::surface0())
                    .bg(theme::mantle())
                    .shadow_lg()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .text_size(cx.global::<settings::Settings>().chrome(12.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_color(theme::overlay0())
                                    .child(SharedString::from("Settings")),
                            )
                            .child(
                                Button::new("settings-close")
                                    .icon(gpui_component::IconName::Close)
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        close(this, window, cx);
                                    })),
                            ),
                    )
                    .child(tabs)
                    // Always mounted (its content applies to whichever tab
                    // is active) so its `FocusHandle` stays tracked across
                    // tab switches — that's what keeps `InputEscape` routing
                    // to the handler below reliable no matter which tab is
                    // showing.
                    .child(Input::new(&font_filter))
                    .child(body)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(size_row)
                            .child(
                                Button::new("settings-reset")
                                    .label("Reset to defaults")
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        cx.update_global::<settings::Settings, _>(|s, _| {
                                            *s = settings::Settings::default();
                                        });
                                        apply_and_save(this, cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}
