//! In-app settings modal, opened with `cmd-,` (see `OpenSettings` in
//! main.rs). Lets the user swap theme, UI font, code font, and font size.
//! Font and size changes apply + persist immediately on selection. Themes
//! preview live *while you hover* the theme list (no save); a theme only
//! sticks — and is written to disk — when you click it. Leaving the list or
//! closing the modal reverts an unclicked preview to the committed theme.
//!
//! Kept out of main.rs to bound that file's growth. `render_settings` is
//! implemented as a method on `ReviewApp` below (in an `impl` block, same
//! type as `render_review`/`render_palette`) so it can reach the private
//! fields (`char_width`, `focus_handle`, `settings`, …) those methods do —
//! this submodule is a descendant of the crate root where `ReviewApp` is
//! defined, so private-to-crate-root items are visible here.

use gpui::{
    div, prelude::*, px, Context, Entity, FocusHandle, Hsla, MouseButton, SharedString,
    Subscription, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    select::{SearchableVec, Select, SelectState},
    Sizable as _, Size,
};

use crate::{settings, theme, ReviewApp};

pub struct SettingsUi {
    /// Card focus target so the "Settings" key-context (Escape-to-close) is
    /// active whenever the modal is up. See `render_settings`.
    pub focus_handle: FocusHandle,
    /// The committed (persisted) theme when the modal opened, updated on every
    /// theme *click*. Hovering a theme row previews it live without saving;
    /// leaving the list — or closing the modal — reverts to this baseline. So
    /// browsing themes is a true preview that only sticks when clicked.
    pub baseline_theme: String,
    /// Index into `theme::all_names()` of the row the keyboard cursor / mouse
    /// hover is on (and thus the theme being previewed). Kept in sync by both
    /// arrow-key nav and hover so the highlight is single-sourced.
    pub theme_cursor: usize,
    pub ui_font_select: Entity<SelectState<SearchableVec<SharedString>>>,
    pub code_font_select: Entity<SelectState<SearchableVec<SharedString>>>,
    /// Holds the two font `SelectEvent` subscriptions alive for the modal's
    /// lifetime; dropped (and thus unsubscribed) when the modal closes.
    pub _subs: Vec<Subscription>,
}

/// Every font name the text system knows about, deduped and sorted for
/// stable, readable display. Still used to populate the two font dropdowns.
pub fn all_font_names(cx: &gpui::App) -> Vec<String> {
    let mut names = cx.text_system().all_font_names();
    names.sort();
    names.dedup();
    names
}

/// After any settings mutation (theme/font/size change or reset): re-apply
/// the (possibly new) theme so the UI + syntax colors follow it live, force
/// a re-measure of the monospace cell (code font or size may have changed —
/// stale `char_width` would misalign diff columns and mouse selection),
/// persist to disk, and repaint. Safe to call even when nothing changed.
///
/// Also refocuses the modal card handle: clicking a dropdown option, a size
/// stepper, or Reset moves focus off the card, and Escape only closes the
/// modal while the "Settings" key context is active (i.e. while the card
/// handle is focused — see the `CloseSettings` handler in `render_settings`).
/// Refocusing here keeps Escape working no matter which control was clicked.
///
/// Invariant: any change to `settings.theme_name` MUST be followed by
/// `apply_ui_theme(&by_name(theme_name))` in the same synchronous step (as
/// done here), or the bare accessors in `theme.rs` (`ACTIVE`) will diverge
/// from the syntax/tint colors derived inline via `by_name`.
pub fn apply_and_save(app: &mut ReviewApp, window: &mut Window, cx: &mut Context<ReviewApp>) {
    // A commit must persist the *committed* theme, never a transient
    // hover-preview. Hover only sets the global for live preview (no save);
    // theme-click/reset set `baseline_theme` before calling here (so this is
    // a no-op for them), while font/size commits drop any active preview back
    // to the baseline. Without this, a keyboard Tab from a hovered theme row
    // to a font/size control (no mouse-move → no hover-revert) would save the
    // un-clicked preview.
    if let Some(baseline) = app.settings.as_ref().map(|ui| ui.baseline_theme.clone()) {
        cx.update_global::<settings::Settings, _>(|s, _| s.theme_name = baseline);
    }
    let s = cx.global::<settings::Settings>().clone();
    theme::apply_ui_theme(&theme::by_name(&s.theme_name), cx);
    app.char_width = None;
    s.save();
    if let Some(ui) = &app.settings {
        window.focus(&ui.focus_handle);
    }
    cx.notify();
}

