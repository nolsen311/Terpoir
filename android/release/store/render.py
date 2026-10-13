# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
#
# Usage (from android/): python3 release/store/render.py release/store \
#   app/src/main/res/font/fraunces.ttf app/src/main/res/font/work_sans.ttf
# Needs Pillow.
# Renders the Play store icon and feature graphic from the launcher icon's
# vector (ic_launcher_foreground.xml shapes, ic_launcher_background colour).
import sys
from PIL import Image, ImageDraw, ImageFont

BG = (0x2E, 0x4A, 0x3B, 255)
PAPER = (0xE8, 0xE2, 0xD0, 255)
AMBER = (0xD9, 0xA4, 0x41, 255)
SS = 4  # supersampling

def cubic(p0, p1, p2, p3, n=64):
    out = []
    for i in range(1, n + 1):
        t = i / n
        mt = 1 - t
        out.append((mt**3*p0[0] + 3*mt*mt*t*p1[0] + 3*mt*t*t*p2[0] + t**3*p3[0],
                    mt**3*p0[1] + 3*mt*mt*t*p1[1] + 3*mt*t*t*p2[1] + t**3*p3[1]))
    return out

# ic_launcher_foreground.xml, 108 x 108 viewport:
#   pages   M53,58 C47,54 40,53 32,54 L32,74 C40,73 47,74 53,78 Z
#           M55,58 C61,54 68,53 76,54 L76,74 C68,73 61,74 55,78 Z
#   bubbles circles at (54,49) r3.2, (49,40) r2.2, (57,33) r1.6
PAGES = [
    [(53, 58)] + cubic((53, 58), (47, 54), (40, 53), (32, 54)) + [(32, 74)] + cubic((32, 74), (40, 73), (47, 74), (53, 78)),
    [(55, 58)] + cubic((55, 58), (61, 54), (68, 53), (76, 54)) + [(76, 74)] + cubic((76, 74), (68, 73), (61, 74), (55, 78)),
]
BUBBLES = [(54, 49, 3.2), (49, 40, 2.2), (57, 33, 1.6)]
# The art's bounds in the viewport: x 32..76, y 31.4..78.
ART_X, ART_Y, ART_W, ART_H = 32, 31.4, 44, 46.6
# The pages alone: y 53..78 (their top edge dips to 54 at the outer corners).
PAGES_Y, PAGES_H = 53, 25

def art(draw, scale, dx, dy):
    """Draws the icon art with viewport point (x, y) at (x*scale+dx, y*scale+dy)."""
    for page in PAGES:
        draw.polygon([(x * scale + dx, y * scale + dy) for x, y in page], fill=PAPER)
    for cx, cy, r in BUBBLES:
        x, y = cx * scale + dx, cy * scale + dy
        draw.ellipse((x - r * scale, y - r * scale, x + r * scale, y + r * scale), fill=AMBER)

def icon(path):
    # What a launcher shows of the adaptive icon: the middle 72 of the 108
    # units, full bleed (Play applies its own mask).
    size = 512 * SS
    img = Image.new("RGBA", (size, size), BG)
    scale = size / 72
    art(ImageDraw.Draw(img), scale, -18 * scale, -18 * scale)
    img.resize((512, 512), Image.LANCZOS).convert("RGB").save(path, optimize=True)

def feature(path, title_font, body_font):
    # One line: the book, sized to the wordmark's capitals and sitting on
    # its baseline, then "Terpoir"; the bubbles rise above the book. The
    # tagline has its own line below, centred under the whole lockup.
    w, h = 1024 * SS, 500 * SS
    img = Image.new("RGBA", (w, h), BG)
    d = ImageDraw.Draw(img)
    title = ImageFont.truetype(title_font, 130 * SS)
    body = ImageFont.truetype(body_font, 36 * SS)
    tagline = "A private journal, kept on your phone"
    t_left, t_top, t_right, t_bottom = d.textbbox((0, 0), "Terpoir", font=title, anchor="ls")
    cap = -d.textbbox((0, 0), "T", font=title, anchor="ls")[1]  # cap height
    scale = cap / PAGES_H
    book_w = ART_W * scale
    gap = 34 * SS
    row_w = book_w + gap + (t_right - t_left)
    bw = d.textlength(tagline, font=body)
    bubbles_h = (PAGES_Y - ART_Y) * scale  # bubbles above the pages
    line_gap = 56 * SS
    body_h = 36 * SS
    block_h = bubbles_h + cap + line_gap + body_h
    top = (h - block_h) / 2
    baseline = top + bubbles_h + cap
    x0 = (w - row_w) / 2
    # Pages' bottom edge (y 78) on the baseline.
    art(d, scale, x0 - ART_X * scale, baseline - 78 * scale)
    d.text((x0 + book_w + gap - t_left, baseline), "Terpoir", font=title, fill=PAPER, anchor="ls")
    d.text(((w - bw) / 2, baseline + line_gap), tagline, font=body, fill=(0xC9, 0xC2, 0xAE, 255), anchor="la")
    left = min(x0, (w - bw) / 2)
    assert left >= 40 * SS and top >= 30 * SS, "content runs into the edges"
    img.resize((1024, 500), Image.LANCZOS).convert("RGB").save(path, optimize=True)

out = sys.argv[1]
icon(out + "/icon-512.png")
feature(out + "/feature-graphic-1024x500.png", sys.argv[2], sys.argv[3])
