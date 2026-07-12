# Zed-Compatible Theme System

**Date:** 2026-07-11
**Status:** Design — approved for planning

## Goal

Refactor lgtm's theme system to Zed's model so that:

1. The internal representation uses **semantic role names** (Zed's vocabulary) instead of hue names (Catppuccin's `base`/`blue`/`red`/…).
2. The app can **load existing Zed theme JSON files** at runtime, so users bring their own themes.

Only **Catppuccin Mocha** ships bundled (the guaranteed default); every other theme comes from disk.

## Background — current system

Established by exploration of the existing code:

- One `Theme` struct (`crates/app/src/theme.rs:89`) with 18 hue-named `u32` color fields (11 UI-chrome + 5 syntax-only) plus `name`/`mode`. It is the single palette representation.
- Consumers:
  - **~161 bare-accessor call sites** (`theme::base()`/`blue()`/…) in `main.rs` and `settings_ui.rs`, backed by a thread-local `ACTIVE: RefCell<Theme>`.
  - **`token_style`** (`theme.rs:545`) maps the `syntax` crate's 16-variant `Token` enum → palette colors (+ italic).
  - **7 diff/tint helpers** (`added_row_bg`, `removed_row_bg`, `added_word_bg`, `removed_word_bg`, `selection_bg`, `void_cell_bg`, `palette_backdrop`) computing hue+opacity.
  - **`apply_ui_theme`** (`theme.rs:474`) maps the palette onto ~50 fields of `gpui_component::Theme` (the widget theme for `Button`/`Input`/`Scrollbar`/`Select`).
- 13 built-in themes via Rust constructors + `all_names()` + `by_name()`.
- Persistence: a single `theme_name: String` in `~/<config>/lgtm/config.toml` (`settings.rs`), resolved through `by_name()`.
- Selection UI: hover-preview theme list in `settings_ui.rs` (`preview_theme` on hover, `commit_theme`/`apply_and_save` on click/Enter).
- Invariant: any change to `settings.theme_name` must be followed by `apply_ui_theme` in the same synchronous step (keeps `ACTIVE` in sync with inline `by_name`-derived colors).

## Design decisions (approved)

| Decision | Choice |
|---|---|
| Scope | Both: Zed-style internal roles **and** load external Zed JSON |
| Theme source | App themes dir **plus** opportunistic scan of Zed's locations |
| Built-ins | Bundle **only Catppuccin Mocha**, as real upstream Zed JSON |
| Consumer model | Keep thread-local `ACTIVE` + bare accessors, renamed to semantic roles |
| Discovery | **Background** task, run **on settings open**, registry **discarded on close** |

---

## Section 1 — Data model & Zed JSON mapping

### Parsing target

A Zed theme file is a *family*:

```rust
struct ThemeFamily { name: String, author: Option<String>, themes: Vec<ZedThemeDef> }
struct ZedThemeDef { name: String, appearance: Appearance, style: Style }   // appearance: light | dark
struct Style { /* Option<Color> per role we consume */, syntax: HashMap<String, SyntaxStyle> }
struct SyntaxStyle { color: Option<Color>, font_style: Option<FontStyle>, font_weight: Option<f32> }
```

`Color` is a custom `Deserialize` for Zed hex strings — `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa` — into gpui `Rgba` (**alpha preserved**; tints now come from the theme, not computed). Unknown `appearance`/enum values are parse errors handled per Section 4.

### Internal `Theme` (post-refactor)

The single struct every consumer sees — same role for every consumer as today, but **semantic-role-named** and fully concrete after the resolver (Section 4). Syntax colors move out of the top-level palette into a `syntax` map.