/// Apply `name` as the active theme for live preview — updates the global so
/// the whole UI (chrome, syntax, tints) repaints in it and forces a diff
/// re-measure, but does NOT persist. Reverting is just another `preview_theme`
/// call with the baseline name. Committing is `apply_and_save` (which saves).
///
/// Upholds the same `theme_name`→`apply_ui_theme` invariant as `apply_and_save`.
fn preview_theme(app: &mut ReviewApp, name: &str, cx: &mut Context<ReviewApp>) {
    let name = name.to_string();
    cx.update_global::<settings::Settings, _>(|s, _| s.theme_name = name.clone());
    theme::apply_ui_theme(&theme::by_name(&name), cx);
    app.char_width = None;
    cx.notify();
}

/// Move the theme cursor to `ix` (clamped) and preview that theme live.
/// Shared by arrow-key nav and hover so both drive the same highlight.
pub(crate) fn preview_theme_at(app: &mut ReviewApp, ix: usize, cx: &mut Context<ReviewApp>) {
    let names = theme::all_names();
    let ix = ix.min(names.len().saturating_sub(1));
    if let Some(ui) = &mut app.settings {
        ui.theme_cursor = ix;
    }
    preview_theme(app, names[ix], cx);
}

/// Commit `name` as the chosen theme: update the global + baseline and persist
/// (via `apply_and_save`). Shared by row click and Enter-to-select.
pub(crate) fn commit_theme(
    app: &mut ReviewApp,
    name: &str,
    window: &mut Window,
    cx: &mut Context<ReviewApp>,
) {
    let name = name.to_string();
    cx.update_global::<settings::Settings, _>(|s, _| s.theme_name = name.clone());
    if let Some(ui) = &mut app.settings {
        ui.baseline_theme = name.clone();
    }
    apply_and_save(app, window, cx);
}

/// One labeled form field: a `subtext`-colored label stacked above its
/// control.
fn field(label: &str, control: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_color(theme::subtext())
                .child(SharedString::from(label.to_string())),
        )
        .child(control)
}

