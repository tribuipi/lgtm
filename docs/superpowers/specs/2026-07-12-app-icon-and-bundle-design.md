# App Icon + Bundle Script — Design

**Date:** 2026-07-12
**Status:** Approved

## Goal

Give the `LGTM.app` macOS bundle a real icon, and provide a repeatable local
script to build the `.app` (and optionally a `.dmg`). Today the app ships with
no icon: the CI workflow (`.github/workflows/release.yml`) assembles the bundle
inline with an `Info.plist` that has no `CFBundleIconFile`.

## Deliverables

1. `assets/generate_icon.py` — Pillow-based icon generator (committed).
2. `assets/LGTM.icns` + `assets/LGTM.iconset/` — generated icon (committed).
3. `scripts/bundle.sh` — local `.app` builder with `--dmg` flag.
4. `.github/workflows/release.yml` — updated to embed the icon in releases.

## Icon design

macOS Big Sur "squircle" tile:

- **Background:** dark vertical gradient matching the app's dark theme,
  roughly `#1a1b26` → `#15161e` (tokyo-night family).
- **Diff rows:** faint horizontal "code line" bars in the lower-mid area —
  two faint green (additions), one faint red (deletion) — low opacity, read as
  texture behind the mark.
- **Checkmark:** a bold vibrant green ✓ (`#3fb950`, GitHub add-green) with
  rounded caps, as the focal point. Must stay legible at 16px.

## Rendering pipeline

`assets/generate_icon.py`:

- Draws at 4× supersampling, downscales with LANCZOS for crisp anti-aliasing.
- Emits the 10 Apple-required PNGs into `assets/LGTM.iconset/`
  (16/32/128/256/512 at @1x and @2x).
- Runs `iconutil -c icns assets/LGTM.iconset -o assets/LGTM.icns`.
- Only dependency is Pillow (already present locally). Pure-Python drawing —
  no SVG rasterizer needed.

## Bundle script

`scripts/bundle.sh` (bash, `set -euo pipefail`):

1. `cargo build --release`
2. Assemble `dist/LGTM.app`:
   - `Contents/MacOS/lgtm` ← `target/release/lgtm`
   - `Contents/Resources/LGTM.icns` ← `assets/LGTM.icns`
   - `Contents/Info.plist` — mirrors the CI plist plus `CFBundleIconFile`.
3. `codesign --force --deep --sign - dist/LGTM.app` (ad-hoc, unsigned).
4. `--dmg` flag → `hdiutil create` a `LGTM.dmg` with an `/Applications` symlink,
   same as CI.

The plist stays the single source of the CI values (bundle id
`com.elliehuxtable.lgtm`, min system 12.0, etc.).

## CI change

`release.yml`: add `CFBundleIconFile` to the inline plist and copy
`assets/LGTM.icns` into `Contents/Resources/` before codesigning, so shipped
DMGs carry the icon.

## Non-goals

- Real (Developer ID) code signing / notarization — stays ad-hoc as today.
- Refactoring the CI to call `bundle.sh` — kept separate to avoid churn; the
  plist/icon values are simply mirrored.