| New role (accessor) | From Zed style key | Replaces today |
|---|---|---|
| `background()` | `background` | `crust()` |
| `editor_bg()` | `editor.background` | `base()` |
| `surface()` | `surface.background` / `elevated_surface.background` | `mantle()` |
| `element_bg()` | `element.background` | some `surface0()` |
| `border()` | `border` | most `surface0()` |
| `text()` | `text` | `text()` |
| `text_muted()` | `text.muted` | `subtext()` |
| `text_subtle()` | `text.placeholder` / derived | `overlay0()` |
| `accent()` | `text.accent` | `blue()` |
| `created()` / `deleted()` / `modified()` | `created` / `deleted` / `modified` | diff tints (green/red) |
| `error()` / `warning()` / `success()` / `info()` | same | `red()` / `peach()` / `green()` |
| `syntax[Token]` | `syntax` map | `mauve`/`yellow`/`lavender`/`maroon`/`sky`/`overlay2` + `token_style` |

`mode` derives from `appearance`.

### Non-1:1 accessor cases (need per-site judgment during migration)

- `surface0()` (27 sites) → `border()` for strokes/dividers vs `element_bg()` for subtle fills/hover surfaces.
- `subtext()`/`overlay0()` → two distinct muted tiers (`text_muted()` / `text_subtle()`) to preserve the current look.
- `mauve()` (4 UI sites) → resolved case-by-case (secondary accent or syntax-derived); it is primarily the keyword syntax color now.

---

## Section 2 — Background discovery & transient registry

### At boot — no discovery

Only the one active theme is needed:

- If `theme_name` is the embedded **Catppuccin Mocha** (default) → apply directly from the `include_str!` JSON, **zero disk access**.
- Otherwise → **targeted resolve**: scan theme dirs, parse files only until the named variant is found, apply it, discard the rest. Not found → fall back to embedded Mocha.

Steady state retains a **single `Theme`** (in `ACTIVE` + `UiTheme`). No registry in memory during normal use.

### On settings open — background discovery

`SettingsUi` holds discovery state:

```rust
enum Discovery { Loading, Ready(ThemeRegistry) }
```

- Seeded immediately with always-available entries (embedded Mocha + the currently-active theme) so the picker is never empty.
- Spawns a **background task** (`cx.background_executor().spawn(...)`) that does the filesystem scan (app dir + Zed locations) and all JSON parsing off the main thread.
- On completion, results are handed back via the async context; the entity merges them, sets `Ready`, and `cx.notify()` re-renders. The list shows a subtle "Discovering themes…" affordance while `Loading`.

### Sources & precedence

1. Embedded **Catppuccin Mocha** (`include_str!`), parsed through the same loader path (dogfoods the parser).
2. App themes dir: `~/.config/lgtm/themes/*.json`.
3. Zed scan: `~/.config/zed/themes/` and Zed extension theme dirs, if present.

Deduped by variant name; **later sources override earlier** (a user may override even the bundled Mocha).

### On settings close — cancel & drop

`SettingsUi` owns the `Task` handle and the registry; dropping the entity cancels an in-flight task and frees the registry. The active theme stays applied (lives in `ACTIVE`/`UiTheme`).

### Persistence

Unchanged shape: `settings.theme_name: String` in `config.toml`. Resolve name → theme; if absent (theme removed/renamed, or a config naming a no-longer-bundled family) → fall back to `"Catppuccin Mocha"`.

**Migration note:** existing users whose config names GitHub/Tokyo Night/Solarized will fall back to Catppuccin Mocha on first launch unless that theme is present on disk. Accepted.

### Picker

`settings_ui.rs` switches from `all_names()`/`by_name()` to iterating the registry (same hover-preview/commit flow, now dynamic; optionally grouped by family). With only Mocha present, the list shows one entry.

---

## Section 3 — Consumer migration

