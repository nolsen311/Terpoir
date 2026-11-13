<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
<!-- Copyright (C) 2026 The Terpoir Authors -->

# F-Droid

F-Droid builds the `foss` release from source and, with `Binaries` and
`AllowedAPKSigningKeys`, publishes the maintainer-signed APK from GitHub
Releases when its own build matches it byte for byte. The recipe here
builds the way `release.yml` does, so the two match:

- `app.terpoir.yml`: the recipe, in the canonical form `fdroid rewritemeta`
  writes (so no comments; the notes are here). It goes to `fdroiddata` as
  `metadata/app.terpoir.yml` when the app is submitted ("Submitting"
  below). Its `sudo` installs the host tools the build needs ("The
  build"); the build server already has JDK 21.
- `setup.sh` (prebuild): uses the `rustup` on the PATH (Debian's, on
  F-Droid's server) or else installs rustup 1.29.1 (checked against its
  SHA-256), then the toolchain `core/rust-toolchain.toml` pins with the two
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

- `sudo` installs what F-Droid's build image (`buildserver-trixie`, Debian
  trixie) lacks: it has a JDK 21, git, curl, patch and unzip, but no C
  compiler. `build-essential` gives the host `cc` that Rust build scripts
  and proc macros link with (and `cargo install cargo-ndk`); `perl` and
  `make` build the vendored OpenSSL; `xz-utils` unpacks the OCR sources;
  `rustup` is Debian's, as F-Droid's review asked (2026-11-05). `env.sh`
  selects JDK 21. A local `fdroid build` doesn't run `sudo` and uses the
  machine's own tools, so it can't catch a missing package; the fork
  pipeline in "Testing it" can.
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

## Submitting

F-Droid only includes apps that have been maintained for more than three
months: the first merge request (0.10.0, 2026-11-03) was closed for that
on 2026-11-05, and for not following the merge request template. Terpoir's
first commit and first release are 2026-10-10 and 2026-10-13 (v0.1.0), so
resubmit no earlier than **2027-01-13**, with the latest release then.

Once a release signed by `sign-release.sh` (v3 only, alignment preserved)
is published and `fdroid build` of its tag matches it through `Binaries`
("Testing it"):

1. The version's PR (or a follow-up) points the recipe's build entry and
   `CurrentVersion`/`CurrentVersionCode` at that release. `fdroiddata`
   wants the build's `commit` as the tag commit's full hash
   (`git rev-parse 'vX.Y.Z^{commit}'`), not the tag name.
2. On GitLab, fork [fdroiddata](https://gitlab.com/fdroid/fdroiddata)
   (the fork must be public), add this recipe as `metadata/app.terpoir.yml`
   on a new branch, and open a merge request titled "New app: Terpoir" with
   the "Default" template. Follow the template as it is: keep all of its
   text, tick every box that applies, and answer any box left unticked on
   that line, instead of in a separate note. The file must end with a
   newline (`fdroid rewritemeta` fails otherwise). Below the template, the
   description can say:

   > Terpoir: a private, encrypted cannabis journal for adults where it is
   > legal. GPL-3.0-or-later, no account, no server, no trackers; every
   > network feature is opt-in (an OpenStreetMap map from OpenFreeMap, or
   > MapTiler with the user's own key; OpenStreetMap address search through
   > Photon). I'm the developer.
   >
   > Reproducible: F-Droid's build matches the signed APK on GitHub
   > Releases (`Binaries`, `AllowedAPKSigningKeys`), checked locally with
   > `fdroid build`. The Rust core is built with a pinned toolchain
   > (`android/fdroid/setup.sh`), and the Tesseract OCR libraries from
   > pinned, hash-checked source tarballs (`android/ocr-native/build.sh`);
   > they can come in as srclibs instead if you prefer. `scandelete` removes
   > encrypted test fixtures the build doesn't read.
   >
   > Anti-features: none expected. The map is optional; OpenFreeMap is free
   > software, and MapTiler is used only with the user's own key.

3. Answer reviewers' questions; the likely ones are srclibs for the OCR
   tarballs and the anti-features
   (`docs/follow-ups.md`, "F-Droid anti-features for the map").

## Each release

`AutoUpdateMode: Version` and `UpdateCheckMode: Tags` let F-Droid's bot add
each `vX.Y.Z` tag's build to `fdroiddata` by itself, copying the last build
entry. F-Droid then builds it, compares it with the signed APK on GitHub
Releases and, when they match, publishes that APK, usually within a few days
of the tag. Nothing else is needed per release, as long as:

- the release is signed with `sign-release.sh` (any other signing breaks the
  match);
- the version PR adds `fastlane/metadata/android/en-US/changelogs/<code>.txt`
  (`release.yml` requires it);
- the build steps don't change. When they do (a new step, a new OCR AAR
  version), change the recipe here and send the same change to
  `fdroiddata` by merge request before the tag.

A build that fails or doesn't match shows on F-Droid's build status pages
and in `fdroiddata`'s pipeline; the published version then stays at the last
one that matched.

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

To test on F-Droid's own build image (which a local `fdroid build` can't
stand in for: it skips `sudo`), push the recipe to a branch of your
`fdroiddata` fork that has **no open merge request**. Its CI then runs the
whole check, `fdroid build` included, on `buildserver-trixie`, without
anything going to F-Droid's reviewers. The 0.10.0 merge request failed
there (2026-11-03) on a missing `cc`, which a local build couldn't show.
