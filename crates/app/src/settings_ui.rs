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

use gpui::{div, prelude::*, px, Context, Entity, FocusHandle, MouseButton, SharedString, Subscription, Window};
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
    pub theme_select: Entity<SelectState<SearchableVec<SharedString>>>,
    pub ui_font_select: Entity<SelectState<SearchableVec<SharedString>>>,
    pub code_font_select: Entity<SelectState<SearchableVec<SharedString>>>,
    /// Holds the three `SelectEvent` subscriptions alive for the modal's
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
    let s = cx.global::<settings::Settings>().clone();
    theme::apply_ui_theme(&theme::by_name(&s.theme_name), cx);
    app.char_width = None;
    s.save();
    if let Some(ui) = &app.settings {
        window.focus(&ui.focus_handle);
    }
    cx.notify();
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
                    .child(field(
                        "Theme",
                        Select::new(&ui.theme_select)
                            .with_size(Size::Small)
                            .menu_width(px(360.))
                            .placeholder("Theme"),
                    ))
                    .child(field(
                        "UI font",
                        Select::new(&ui.ui_font_select)
                            .with_size(Size::Small)
                            .menu_width(px(360.))
                            .placeholder("UI font"),
                    ))
                    .child(field("Code font", code_font_block))
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
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        cx.update_global::<settings::Settings, _>(|s, _| {
                                            *s = settings::Settings::default();
                                        });
                                        apply_and_save(this, window, cx);
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}
