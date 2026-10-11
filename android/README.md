<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
# Terpoir for Android

Kotlin + Jetpack Compose. UI and OS integration only: crypto, the data model,
storage and sync live in the Rust core (`../core`).

## Build

Needs JDK 17+, the Android SDK (`ANDROID_HOME`), and for the Rust core:
`rustup` (the toolchain is pinned in `../core/rust-toolchain.toml`), the
Android targets, [cargo-ndk](https://crates.io/crates/cargo-ndk), and the NDK
version named in `app/build.gradle.kts`. From the repository root,
`.github/scripts/setup-android-core.sh` installs them on Linux.

The build compiles the core for `arm64-v8a` and `x86_64` and generates its
Kotlin bindings (package `app.terpoir.core`); both are build outputs, never
committed.

```sh
./gradlew assemble check lintFossRelease lintPlayRelease assembleFossDebugAndroidTest
```

| Flavor | Channel | Google services |
|---|---|---|
| `play` | Google Play | Allowed for opt-in features (`Capability`) |
| `foss` | F-Droid, GitHub Releases | None; enforced by `verifyFossDependencies` |

On-device tests (`app/src/androidTest`) run in CI on an Android 11 emulator.

`check` also runs `verify<Variant>Permissions`: the app requests no
permission that is not on the allowlist in `app/build.gradle.kts`.

## Dependencies

Declared in `gradle/libs.versions.toml` with a reason for each. Every
artifact is verified against `gradle/verification-metadata.xml`; after
changing dependencies, regenerate it **with an empty Gradle cache** (a warm
cache hides artifacts, which CI then rejects):

```sh
GRADLE_USER_HOME=$(mktemp -d) ./gradlew --write-verification-metadata sha256 \
    assemble check lintFossRelease lintPlayRelease assembleFossDebugAndroidTest
```
