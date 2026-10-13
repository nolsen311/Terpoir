<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Google Play store graphics

- `icon-512.png`: the 512 × 512 app icon: the launcher icon's art (an open
  journal with bubbles rising from its spine,
  `app/src/main/res/drawable/ic_launcher_foreground.xml`) on its background
  colour (`#2E4A3B`), cropped to the middle 72 units a launcher shows, full
  bleed (Play applies its own mask).
- `feature-graphic-1024x500.png`: the 1024 × 500 feature graphic. One
  line holds the open journal, sized to the capitals and sitting on the
  baseline, with its bubbles rising above, then "Terpoir" in Fraunces. The
  tagline in Work Sans has its own line below. Both fonts are bundled
  (OFL). Everything is kept away from the edges, which Play can crop.

Both are rendered by `render.py` (Pillow) from the same vector and fonts,
so they change with the icon artwork. All-ages: no drug imagery
(`docs/design-decisions.md`, "App store policy").
