<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
# OCR native libraries from source

On-device text recognition for receipt and label scans (Phase 3, rapid
product entry) uses [Tesseract4Android] (Apache-2.0): Tesseract with
Leptonica, libpng and libjpeg, plus a JNI wrapper and a small Java API. The
published AAR comes from JitPack and carries native libraries that JitPack
built. F-Droid builds apps from source and does not take prebuilt native
code, so `build.sh` here rebuilds the same library from pinned upstream
source tarballs, as a drop-in replacement for the JitPack AAR.

| File | What it is |
|---|---|
| `sources.txt` | Every input: version, SHA-256 and URL |
| `build.sh` | The recipe: fetch, verify, build, package |
| `SHA256SUMS` | Expected hashes of the outputs (the reproducibility record) |

## What it builds

- **Sources:** Tesseract 5.5.3, Leptonica 1.87.0, libpng 1.6.59 and IJG
  libjpeg 9f from their release tarballs. From Tesseract4Android 4.9.0 only
  its own code is used (CMake files, JNI wrapper, Java API); its bundled
  copies of the four libraries are never unpacked. Each tarball is checked
  against its SHA-256 before it is used.
- **Toolchain:** the NDK version `app/build.gradle.kts` pins, for the app's
  `minSdk`, `arm64-v8a` and `x86_64` only; CMake 3.31.6 from the SDK; JDK 17+
  compiling the Java API for Java 17 against `platforms;android-37.0`.
- **Checks:** every library has 16 KB-aligned LOAD segments, and none links
  `liblog`. The wrapper's logcat macros are compiled out, as for the Rust
  core (`docs/design-decisions.md`, "Logs and crash reports").
- **Output:** `tesseract4android-4.9.0.aar` (same classes, package and
  library names as the JitPack AAR: `libjpeg.so`, `libpngx.so`,
  `libleptonica.so`, `libtesseract.so`), the loose `jniLibs/<abi>/*.so`, and
  `SHA256SUMS`.

## Running it

```sh
sdkmanager $(android/ocr-native/build.sh --sdk-packages)
android/ocr-native/build.sh out/
```

Needs `ANDROID_HOME`, a JDK 17 or newer, `curl`, `sha256sum`, `tar`, `xz` and
`python3`. Set `OCR_DOWNLOADS` to a directory that already holds the
tarballs to build offline (they are still checked against `sources.txt`).
Takes a few minutes on a 4-core machine.

Reproducible: from any directory, and wherever the SDK is installed, the
outputs match `SHA256SUMS` byte for byte. Compiled-in paths (including the
NDK's, which the NDK's debug info and so the build ID would otherwise
carry) are mapped to fixed names, `SOURCE_DATE_EPOCH` is
pinned, the libraries are stripped, and the AAR and its `classes.jar` are
written with fixed timestamps and order, uncompressed.

## CI

`.github/workflows/ocr-native.yml` (**OCR native libraries**) runs the
recipe on two runners from checkouts at different paths (the second also
reaching the SDK through a different path). It requires the
two results to be identical and to match `SHA256SUMS`, and keeps them as a
workflow artifact for 90 days. It runs weekly, on manual dispatch, and when
anything here (or the NDK pin) changes. It is not a required check, and the
app's own CI does not wait for it: the main build keeps using the JitPack
AAR.

To bump a version: edit its line in `sources.txt` (version, URL, SHA-256 of
the new tarball), push, and copy the hashes the workflow prints into
`SHA256SUMS`.

## Using it from the app (lands with the scanner)

The app does not depend on Tesseract4Android yet. When the scanner adds it,
the dependency gets a Gradle property switch for the `foss` flavor:

```kotlin
// app/build.gradle.kts (planned)
val ocrAar = providers.gradleProperty("terpoir.ocr.aar").orNull
dependencies {
    if (ocrAar != null) {
        "fossImplementation"(files(ocrAar))  // built here, from source
    } else {
        "fossImplementation"(libs.tesseract4android)  // JitPack
    }
}
```

On Play, Tesseract is not in the base app: it comes as an on-demand
feature module (with the language data as an asset pack) that depends on
the JitPack AAR (`docs/design-decisions.md`, Rapid product entry).

The JitPack repository is declared only when the property is unset, so a
from-source build never contacts JitPack. CI keeps building without the
property.

### F-Droid recipe (outline)

```yaml
Builds:
  - versionName: x.y.z
    versionCode: N
    commit: vx.y.z
    subdir: android/app
    gradle:
      - foss
    ndk: r30
    prebuild:
      - sdkmanager $(../ocr-native/build.sh --sdk-packages)
    build:
      - ../ocr-native/build.sh ../ocr-native/out
    gradleprops:
      - terpoir.ocr.aar=../ocr-native/out/tesseract4android-4.9.0.aar
```

The exact fields (and whether the tarballs come in as `srclibs` instead of
being downloaded by the script) are settled with F-Droid when the app is
submitted (Phase 4); `OCR_DOWNLOADS` supports either way. F-Droid can check
its build against `SHA256SUMS`.

[Tesseract4Android]: https://github.com/adaptech-cz/Tesseract4Android
