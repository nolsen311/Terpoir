<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# Bundled fonts

The app bundles its fonts in `app/src/main/res/font/` rather than using
downloadable Google Fonts. Downloadable fonts go through Play services,
which the FOSS flavor does not have and which would contact Google.

Both fonts are under the SIL Open Font License 1.1. The license texts ship
inside the app at `app/src/main/assets/licenses/`.

| File | Source | SHA-256 of the source file |
|---|---|---|
| `fraunces.ttf` | `ofl/fraunces/Fraunces[SOFT,WONK,opsz,wght].ttf` | `177ff6c0f14e5550a3c624247cd1189611d4eb65d000b14944c63d967958abbb` |
| `work_sans.ttf` | `ofl/worksans/WorkSans[wght].ttf` | `f50f61f2ba738e239442d40bf1069adb195c224b6a5a73a581fc2f3ed62a9f63` |

Both come from [google/fonts](https://github.com/google/fonts) at commit
`bd8f81ddb5c74d5c8897b36ad88b440266245103`.

## How the bundled files were made

The fonts were processed with fontTools 4.60.1:

1. Fraunces's decorative axes are pinned to their defaults, `SOFT=0` and
   `WONK=0`. The weight and optical-size axes stay variable.
2. Both fonts are subset to the Latin range Google Fonts uses for its
   `latin` subset. All OpenType layout features (kerning, ligatures) are
   kept, and hinting is dropped.

```sh
python -c "
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
f = instancer.instantiateVariableFont(TTFont('Fraunces.ttf'), {'SOFT': 0, 'WONK': 0})
f.save('fraunces_pinned.ttf')
"
LATIN="U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD"
pyftsubset fraunces_pinned.ttf --unicodes="$LATIN" --layout-features='*' --no-hinting --output-file=fraunces.ttf
pyftsubset WorkSans.ttf --unicodes="$LATIN" --layout-features='*' --no-hinting --output-file=work_sans.ttf
```

Results:

| File | Size | SHA-256 |
|---|---|---|
| `fraunces.ttf` | 116,828 bytes | `2b50ea93208128d86f30de3e76da2cb18e01c7c9372f694d7c69bf221ed74794` |
| `work_sans.ttf` | 175,608 bytes | `246ad27f5b3750a2ae53c34528d388147e819760858b2d4ee2bbdf3a3e47a101` |

Fraunces's default instance is Black (weight 900), so the theme
(`ui/theme/Type.kt`) always sets the weight axis explicitly.
