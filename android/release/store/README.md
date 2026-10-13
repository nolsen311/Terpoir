<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Google Play store graphics

- `icon-512.png`: the 512 × 512 app icon, the launcher icon's leaf
  (`app/src/main/res/drawable/ic_launcher_foreground.xml`) on its background
  colour (`#2E4A3B`), full bleed (Play applies its own mask).
- `feature-graphic-1024x500.png`: the 1024 × 500 feature graphic: the
  leaf, "Terpoir" in Fraunces and the tagline in Work Sans (both bundled,
  OFL), kept away from the edges, which Play can crop.

Both are rendered by `render.py` (Pillow) from the same vector and fonts,
so they change with the icon artwork. All-ages: no drug imagery
(`docs/design-decisions.md`, "App store policy").
