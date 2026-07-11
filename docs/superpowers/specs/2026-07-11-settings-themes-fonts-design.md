# Settings: Themes, Font Family, Font Size

**Date:** 2026-07-11
**Status:** Approved (brainstorm)

## Goal

Let users customize the appearance of `lgtm`:

- **Theme** — pick from a curated set of built-in themes (both UI palette and syntax colors).
- **UI font family** and **code font family** — two independent pickers, populated from installed system fonts.
- **Font size** — a single master knob that scales the code/diff text *and* all UI chrome proportionally.

Settings are changed through an **in-app settings UI** (a modal), apply live, and **persist** across launches.

The app currently has **no configuration infrastructure** — this is net-new. `theme.rs` hardcodes Catppuccin Mocha; fonts are the constants `MONO = "Menlo"` and `TEXT_SIZE = 13.0` in `main.rs`.

## Decisions (from brainstorming)

- **Mechanism:** in-app settings UI (opened with `cmd-,`).
- **Themes:** ship a curated built-in set now, with a clean data seam so external theme loading (e.g. Helix `.toml`) can be added later.
- **Font scope:** two font families (UI + code) and **one** font size that scales everything.
- **Font picker:** enumerate installed system fonts via `cx.text_system().all_font_names()`, presented as a fuzzy-filtered list.
- **Non-monospace code font:** user-beware (option a). The code-font picker lists all fonts; picking a proportional font will misalign diff columns. A small note in the UI warns about this. No monospace allowlist for now.

## Architecture

A **`Settings` global** (gpui `Global`) is the single source of truth, mirroring how `gpui-component`'s own `Theme` lives as a global. Everything reads appearance values from it.

```
Settings (global)
  ├─ theme_name: String        // selects a built-in Theme
  ├─ ui_font: String
  ├─ code_font: String
  └─ font_size: f32            // master size; chrome scales relative to 13.0 baseline
```

### New / changed modules

- **`settings.rs` (new)** — the `Settings` struct, its `Global` registration, defaults, and `load()`/`save()`.
  - Persisted as **TOML** to the platform config dir: `~/Library/Application Support/lgtm/config.toml` (via the `dirs` crate). TOML chosen for hand-editability, consistent with the future external-theme direction.
  - New deps: `serde` (derive), `toml`, `dirs`.
  - `load()` reads the file if present, falling back to defaults for any missing/invalid field (never panics on a bad config). `save()` writes the whole struct.
  - Provides derived accessors:
    - `scale() -> f32` = `font_size / 13.0`.
    - `chrome(px_base: f32) -> Pixels` = `px(px_base * scale())` — used to replace every hardcoded chrome `text_size`.
    - `row_height() -> f32` = `font_size * (22.0 / 13.0)` — replaces the `ROW_HEIGHT` constant for the diff.

- **`theme.rs` (refactor)** — turn the hardcoded functions into a data-driven `Theme`:
  - A `Theme` struct holding the UI palette fields currently set in `apply_ui_theme` plus the syntax-token color map currently in `token_style`, plus a `mode: ThemeMode` (Light/Dark).
  - Built-in constructors returning `Theme` values:
    - `catppuccin_mocha()` — **default**, exact current values.
    - `catppuccin_latte()` — light.
    - one additional dark theme (Tokyo Night or Gruvbox).
  - A registry/lookup: `by_name(&str) -> Theme` and `all() -> &[(name, ThemeName)]` for the picker, with fallback to the default on unknown name.
  - `apply_ui_theme(&Theme, cx)` applies a given `Theme` to the gpui-component global theme (same field assignments as today, but sourced from the struct).
  - `token_style(&Theme, Token) -> HighlightStyle` reads syntax colors from the theme.
  - **This `Theme` struct is the seam** a future external loader targets (parse Helix `.toml` → `Theme`).

- **`settings_ui.rs` (new)** — the settings modal, a gpui `Entity`:
  - Opened via a new `OpenSettings` action bound to `cmd-,` (global context, works even when an input is focused).
  - Controls:
    - **Theme** — list/segmented control of built-in theme names.
    - **UI font** — fuzzy-filtered dropdown of `all_font_names()`.
    - **Code font** — same list, with a one-line note: "Use a monospace font — others will misalign the diff."
    - **Font size** — stepper / `+`/`−` (and a numeric field), reasonable clamp (e.g. 8–32).
    - **Reset to defaults** button.
  - Each change: mutate the `Settings` global → apply effects (below) → `save()` → request redraw.
  - Modal styling and dismissal follow the existing command-palette pattern (backdrop + escape to close).

### `main.rs` changes

- Remove the `TEXT_SIZE` / `ROW_HEIGHT` / `MONO` hardcodes as *sources of truth*; read from `Settings` instead.
  - Diff pane font family → `settings.code_font`, size → `settings.font_size`, line height → `settings.row_height()`.
  - Every `.text_size(px(N.))` chrome call → `.text_size(settings.chrome(N.))`. UI text uses `settings.ui_font` where a family is set.
- **Metric invalidation (correctness-critical):** `ReviewApp.char_width` caches the mono cell advance measured once at `(MONO, TEXT_SIZE)`. Mouse→column math and layout depend on it. When the code font or size changes, reset `char_width = None` so it re-measures at the new `(code_font, font_size)`, and ensure the row-height derivation uses the new size. `char_width()` must resolve `settings.code_font` instead of the `MONO` constant.
- Startup: after `gpui_component::init`, `Settings::load()` into a global, then `apply_ui_theme(&settings.theme(), cx)` (replaces the current unconditional Mocha application). Bind `cmd-,` and register the `OpenSettings` action to open the modal.

### Data flow on a settings change

1. User edits a control in the settings modal.
2. Modal updates the `Settings` global field.
3. If theme changed → `apply_ui_theme(&new_theme, cx)`.
4. If code font or size changed → clear `ReviewApp.char_width`.
5. `Settings::save()` writes TOML.
6. `cx.refresh()` / notify so the root view re-renders with new values.

## Error handling

- Missing/corrupt config file → fall back to defaults, don't crash; overwrite on next save.
- Unknown `theme_name` → default theme.
- Font name that doesn't resolve → gpui falls back to its default font; we accept the resulting rendering (no hard validation). Picker only offers real installed names, so this is an edge case (e.g. a font uninstalled since it was chosen).
- Config-dir unavailable (can't resolve `dirs`) → run with in-memory defaults; `save()` is a no-op that logs.

## Testing

- **`settings.rs` unit tests:** defaults; round-trip serialize→deserialize; `load()` from a partial/garbage TOML yields defaults for bad fields; `scale()`/`chrome()`/`row_height()` math at a few sizes.
- **`theme.rs` unit tests:** `by_name` returns the right built-in and falls back on unknown; each built-in populates the expected fields (spot-check a couple of colors, incl. that Mocha still matches the current hardcoded values); `token_style` returns theme-sourced colors.
- **Manual verification (documented in the plan):** open `cmd-,`; switch theme (dark→light) live; change code font and size and confirm the diff re-lays-out and mouse selection still lands on the right column; restart and confirm settings persisted.

## Out of scope (YAGNI)

- External/Helix theme file loading (seam only; not implemented).
- Independent UI vs. code font *sizes* (single size for now).
- Monospace-only filtering / allowlist for the code font picker.
- Per-repo or per-item settings; live hot-reload of the config file edited outside the app.

## Keymap / docs

- Add `cmd-,` → open settings to the README keymap table and the app's key handling.
