<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# On-device text recognition

Label and receipt scans read text on the phone. Nothing is sent anywhere,
and the photo is discarded once it has been read
(`docs/design-decisions.md`, "Rapid product entry").

## Engine: Tesseract (both flavors)

- **Library:** [Tesseract4Android](https://github.com/adaptech-cz/Tesseract4Android)
  4.9.0, `cz.adaptech.tesseract4android:tesseract4android` (Apache-2.0).
  It bundles Tesseract 5.5.1 (Apache-2.0), Leptonica 1.85.0, libpng 1.6.48
  and libjpeg 9f. Their license texts ship in the app at
  `app/src/main/assets/licenses/`.
- **Source:** published only on JitPack, which builds it from the tagged
  source. `settings.gradle.kts` restricts JitPack to this one group, and
  `gradle/verification-metadata.xml` pins its checksum.
- **Native libraries:** 16 KB page aligned. The JitPack build calls
  Android's logger for a few fixed status lines (for example "Initialized
  Tesseract API with language=eng"), never image or text content; R8
  cannot strip native calls.
- **Built from source:** `android/ocr-native/` rebuilds the same AAR from
  pinned upstream source with logging compiled out. The `foss` flavor uses
  it when given `-Pterpoir.ocr.aar=<path>`, as F-Droid's build does; app CI
  and the Play flavor use the JitPack AAR.

### Model

`app/src/main/assets/tessdata/eng.traineddata` is the English model from
[tesseract-ocr/tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast)
at tag `4.1.0` (Apache-2.0).

| File | SHA-256 |
|---|---|
| `eng.traineddata` | `7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2` |

The app copies it once to its no-backup directory, because Tesseract reads
models from files.
