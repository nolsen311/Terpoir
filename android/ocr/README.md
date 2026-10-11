<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# On-device text recognition

Label and receipt scans read text on the phone. Nothing is sent anywhere,
and the photo is discarded once it has been read
(`docs/design-decisions.md`, "Rapid product entry").

This directory is also the Play build's on-demand module (`:ocr`).

## Modules

| Module | What it is |
|---|---|
| `:ocr-tesseract` (`../ocr-tesseract/`) | `TesseractReader`, the wrapper both builds use. Compiles against Tesseract4Android without shipping it. |
| `:ocr` (here) | Play only: an on-demand feature module with Tesseract4Android and the wrapper. Its foss variant is empty. |
| `:ocr-model` (`../ocr-model/`) | The model. On Play, the on-demand asset pack `ocr_model`; the foss app bundles the same file. |

The scanner reads text through one interface, `TextEngine`
(`app/src/main/kotlin/app/terpoir/scan/`), and `Distribution` picks the
engine:

- **foss:** `BundledTextEngine`, with Tesseract and the model in the APK.
- **Play:** `PlayTesseractEngine`. Before the first label or receipt scan
  the app offers to download `:ocr` and `ocr_model` from Google Play; the
  Play Store fetches them, so the app needs no INTERNET permission.
  `GoogleTextEngine` (ML Kit through Play services) replaces it while the
  user has "Google text recognition" on in the Privacy Center.

`./gradlew bundlePlayRelease` builds the only artifact with all three
modules; `verifyPlayReleaseBundle` (part of `check`) checks that Tesseract
is only in `:ocr` and the model only in the pack. A Play debug APK
installed directly has neither: test on-demand delivery with
[bundletool](https://developer.android.com/tools/bundletool)'s
`build-apks --local-testing` and `install-apks`.

## Engine: Tesseract

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
  and the Play module use the JitPack AAR.

### Model

`ocr-model/src/main/assets/tessdata/eng.traineddata` is the English model from
[tesseract-ocr/tessdata_fast](https://github.com/tesseract-ocr/tessdata_fast)
at tag `4.1.0` (Apache-2.0).

| File | SHA-256 |
|---|---|
| `eng.traineddata` | `7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2` |

The foss app copies it once to its no-backup directory, because Tesseract
reads models from files. On Play, Tesseract reads it where the asset pack
was installed.
