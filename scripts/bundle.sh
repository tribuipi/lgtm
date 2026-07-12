#!/usr/bin/env bash
# Build LGTM.app (and optionally LGTM.dmg) locally.
#
# Usage:
#   scripts/bundle.sh            # build dist/LGTM.app
#   scripts/bundle.sh --dmg      # also build LGTM.dmg
#
# Mirrors the bundle produced by .github/workflows/release.yml, plus the app
# icon from assets/LGTM.icns. The app is ad-hoc signed (unsigned), so macOS
# will quarantine it on first launch.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MAKE_DMG=false
for arg in "$@"; do
  case "$arg" in
    --dmg) MAKE_DMG=true ;;
    *) echo "unknown option: $arg" >&2; exit 2 ;;
  esac
done

VERSION="0.1.0"
GIT_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
APP="dist/LGTM.app"

echo "==> cargo build --release"
cargo build --release

echo "==> assembling $APP"
rm -rf dist
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/lgtm "$APP/Contents/MacOS/lgtm"
cp assets/LGTM.icns "$APP/Contents/Resources/LGTM.icns"

cat > "$APP/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>LGTM</string>
  <key>CFBundleDisplayName</key>
  <string>LGTM</string>
  <key>CFBundleIdentifier</key>
  <string>com.elliehuxtable.lgtm</string>
  <key>CFBundleExecutable</key>
  <string>lgtm</string>
  <key>CFBundleIconFile</key>
  <string>LGTM</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${GIT_SHA}</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
EOF

echo "==> codesign (ad-hoc)"
codesign --force --deep --sign - "$APP"

echo "built $APP"

if [ "$MAKE_DMG" = true ]; then
  echo "==> creating LGTM.dmg"
  ln -sf /Applications dist/Applications
  hdiutil create -volname LGTM -srcfolder dist -ov -format UDZO LGTM.dmg
  echo "built LGTM.dmg"
fi
