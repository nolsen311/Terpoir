# Terpoir for iOS

Planned in `docs/roadmap.md` ("iOS"): a core MVP in Swift and SwiftUI over
the same Rust core as Android. Nothing the core owns (crypto, the data
model, sync, merging) is reimplemented in Swift.

## The core (`TerpoirCore`)

`TerpoirCore/` is a Swift package around the Rust core: the static library
`TerpoirCoreFFI.xcframework` (iPhones and the Apple-silicon simulator) and
the Swift bindings UniFFI generates. Both are build products:

```sh
ios/scripts/build-core.sh        # macOS with Xcode and rustup
cd ios/TerpoirCore && xcodebuild test -scheme TerpoirCore \
    -destination 'platform=iOS Simulator,name=iPhone 16'
```

The script builds `terpoir-ffi` for `aarch64-apple-ios` and
`aarch64-apple-ios-sim` with the pinned toolchain (`core/rust-toolchain.toml`),
SQLCipher and OpenSSL from source as on Android, and SQLCipher's logging
compiled out (`core/.cargo/config.toml`). The bindings come from the
workspace's own `uniffi-bindgen`, run on a host build of the same library.

CI (`.github/workflows/ios.yml`) does the same on a macOS runner and runs
`Tests/TerpoirCoreTests` on an iOS simulator. Nothing here is signed.

## The app

`project.yml` describes the Xcode project; XcodeGen generates
`Terpoir.xcodeproj` from it (never committed):

```sh
ios/scripts/build-core.sh
xcodegen generate --spec ios/project.yml
ios/scripts/test-on-simulator.sh 1 -project ios/Terpoir.xcodeproj -scheme Terpoir
```

`App/` is the SwiftUI app (bundle ID `app.terpoir`, iOS 17+, iPhone only
for now), `AppTests/` its tests. Simulator builds are signed ad hoc (no
certificate or account); real signing
happens only in the release workflow (`docs/design-decisions.md`, "Signing
keys").
