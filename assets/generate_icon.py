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


def draw_diff_rows(draw: ImageDraw.ImageDraw) -> None:
    """Faint code-line bars behind the checkmark: two adds, one delete."""
    # (top_fraction, width_fraction, color, alpha)
    rows = [
        (0.30, 0.42, ADD_GREEN, 46),
        (0.45, 0.30, ADD_GREEN, 46),
        (0.60, 0.36, DEL_RED, 46),
    ]
    left = int(0.20 * N)
    height = int(0.055 * N)
    for top_frac, width_frac, color, alpha in rows:
        top = int(top_frac * N)
        right = left + int(width_frac * N)
        draw.rounded_rectangle(
            (left, top, right, top + height),
            radius=height // 2,
            fill=color + (alpha,),
        )


def draw_check(draw: ImageDraw.ImageDraw) -> None:
    """Bold green checkmark with rounded caps."""
    # Control points as fractions of the canvas.
    p1 = (0.30 * N, 0.56 * N)   # start (left)
    p2 = (0.44 * N, 0.70 * N)   # elbow (bottom)
    p3 = (0.72 * N, 0.36 * N)   # end (top-right)
    width = int(0.085 * N)
    draw.line([p1, p2, p3], fill=ADD_GREEN + (255,), width=width, joint="curve")
    r = width // 2
    for cx, cy in (p1, p2, p3):
        draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=ADD_GREEN + (255,))


def render() -> Image.Image:
    # Background layer (opaque gradient), masked to the squircle.
    bg = vertical_gradient(N, BG_TOP, BG_BOTTOM).convert("RGBA")

    # Foreground layer (diff rows + check) drawn with alpha, then composited.
    fg = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    d = ImageDraw.Draw(fg)
    draw_diff_rows(d)
    draw_check(d)
    icon = Image.alpha_composite(bg, fg)

    # Apply squircle mask. Corner radius ~22.4% is the macOS-ish look.
    mask = rounded_mask(N, int(0.2237 * N))
    icon.putalpha(mask)

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
