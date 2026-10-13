# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
#
# Usage (from android/): python3 release/store/render.py release/store \
#   app/src/main/res/font/fraunces.ttf app/src/main/res/font/work_sans.ttf
# Needs Pillow.
# Renders the Play store icon and feature graphic from the launcher icon's
# vector (ic_launcher_foreground.xml path, ic_launcher_background colour).
import sys
from PIL import Image, ImageDraw, ImageFont

BG = (0x2E, 0x4A, 0x3B, 255)
LEAF = (0xE8, 0xE2, 0xD0, 255)
SS = 4  # supersampling

def cubic(p0, p1, p2, p3, n=64):
    out = []
    for i in range(1, n + 1):
        t = i / n
        mt = 1 - t
        out.append((mt**3*p0[0] + 3*mt*mt*t*p1[0] + 3*mt*t*t*p2[0] + t**3*p3[0],
                    mt**3*p0[1] + 3*mt*mt*t*p1[1] + 3*mt*t*t*p2[1] + t**3*p3[1]))
    return out

# M54,30 C66,40 70,56 54,78 C38,56 42,40 54,30 Z  (108x108 viewport)
LEAF_PATH = [(54, 30)] + cubic((54, 30), (66, 40), (70, 56), (54, 78)) + cubic((54, 78), (38, 56), (42, 40), (54, 30))

def leaf(draw, scale, dx, dy):
    draw.polygon([(x * scale + dx, y * scale + dy) for x, y in LEAF_PATH], fill=LEAF)

def icon(path):
    size = 512 * SS
    img = Image.new("RGBA", (size, size), BG)
    leaf(ImageDraw.Draw(img), size / 108, 0, 0)
    img.resize((512, 512), Image.LANCZOS).convert("RGB").save(path, optimize=True)

def feature(path, title_font, body_font):
    w, h = 1024 * SS, 500 * SS
    img = Image.new("RGBA", (w, h), BG)
    d = ImageDraw.Draw(img)
    title = ImageFont.truetype(title_font, 120 * SS)
    tagline = "A private journal, kept on your phone"
    body = ImageFont.truetype(body_font, 34 * SS)
    tw = d.textlength("Terpoir", font=title)
    bw = d.textlength(tagline, font=body)
    leaf_w = 28 * (300 * SS / 48)  # the leaf is 28 x 48 units; 300 px tall
    gap = 56 * SS
    block = leaf_w + gap + max(tw, bw)
    x0 = (w - block) / 2
    # Leaf: 48 units tall (y 30..78), 300 px, centred vertically.
    scale = 300 * SS / 48
    leaf(d, scale, x0 - 40 * scale, (h - 300 * SS) / 2 - 30 * scale)
    tx = x0 + leaf_w + gap
    d.text((tx, 140 * SS), "Terpoir", font=title, fill=LEAF)
    d.text((tx + 4 * SS, 300 * SS), tagline, font=body, fill=(0xC9, 0xC2, 0xAE, 255))
    assert tx + max(tw, bw) <= w - 40 * SS, "text runs off the graphic"
    img.resize((1024, 500), Image.LANCZOS).convert("RGB").save(path, optimize=True)

out = sys.argv[1]
icon(out + "/icon-512.png")
feature(out + "/feature-graphic-1024x500.png", sys.argv[2], sys.argv[3])