1. **Bare accessors (~161 sites).** Rename thread-local accessors to the semantic roles and repoint call sites. Mostly mechanical 1:1 (`blue()`→`accent()`, `subtext()`→`text_muted()`, `peach()`→`warning()`, …). Non-1:1 cases per Section 1. `green()`/`red()` direct UI sites → `success()`/`error()`; diff *rows* go through the tint helpers.
2. **`token_style` → syntax map.** Reads `theme.syntax[Token]` (color + `font_style` + `font_weight`) instead of hardcoded hues. A **Zed-syntax-key → app `Token`** table collapses Zed's finer keys onto the 16 tokens (e.g. `keyword`,`keyword.control`→`Keyword`; `function`,`function.method`→`Function`; `comment`,`comment.doc`→`Comment`). Missing tokens fall back per Section 4.
3. **Diff tints → status roles.** `added_row_bg`→`created.background` (or `created()`@opacity); `added_word_bg` stronger; `removed_*`→`deleted.*`; `selection_bg`→`players[0].selection` (fallback `accent()`@opacity); `void_cell_bg`/`palette_backdrop`→`background()`@opacity; file-status badges → `modified()`/`created()`/`deleted()`.
4. **gpui-component widget theme.** `apply_ui_theme` rewritten to populate `UiTheme` from semantic roles: `primary`/`ring`/`link`=`accent()`, `background`=`editor_bg()`, `border`/`input`=`border()`, `danger`=`error()`, `success`=`success()`, `warning`=`warning()`, popover/sidebar/title_bar=`surface()`, etc. Keeps `Button`/`Input`/`Scrollbar`/`Select` themed (including the `.primary()` [+] button).

The old hue-named `Theme` struct, the 13 Rust constructors, `all_names()`, and `by_name()` are removed once consumers are migrated.

---

## Section 4 — Fallback resolver, error handling & testing

### Fallback resolver

Raw parsing yields `Option<Color>` per role (Zed themes legally omit/`null` roles). A pure, deterministic resolver turns `Style` + `appearance` into a fully-concrete `Theme` so no consumer sees a hole:

- Anchors: `background`/`editor.background` fall back to each other; `text` from `editor.foreground`; `text.muted`/`text.subtle` derived from `text` toward `background`.
- `border`/`element_bg` from `surface`/`background` blends.
- `created`/`deleted`/`modified` → `success`/`error`/`warning` → appearance-keyed green/red/amber defaults.
- `accent` from `text.accent` → first `players[].cursor` → appearance default.
- `.background` tint variants derived from their base role at low opacity when absent.
- Syntax: missing `Token` → `text()` (matches today's `Variable→text`); comments → `text_subtle()`.

This resolver is the single chokepoint where "possibly incomplete Zed JSON" becomes "guaranteed-complete app theme."

### Error handling

Contained per-file: a malformed/unreadable file is skipped with a logged warning; a bad *variant* within a family is skipped while siblings load; a scan error in one location does not abort the others. Embedded Mocha is validated at build/test time so the default can never fail. Boot never panics on theme problems — worst case is Mocha.

### Testing

- **Parser/resolver units:** hex forms (`#rgb`/`#rrggbb`/`#rrggbbaa`), `null` roles, missing roles → resolver fills; every `Token` resolves to a style.
- **Embedded default:** parse+resolve bundled Mocha and assert key roles (shipping a broken default is impossible).
- **Golden/regression:** checked-in fixtures for one Catppuccin + one non-Catppuccin family; snapshot resolved roles to guard the Zed-key→role and syntax-key→`Token` mappings.
- **Registry:** collision/override precedence (later wins), malformed file skipped, name-not-found → Mocha fallback.
- Existing diff/highlight tests updated to new accessor/tint names.

---

## Implementation phasing (for the plan step)

1. Data model + color parsing + resolver + parser tests (no consumer changes).
2. Rewrite `apply_ui_theme`; rename/repoint accessors, `token_style`, tints across the ~161 sites; remove old struct/constructors.
3. Embedded Mocha + boot targeted-resolve + persistence fallback.
4. Background discovery + transient registry + picker wiring in `settings_ui`.

## Out of scope (YAGNI)

- Filesystem watching / live theme reload (manual reopen of settings re-discovers).
- A theme editor/builder UI.
- Persisting resolved palettes into config (targeted boot resolve is used instead).
- Bundling families other than Catppuccin Mocha.
