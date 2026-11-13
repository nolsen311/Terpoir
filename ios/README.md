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