impl ReviewApp {
    /// The settings modal: a dimming backdrop (click closes) over a centered
    /// card laid out as a vertical form. The card owns a `FocusHandle` and a
    /// "Settings" key context so Escape closes the modal (see `CloseSettings`
    /// in main.rs).
    pub(crate) fn render_settings(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(ui) = &self.settings else {
            return div().into_any_element();
        };
        let s = cx.global::<settings::Settings>().clone();

        let close = |this: &mut Self, window: &mut Window, cx: &mut Context<Self>| {
            // Drop any lingering hover-preview back to the committed theme
            // before closing (e.g. Escape pressed while a row is hovered).
            let baseline = this.settings.as_ref().map(|ui| ui.baseline_theme.clone());
            if let Some(baseline) = baseline {
                preview_theme(this, &baseline, cx);
            }
            this.settings = None;
            window.focus(&this.focus_handle);
            cx.notify();
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
                    .on_click(cx.listener(|this, _, window, cx| {
                        let size = cx.global::<settings::Settings>().font_size;
                        cx.update_global::<settings::Settings, _>(|s, _| {
                            s.set_font_size(size - 1.0)
                        });
                        apply_and_save(this, window, cx);
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
                    .on_click(cx.listener(|this, _, window, cx| {
                        let size = cx.global::<settings::Settings>().font_size;
                        cx.update_global::<settings::Settings, _>(|s, _| {
                            s.set_font_size(size + 1.0)
                        });
                        apply_and_save(this, window, cx);
                    })),
            );

        // Theme picker: a hover-preview list. Hovering a row applies that
        // theme live (no save); clicking commits + persists it; leaving the
        // list reverts to the committed baseline (see `preview_theme`).
        let baseline = ui.baseline_theme.clone();
        // The cursor (arrow-key nav or hover) is the single source of the
        // highlighted/previewed row.
        let cursor = ui.theme_cursor;
        let mut theme_list = div()
            .id("theme-list")
            .flex()
            .flex_col()
            .max_h(px(220.))
            .overflow_y_scroll()
            .on_hover(cx.listener({
                let baseline = baseline.clone();
                move |this, hovered: &bool, _window, cx| {
                    // Pointer left the whole list → snap the cursor back to the
                    // committed theme and drop the preview.
                    if !*hovered {
                        let ix = theme::all_names()
                            .iter()
                            .position(|n| *n == baseline)
                            .unwrap_or(0);
                        preview_theme_at(this, ix, cx);
                    }
                }
            }));
        for (i, name) in theme::all_names().iter().enumerate() {
            let name: &'static str = name;
            // `active` (row highlight) follows the cursor (previewed theme);
            // `selected` (right-side tick) marks the *committed* one, so while
            // you browse others the tick still shows what's actually chosen.
            let active = i == cursor;
            let selected = name == baseline.as_str();
            theme_list = theme_list.child(
                div()
                    .id(name)
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .when(active, |row| {
                        row.bg(Hsla::from(theme::surface0())).text_color(theme::text())
                    })
                    .when(!active, |row| {
                        row.text_color(theme::subtext())
                            .hover(|style| style.bg(Hsla::from(theme::surface0()).opacity(0.5)))
                    })
                    .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                        if *hovered {
                            preview_theme_at(this, i, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        commit_theme(this, name, window, cx);
                    }))
                    .child(SharedString::from(name))
                    .when(selected, |row| {
                        row.child(
                            gpui_component::Icon::new(gpui_component::IconName::Check)
                                .with_size(Size::Small)
                                .text_color(theme::green()),
                        )
                    }),
            );
        }

        // Code-font field: dropdown plus a note steering users to a
        // monospace face (proportional fonts misalign the diff grid).
        let code_font_block = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                Select::new(&ui.code_font_select)
                    .with_size(Size::Small)
                    .menu_width(px(360.))
                    .placeholder("Code font"),
            )
            .child(
                div()
                    .text_color(theme::overlay0())
                    .child(SharedString::from(
                        "Use a monospace font — proportional fonts will misalign the diff.",
                    )),
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
                    .track_focus(&ui.focus_handle)
                    .key_context("Settings")
                    .w(px(420.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_action(cx.listener(move |this, _: &crate::CloseSettings, window, cx| {
                        close(this, window, cx);
                    }))
                    // Arrow keys move the theme cursor (previewing live);
                    // Enter commits the theme under the cursor.
                    .on_action(cx.listener(|this, _: &crate::SettingsThemePrev, _window, cx| {
                        let cur = this.settings.as_ref().map(|ui| ui.theme_cursor);
                        if let Some(cur) = cur {
                            preview_theme_at(this, cur.saturating_sub(1), cx);
                        }
                    }))
                    .on_action(cx.listener(|this, _: &crate::SettingsThemeNext, _window, cx| {
                        let cur = this.settings.as_ref().map(|ui| ui.theme_cursor);
                        if let Some(cur) = cur {
                            preview_theme_at(this, cur + 1, cx);
                        }
                    }))
                    .on_action(cx.listener(|this, _: &crate::SettingsThemeConfirm, window, cx| {
                        let ix = this.settings.as_ref().map(|ui| ui.theme_cursor);
                        if let Some(ix) = ix {
                            let names = theme::all_names();
                            let name = names[ix.min(names.len().saturating_sub(1))];
                            commit_theme(this, name, window, cx);
                        }
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
                    .child(field("Theme", theme_list))
                    .child(field(
                        "UI font",
                        Select::new(&ui.ui_font_select)
                            .with_size(Size::Small)
                            .menu_width(px(360.))
                            .placeholder("UI font"),
                    ))
                    .child(field("Code font", code_font_block))
                    .child(field("Font size", size_row))
                    .child(
                        div().flex().justify_end().child(
                            Button::new("settings-reset")
                                .label("Reset to defaults")
                                .ghost()
                                .small()
                                .on_click(cx.listener(|this, _, window, cx| {
                                    cx.update_global::<settings::Settings, _>(|s, _| {
                                        *s = settings::Settings::default();
                                    });
                                    // Reset changes the theme too, so move the
                                    // preview baseline with it.
                                    let default_theme = settings::Settings::default().theme_name;
                                    if let Some(ui) = &mut this.settings {
                                        ui.baseline_theme = default_theme;
                                    }
                                    apply_and_save(this, window, cx);
                                })),
                        ),
                    ),
            )
            .into_any_element()
    }
}
