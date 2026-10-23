<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# F-Droid

F-Droid builds the `foss` release from source and, with `Binaries` and
`AllowedAPKSigningKeys`, publishes the maintainer-signed APK from GitHub
Releases when its own build matches it byte for byte. The recipe here
builds the way `release.yml` does, so the two match:

- `app.terpoir.yml`: the recipe, in the canonical form `fdroid rewritemeta`
  writes (so no comments; the notes are here). It goes to `fdroiddata` as
  `metadata/app.terpoir.yml` when the app is submitted (1.0.0).
- `setup.sh` (prebuild): installs rustup 1.29.1 (checked against its
  SHA-256), the toolchain `core/rust-toolchain.toml` pins with the two
  Android targets, the `cargo-ndk` version CI pins
  (`.github/scripts/setup-android-core.sh`, built with `--locked`), and the
  SDK packages the OCR libraries need (`android/ocr-native/build.sh
  --sdk-packages`).
- `env.sh`: sourced first in the build step, so Cargo is on the `PATH`,
  JDK 21 is used, and `SOURCE_DATE_EPOCH` is the commit time, as in CI.
  fdroid sets that variable before cloning, from the metadata file's
  commit, which would change the Rust core's embedded OpenSSL build date.
- `fastlane/metadata/android/en-US/` (repository root): the listing. The
  descriptions are the Play listing's (`docs/play-listing.md`) with the
  `foss` build's differences: Tesseract built in, the optional online map,
  nothing from Google. `changelogs/<versionCode>.txt` holds the version's
  Play notes; `release.yml` stops a tag without it.
  `images/phoneScreenshots/` are drawn from the real screens (below).

## Screenshots

`StoreScreenshots` (`android/app/src/testFoss/`) draws the store
screenshots from the app's own screens with Robolectric, on the JVM, over a
demo journal (`DemoJournal.kt`: made-up brands and shop, two weeks of
sessions) opened on the core built for the computer running it. Nothing is
mocked up: the journal, Insights and the Privacy Center are the real code
and the real core. Regenerate them after a visible change:

```bash
cd android
./gradlew -Pterpoir.screenshots=../fastlane/metadata/android/en-US/images/phoneScreenshots \
    :app:testFossDebugUnitTest --tests app.terpoir.StoreScreenshots
```

The dates in them are relative to the day they are drawn. Without the
property, Gradle leaves the class out of the unit tests.

## The build

- `sudo` installs JDK 21. F-Droid's build server (Debian trixie) already
  defaults to it; installing it by name keeps the build on 21 if that
  default moves, and `env.sh` selects it.
- `scandelete` removes the compatibility fixtures
  (`core/crates/terpoir-ffi/tests/fixtures`), encrypted test data the
  scanner reports as binaries. The release build doesn't read them.
- `build` builds the OCR AAR from pinned, hash-checked source tarballs
  (`android/ocr-native/`), then runs Gradle itself rather than through the
  `gradle:` field, because Gradle's cargo-ndk step needs `env.sh`'s
  `PATH`. `-Pterpoir.reproducible` gives the path-independent Rust core;
  `-Pterpoir.ocr.aar` points the `foss` flavor at the from-source AAR.
- `output` is the unsigned APK, which F-Droid compares with the signed one
  after copying the signature across.

Whether the OCR source tarballs come in as `srclibs` instead of being
downloaded by `build.sh` is for F-Droid's reviewers; `OCR_DOWNLOADS` supports
either.

## Updating it

`AutoUpdateMode: Version` and `UpdateCheckMode: Tags` let F-Droid's bot add
each `vX.Y.Z` tag's build to `fdroiddata` by itself. Change the recipe here
when the build changes (a new step, a new OCR AAR version), and the same
change goes to `fdroiddata` by merge request.

## Testing it

With [fdroidserver](https://gitlab.com/fdroid/fdroidserver) and a sparse
checkout of `fdroiddata`'s `config/` (for the categories):

```bash
cp android/fdroid/app.terpoir.yml <fdroiddata>/metadata/
cd <fdroiddata>
fdroid rewritemeta app.terpoir   # no diff: still canonical
fdroid lint app.terpoir
fdroid scanner app.terpoir:<versionCode>
fdroid build app.terpoir:<versionCode>
```

To test a commit before its tag exists, set the build's `commit` to it in
the copy and commit the copy (fdroid dates the build from the metadata
file's commit until `env.sh` replaces it). Put F-Droid's
[`gradlew-fdroid`](https://gitlab.com/fdroid/gradlew-fdroid) on the `PATH`
as `gradle`, as on its build server. Until a release is built from the
same commit, `Binaries` points at an older APK and the final comparison
fails; compare `build/app.terpoir/android/app/build/outputs/apk/foss/release/app-foss-release-unsigned.apk`
with `release.yml`'s build instead (2026-10-22: byte-identical).
