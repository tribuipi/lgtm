#!/usr/bin/env python3
"""Generate the LGTM macOS app icon.

Draws a Big Sur "squircle" tile with a dark diff-themed background and a bold
green checkmark, then emits an .iconset and packs it into LGTM.icns via
`iconutil`.

Usage:
    python3 assets/generate_icon.py

Requires: Pillow, and `iconutil` (ships with macOS).
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw

# --- palette (tokyo-night dark + GitHub diff colors) -----------------------
BG_TOP = (26, 27, 38)        # #1a1b26
BG_BOTTOM = (21, 22, 30)     # #15161e
ADD_GREEN = (63, 185, 80)    # #3fb950  (checkmark + faint add rows)
DEL_RED = (248, 81, 73)      # #f85149  (faint delete row)

SS = 4                        # supersampling factor
BASE = 1024                   # logical icon size
N = BASE * SS                 # working canvas size

# Apple's macOS icon grid: within a 1024 canvas, the rounded-rect body is
# 824x824 (824/1024 ~= 0.8047), leaving a transparent margin so the icon sits
# the same size as every other app in the Dock. Filling the whole canvas makes
# the icon look oversized next to its neighbors.
BODY_FRAC = 824 / 1024

ASSETS = Path(__file__).resolve().parent
ICONSET = ASSETS / "LGTM.iconset"
ICNS = ASSETS / "LGTM.icns"


def rounded_mask(size: int, radius: int) -> Image.Image:
    """Alpha mask for a rounded square (Apple's rounded-rect approximation)."""
    mask = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def vertical_gradient(size: int, top: tuple, bottom: tuple) -> Image.Image:
    grad = Image.new("RGB", (1, size))
    for y in range(size):
        t = y / (size - 1)
        grad.putpixel(
            (0, y),
            tuple(round(top[i] + (bottom[i] - top[i]) * t) for i in range(3)),
        )
    return grad.resize((size, size))


def draw_diff_rows(draw: ImageDraw.ImageDraw, size: int) -> None:
    """Faint code-line bars behind the checkmark: two adds, one delete."""
    # (top_fraction, width_fraction, color, alpha)
    rows = [
        (0.30, 0.42, ADD_GREEN, 46),
        (0.45, 0.30, ADD_GREEN, 46),
        (0.60, 0.36, DEL_RED, 46),
    ]
    left = int(0.20 * size)
    height = int(0.055 * size)
    for top_frac, width_frac, color, alpha in rows:
        top = int(top_frac * size)
        right = left + int(width_frac * size)
        draw.rounded_rectangle(
            (left, top, right, top + height),
            radius=height // 2,
            fill=color + (alpha,),
        )


def draw_check(draw: ImageDraw.ImageDraw, size: int) -> None:
    """Bold green checkmark with rounded caps."""
    # Control points as fractions of the tile.
    p1 = (0.30 * size, 0.56 * size)   # start (left)
    p2 = (0.44 * size, 0.70 * size)   # elbow (bottom)
    p3 = (0.72 * size, 0.36 * size)   # end (top-right)
    width = int(0.085 * size)
    draw.line([p1, p2, p3], fill=ADD_GREEN + (255,), width=width, joint="curve")
    r = width // 2
    for cx, cy in (p1, p2, p3):
        draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=ADD_GREEN + (255,))


def render() -> Image.Image:
    # Render the icon body as a self-contained squircle tile...
    body = round(BODY_FRAC * N)

    # Background layer (opaque gradient), masked to the squircle.
    bg = vertical_gradient(body, BG_TOP, BG_BOTTOM).convert("RGBA")

    # Foreground layer (diff rows + check) drawn with alpha, then composited.
    fg = Image.new("RGBA", (body, body), (0, 0, 0, 0))
    d = ImageDraw.Draw(fg)
    draw_diff_rows(d, body)
    draw_check(d, body)
    tile = Image.alpha_composite(bg, fg)

    # Apply squircle mask. Corner radius ~22.4% is the macOS-ish look.
    mask = rounded_mask(body, int(0.2237 * body))
    tile.putalpha(mask)

    # ...then center it on a transparent canvas so the Apple margin is preserved.
    icon = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    offset = (N - body) // 2
    icon.paste(tile, (offset, offset))

    return icon.resize((BASE, BASE), Image.Resampling.LANCZOS)


def write_iconset(icon: Image.Image) -> None:
    ICONSET.mkdir(parents=True, exist_ok=True)
    # (logical size, scale) -> Apple filename convention.
    specs = [
        (16, 1), (16, 2),
        (32, 1), (32, 2),
        (128, 1), (128, 2),
        (256, 1), (256, 2),
        (512, 1), (512, 2),
    ]
    for size, scale in specs:
        px = size * scale
        name = f"icon_{size}x{size}{'' if scale == 1 else '@2x'}.png"
        icon.resize((px, px), Image.Resampling.LANCZOS).save(ICONSET / name)


def pack_icns() -> None:
    subprocess.run(
        ["iconutil", "-c", "icns", str(ICONSET), "-o", str(ICNS)],
        check=True,
    )


def main() -> int:
    icon = render()
    write_iconset(icon)
    pack_icns()
    print(f"wrote {ICONSET}")
    print(f"wrote {ICNS}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
