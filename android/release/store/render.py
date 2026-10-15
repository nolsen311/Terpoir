# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
#
# Usage (from android/): python3 release/store/render.py release/store \
#   app/src/main/res/font/fraunces.ttf app/src/main/res/font/work_sans.ttf
#
#   or, for a release announcement's image (.github/workflows/announce.yml):
#   python3 release/store/render.py --card OUT.png VERSION HEADLINE \
#   app/src/main/res/font/fraunces.ttf app/src/main/res/font/work_sans.ttf
# Needs Pillow.
# Renders the Play store icon and feature graphic from the launcher icon's
# vector (ic_launcher_foreground.xml shapes, ic_launcher_background colour),
# and the release card: the feature graphic's lockup with the version and
# the release's headline in place of the tagline.
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

TAGLINE = "A private journal, kept on your phone"


def wrap(d, text, font, width, max_lines):
    """Fits text into one line, or else two balanced lines (max_lines 2),
    each no wider than width."""
    words = text.split()
    if d.textlength(text, font=font) <= width or max_lines == 1:
        assert d.textlength(text, font=font) <= width, f"too long for one line: {text!r}"
        return [text]
    splits = [(" ".join(words[:i]), " ".join(words[i:])) for i in range(1, len(words))]
    fits = [p for p in splits if all(d.textlength(l, font=font) <= width for l in p)]
    assert fits, f"too long for two lines: {text!r}"
    return list(min(fits, key=lambda p: max(d.textlength(l, font=font) for l in p)))


def feature(path, title_font, body_font, size=(1024, 500), version=None, subtitle=None):
    # One line: the book, sized to the wordmark's capitals and sitting on
    # its baseline, then "Terpoir"; the bubbles rise above the book. The
    # tagline has its own line below, centred under the whole lockup. A
    # release card adds the version, in amber, between the two, and shows
    # the release's headline (up to two lines) in place of the tagline.
    w, h = size[0] * SS, size[1] * SS
    img = Image.new("RGBA", (w, h), BG)
    d = ImageDraw.Draw(img)
    title = ImageFont.truetype(title_font, 130 * SS)
    body = ImageFont.truetype(body_font, (40 if version else 36) * SS)
    subtitle = subtitle or TAGLINE
    sub_lines = wrap(d, subtitle, body, (960 if version else w / SS - 160) * SS, 2 if version else 1)
    tagline = max(sub_lines, key=lambda l: d.textlength(l, font=body))
    t_left, t_top, t_right, t_bottom = d.textbbox((0, 0), "Terpoir", font=title, anchor="ls")
    cap = -d.textbbox((0, 0), "T", font=title, anchor="ls")[1]  # cap height
    scale = cap / PAGES_H
    book_w = ART_W * scale
    gap = 34 * SS
    row_w = book_w + gap + (t_right - t_left)
    bw = d.textlength(tagline, font=body)
    bubbles_h = (PAGES_Y - ART_Y) * scale  # bubbles above the pages
    line_gap = 56 * SS
    sub_lead = (54 if version else 36) * SS
    body_h = sub_lead * (len(sub_lines) - 1) + (40 if version else 36) * SS
    ver = ImageFont.truetype(title_font, 60 * SS)
    ver_text = f"Version {version}" if version else None
    ver_gap = 64 * SS
    ver_h = 60 * SS if version else 0
    block_h = bubbles_h + cap + (ver_gap + ver_h if version else 0) + line_gap + body_h
    top = (h - block_h) / 2
    baseline = top + bubbles_h + cap
    x0 = (w - row_w) / 2
    # Pages' bottom edge (y 78) on the baseline.
    art(d, scale, x0 - ART_X * scale, baseline - 78 * scale)
    d.text((x0 + book_w + gap - t_left, baseline), "Terpoir", font=title, fill=PAPER, anchor="ls")
    tag_y = baseline + line_gap
    if version:
        vw = d.textlength(ver_text, font=ver)
        d.text(((w - vw) / 2, baseline + ver_gap), ver_text, font=ver, fill=AMBER, anchor="la")
        tag_y += ver_gap + ver_h
    sub_fill = PAPER if version else (0xC9, 0xC2, 0xAE, 255)
    for i, line in enumerate(sub_lines):
        lw = d.textlength(line, font=body)
        d.text(((w - lw) / 2, tag_y + i * sub_lead), line, font=body, fill=sub_fill, anchor="la")
    left = min(x0, (w - bw) / 2)
    assert left >= 40 * SS and top >= 30 * SS, "content runs into the edges"
    img.resize(size, Image.LANCZOS).convert("RGB").save(path, optimize=True)


def card(path, version, headline, title_font, body_font):
    """The image posted with a release announcement: 1200 x 675 (16:9)."""
    feature(path, title_font, body_font, size=(1200, 675), version=version, subtitle=headline)


if __name__ == "__main__":
    if sys.argv[1] == "--card":
        card(sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5], sys.argv[6])
    else:
        out = sys.argv[1]
        icon(out + "/icon-512.png")
        feature(out + "/feature-graphic-1024x500.png", sys.argv[2], sys.argv[3])
