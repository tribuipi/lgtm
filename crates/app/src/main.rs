mod theme;

use diff_core::{DiffRow, FileStatus, PrDiff};
use gpui::{
    actions, div, prelude::*, px, size, uniform_list, App, Application, Bounds, Context,
    FocusHandle, HighlightStyle, Hsla, KeyBinding, Keystroke, ListHorizontalSizingBehavior,
    ScrollStrategy, SharedString, StyledText, TitlebarOptions, UniformListScrollHandle, Window,
    WindowBounds, WindowOptions,
};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    kbd::Kbd,
    scroll::Scrollbar,
    tag::Tag,
    IconName, Root, Sizable as _, TitleBar,
};
use std::ops::Range;

const MONO: &str = "Menlo";
const ROW_HEIGHT: f32 = 22.0;
const TEXT_SIZE: f32 = 13.0;

actions!(review, [NextFile, PrevFile, NextHunk, PrevHunk, GoToTop, GoToBottom, Quit]);

fn main() {
    let arg = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => {
            eprintln!("usage: review <owner/repo#123 | PR URL | PR number>");
            std::process::exit(2);
        }
    };
    let locator = gh::resolve_pr_arg(&arg).unwrap_or_else(die);
    eprintln!(
        "fetching {}#{} via gh...",
        locator.repo_slug(),
        locator.number
    );
    let meta_loc = locator.clone();
    let meta_thread = std::thread::spawn(move || gh::fetch_meta(&meta_loc));
    let patch = gh::fetch_patch(&locator).unwrap_or_else(die);
    let meta = meta_thread.join().unwrap().unwrap_or_else(die);
    let diff = diff_core::parse_patch(&patch);

    let window_title: SharedString = format!(
        "review — {}#{}: {}",
        locator.repo_slug(),
        locator.number,
        meta.title
    )
    .into();

    Application::new()
        .with_assets(gpui_component_assets::Assets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            theme::apply_ui_theme(cx);
            cx.bind_keys([
                KeyBinding::new("]", NextFile, Some("ReviewApp")),
                KeyBinding::new("[", PrevFile, Some("ReviewApp")),
                KeyBinding::new("n", NextHunk, Some("ReviewApp")),
                KeyBinding::new("p", PrevHunk, Some("ReviewApp")),
                KeyBinding::new("home", GoToTop, Some("ReviewApp")),
                KeyBinding::new("end", GoToBottom, Some("ReviewApp")),
                KeyBinding::new("cmd-q", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx| cx.quit());

            let bounds = Bounds::centered(None, size(px(1280.), px(860.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(window_title.clone()),
                        ..TitleBar::title_bar_options()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| ReviewApp::new(meta, &diff, cx));
                    window.focus(&view.read(cx).focus_handle);
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .unwrap();
            cx.activate(true);
        });
}

fn die<E: std::fmt::Display, T>(err: E) -> T {
    eprintln!("error: {err}");
    std::process::exit(1);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Context,
    Added,
    Removed,
}

enum Row {
    Spacer,
    FileHeader {
        path: SharedString,
        old_path: Option<SharedString>,
        status: FileStatus,
        additions: u32,
        deletions: u32,
    },
    HunkHeader {
        label: SharedString,
    },
    Binary,
    Line {
        old_no: Option<u32>,
        new_no: Option<u32>,
        kind: LineKind,
        text: SharedString,
        intra: Vec<Range<usize>>,
    },
}

struct ReviewApp {
    meta: gh::PrMeta,
    rows: Vec<Row>,
    file_rows: Vec<usize>,
    hunk_rows: Vec<usize>,
    cursor: usize,
    scroll: UniformListScrollHandle,
    focus_handle: FocusHandle,
}

impl ReviewApp {
    fn new(meta: gh::PrMeta, diff: &PrDiff, cx: &mut Context<Self>) -> Self {
        let mut rows = Vec::new();
        let mut file_rows = Vec::new();
        let mut hunk_rows = Vec::new();

        for file in &diff.files {
            if !rows.is_empty() {
                rows.push(Row::Spacer);
            }
            file_rows.push(rows.len());
            rows.push(Row::FileHeader {
                path: file.display_path().to_string().into(),
                old_path: match file.status {
                    FileStatus::Renamed => file.old_path.clone().map(Into::into),
                    _ => None,
                },
                status: file.status,
                additions: file.additions,
                deletions: file.deletions,
            });
            if file.status == FileStatus::Binary {
                rows.push(Row::Binary);
                continue;
            }
            for hunk in &file.hunks {
                hunk_rows.push(rows.len());
                let mut label = format!(
                    "@@ -{},{} +{},{} @@",
                    hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                );
                if !hunk.section.is_empty() {
                    label.push(' ');
                    label.push_str(&hunk.section);
                }
                rows.push(Row::HunkHeader {
                    label: label.into(),
                });
                for row in &hunk.rows {
                    rows.push(match row {
                        DiffRow::Context {
                            old_no,
                            new_no,
                            text,
                        } => Row::Line {
                            old_no: Some(*old_no),
                            new_no: Some(*new_no),
                            kind: LineKind::Context,
                            text: text.clone().into(),
                            intra: Vec::new(),
                        },
                        DiffRow::Added {
                            new_no,
                            text,
                            intra,
                        } => Row::Line {
                            old_no: None,
                            new_no: Some(*new_no),
                            kind: LineKind::Added,
                            text: text.clone().into(),
                            intra: intra.clone(),
                        },
                        DiffRow::Removed {
                            old_no,
                            text,
                            intra,
                        } => Row::Line {
                            old_no: Some(*old_no),
                            new_no: None,
                            kind: LineKind::Removed,
                            text: text.clone().into(),
                            intra: intra.clone(),
                        },
                    });
                }
            }
        }

        Self {
            meta,
            rows,
            file_rows,
            hunk_rows,
            cursor: 0,
            scroll: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
        }
    }

    fn jump(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.cursor = ix;
        self.scroll.scroll_to_item_strict(ix, ScrollStrategy::Top);
        cx.notify();
    }

    fn jump_next(&mut self, targets: &[usize], cx: &mut Context<Self>) {
        if let Some(&ix) = targets.iter().find(|&&ix| ix > self.cursor) {
            self.jump(ix, cx);
        }
    }

    fn jump_prev(&mut self, targets: &[usize], cx: &mut Context<Self>) {
        if let Some(&ix) = targets.iter().rev().find(|&&ix| ix < self.cursor) {
            self.jump(ix, cx);
        }
    }

    fn render_row(&self, ix: usize) -> gpui::AnyElement {
        let row_height = px(ROW_HEIGHT);
        match &self.rows[ix] {
            Row::Spacer => div().h(row_height).into_any_element(),
            Row::FileHeader {
                path,
                old_path,
                status,
                additions,
                deletions,
            } => {
                let (status_label, status_color) = match status {
                    FileStatus::Added => ("added", theme::green()),
                    FileStatus::Deleted => ("deleted", theme::red()),
                    FileStatus::Modified => ("modified", theme::blue()),
                    FileStatus::Renamed => ("renamed", theme::mauve()),
                    FileStatus::Binary => ("binary", theme::peach()),
                };
                let status: Hsla = status_color.into();
                let mut header = div()
                    .h(row_height)
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .bg(theme::mantle())
                    .child(
                        Tag::custom(status.opacity(0.15), status, status.opacity(0.4))
                            .small()
                            .child(SharedString::from(status_label)),
                    )
                    .child(
                        div()
                            .text_color(theme::text())
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(path.clone()),
                    );
                if let Some(old_path) = old_path {
                    header = header.child(
                        div()
                            .text_color(theme::overlay0())
                            .child(SharedString::from(format!("← {old_path}"))),
                    );
                }
                header
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_color(theme::green())
                            .child(SharedString::from(format!("+{additions}"))),
                    )
                    .child(
                        div()
                            .text_color(theme::red())
                            .child(SharedString::from(format!("−{deletions}"))),
                    )
                    .into_any_element()
            }
            Row::HunkHeader { label } => div()
                .h(row_height)
                .flex()
                .items_center()
                .px_3()
                .bg(theme::crust())
                .text_color(theme::overlay0())
                .child(label.clone())
                .into_any_element(),
            Row::Binary => div()
                .h(row_height)
                .flex()
                .items_center()
                .px_3()
                .text_color(theme::overlay0())
                .child(SharedString::from("binary file changed"))
                .into_any_element(),
            Row::Line {
                old_no,
                new_no,
                kind,
                text,
                intra,
            } => {
                let (row_bg, word_bg, marker, marker_color) = match kind {
                    LineKind::Context => (None, None, "", theme::overlay0()),
                    LineKind::Added => (
                        Some(theme::added_row_bg()),
                        Some(theme::added_word_bg()),
                        "+",
                        theme::green(),
                    ),
                    LineKind::Removed => (
                        Some(theme::removed_row_bg()),
                        Some(theme::removed_word_bg()),
                        "−",
                        theme::red(),
                    ),
                };
                let number = |no: Option<u32>| {
                    div()
                        .w(px(44.))
                        .flex_shrink_0()
                        .text_color(theme::overlay0())
                        .flex()
                        .justify_end()
                        .child(SharedString::from(
                            no.map(|no| no.to_string()).unwrap_or_default(),
                        ))
                };
                let content: gpui::AnyElement = if intra.is_empty() {
                    div().child(text.clone()).into_any_element()
                } else {
                    let highlights = intra.iter().map(|range| {
                        (
                            range.clone(),
                            HighlightStyle {
                                background_color: Some(word_bg.unwrap().into()),
                                ..Default::default()
                            },
                        )
                    });
                    StyledText::new(text.clone())
                        .with_highlights(highlights)
                        .into_any_element()
                };
                let mut line = div().h(row_height).flex().items_center();
                if let Some(bg) = row_bg {
                    line = line.bg(bg);
                }
                line.child(number(*old_no))
                    .child(number(*new_no))
                    .child(
                        div()
                            .w(px(28.))
                            .flex_shrink_0()
                            .flex()
                            .justify_center()
                            .text_color(marker_color)
                            .child(SharedString::from(marker)),
                    )
                    .child(div().whitespace_nowrap().child(content))
                    .into_any_element()
            }
        }
    }

    fn render_titlebar(&self) -> impl IntoElement {
        let meta = &self.meta;
        let (state_color, state_label) = match meta.state.as_str() {
            "OPEN" => (theme::green(), "open"),
            "MERGED" => (theme::mauve(), "merged"),
            "CLOSED" => (theme::red(), "closed"),
            other => (theme::overlay0(), other),
        };
        let state: Hsla = state_color.into();
        let url = meta.url.clone();
        TitleBar::new()
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .flex_1()
                    .child(
                        Tag::custom(state.opacity(0.15), state, state.opacity(0.4))
                            .small()
                            .child(SharedString::from(state_label.to_string())),
                    )
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .truncate()
                            .child(SharedString::from(meta.title.clone())),
                    )
                    .child(
                        div()
                            .text_color(theme::subtext())
                            .child(SharedString::from(format!("#{}", meta.number))),
                    )
                    .child(
                        div()
                            .text_color(theme::subtext())
                            .whitespace_nowrap()
                            .child(SharedString::from(format!("by {}", meta.author.login))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_shrink_0()
                    .pr_3()
                    .child(div().text_color(theme::overlay0()).child(SharedString::from(
                        format!("{} ← {}", meta.base_ref_name, meta.head_ref_name),
                    )))
                    .child(
                        div()
                            .text_color(theme::green())
                            .child(SharedString::from(format!("+{}", meta.additions))),
                    )
                    .child(
                        div()
                            .text_color(theme::red())
                            .child(SharedString::from(format!("−{}", meta.deletions))),
                    )
                    .child(
                        Button::new("open-in-browser")
                            .icon(IconName::ExternalLink)
                            .ghost()
                            .xsmall()
                            .on_click(move |_, _, cx| cx.open_url(&url)),
                    ),
            )
    }

    fn render_footer(&self) -> impl IntoElement {
        let hint = |keys: &[&str], label: &'static str| {
            let mut hint = div().flex().items_center().gap_1();
            for key in keys {
                hint = hint.child(Kbd::new(Keystroke::parse(key).unwrap()));
            }
            hint.child(
                div()
                    .text_color(theme::overlay0())
                    .child(SharedString::from(label)),
            )
        };
        div()
            .h(px(28.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_4()
            .px_3()
            .bg(theme::mantle())
            .border_t_1()
            .border_color(theme::surface0())
            .text_size(px(12.))
            .child(hint(&["]", "["], "files"))
            .child(hint(&["n", "p"], "hunks"))
            .child(hint(&["home", "end"], "top/bottom"))
    }
}

impl Render for ReviewApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::base())
            .text_color(theme::text())
            .track_focus(&self.focus_handle)
            .key_context("ReviewApp")
            .on_action(cx.listener(|this, _: &NextFile, _, cx| {
                let targets = this.file_rows.clone();
                this.jump_next(&targets, cx)
            }))
            .on_action(cx.listener(|this, _: &PrevFile, _, cx| {
                let targets = this.file_rows.clone();
                this.jump_prev(&targets, cx)
            }))
            .on_action(cx.listener(|this, _: &NextHunk, _, cx| {
                let targets = this.hunk_rows.clone();
                this.jump_next(&targets, cx)
            }))
            .on_action(cx.listener(|this, _: &PrevHunk, _, cx| {
                let targets = this.hunk_rows.clone();
                this.jump_prev(&targets, cx)
            }))
            .on_action(cx.listener(|this, _: &GoToTop, _, cx| this.jump(0, cx)))
            .on_action(cx.listener(|this, _: &GoToBottom, _, cx| {
                let last = this.rows.len().saturating_sub(1);
                this.jump(last, cx)
            }))
            .child(self.render_titlebar())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .font_family(MONO)
                    .text_size(px(TEXT_SIZE))
                    .line_height(px(ROW_HEIGHT))
                    .child(
                        uniform_list("diff", self.rows.len(), move |range, _window, cx| {
                            let this = entity.read(cx);
                            range.map(|ix| this.render_row(ix)).collect()
                        })
                        .track_scroll(self.scroll.clone())
                        .with_horizontal_sizing_behavior(
                            ListHorizontalSizingBehavior::Unconstrained,
                        )
                        .h_full(),
                    )
                    .child(Scrollbar::new(&self.scroll)),
            )
            .child(self.render_footer())
    }
}
