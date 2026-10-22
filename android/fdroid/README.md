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
- `env.sh`: sourced first in the build step, so Cargo is on the `PATH` and
  JDK 21 is used, as in CI.
- `fastlane/metadata/android/en-US/` (repository root): the listing. The
  descriptions are the Play listing's (`docs/play-listing.md`) with the
  `foss` build's differences: Tesseract built in, the optional online map,
  nothing from Google. `changelogs/<versionCode>.txt` holds the version's
  Play notes; `release.yml` stops a tag without it.

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
the copy.
