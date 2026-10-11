<!-- SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception -->
# Terpoir for Android

Kotlin + Jetpack Compose. UI and OS integration only: crypto, the data model,
storage and sync live in the Rust core (`../core`).

## Build

Needs JDK 17+ and the Android SDK (`ANDROID_HOME`).

```sh
./gradlew assemble check lintFossRelease lintPlayRelease
```

| Flavor | Channel | Google services |
|---|---|---|
| `play` | Google Play | Allowed for opt-in features (`Capability`) |
| `foss` | F-Droid, GitHub Releases | None; enforced by `verifyFossDependencies` |

`check` also runs `verify<Variant>Permissions`: the app requests no
permission that is not on the allowlist in `app/build.gradle.kts`.

## Dependencies

Declared in `gradle/libs.versions.toml` with a reason for each. Every
artifact is verified against `gradle/verification-metadata.xml`; after
changing dependencies, regenerate it **with an empty Gradle cache** (a warm
cache hides artifacts, which CI then rejects):

```sh
GRADLE_USER_HOME=$(mktemp -d) ./gradlew --write-verification-metadata sha256 \
    assemble check lintFossRelease lintPlayRelease
```
