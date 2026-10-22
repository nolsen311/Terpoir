#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Prepares a machine with the Android SDK, such as F-Droid's build server,
# to build the foss release the way release.yml does (android/fdroid/
# README.md): rustup (pinned, its download checked against a SHA-256), the
# Rust toolchain core/rust-toolchain.toml pins with the Android targets,
# cargo-ndk (the version CI pins, built with --locked), and the SDK
# packages the OCR libraries need (android/ocr-native/build.sh). Installs
# nothing that is already there. Then `source android/fdroid/env.sh`.
#
# Usage: android/fdroid/setup.sh   (ANDROID_HOME set)

set -euo pipefail

RUSTUP_VERSION=1.29.1
RUSTUP_SHA256=dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71

root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
: "${ANDROID_HOME:?ANDROID_HOME must be set}"
CARGO_HOME=${CARGO_HOME:-$HOME/.cargo}
export CARGO_HOME
export PATH="$CARGO_HOME/bin:$PATH"

if ! command -v rustup >/dev/null; then
    work=$(mktemp -d)
    curl -sSfL --proto '=https' --tlsv1.2 -o "$work/rustup-init" \
        "https://static.rust-lang.org/rustup/archive/$RUSTUP_VERSION/x86_64-unknown-linux-gnu/rustup-init"
    echo "$RUSTUP_SHA256  $work/rustup-init" | sha256sum -c --quiet -
    chmod +x "$work/rustup-init"
    "$work/rustup-init" -y --quiet --no-modify-path --profile minimal --default-toolchain none
    rm -rf "$work"
fi

toolchain=$(sed -n 's/^channel = "\(.*\)"$/\1/p' core/rust-toolchain.toml)
rustup toolchain install "$toolchain" --profile minimal \
    --target aarch64-linux-android --target x86_64-linux-android

cargo_ndk=$(sed -n 's/^CARGO_NDK_VERSION=//p' .github/scripts/setup-android-core.sh)
if ! cargo ndk --version 2>/dev/null | grep -qx "cargo-ndk ${cargo_ndk}"; then
    cargo +"$toolchain" install --locked "cargo-ndk@${cargo_ndk}"
fi

sdkmanager=$(command -v sdkmanager || echo "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager")
read -ra packages <<< "$(android/ocr-native/build.sh --sdk-packages)"
{ yes || true; } | "$sdkmanager" --licenses >/dev/null
"$sdkmanager" "${packages[@]}" >/dev/null
echo "Ready: Rust $toolchain, cargo-ndk $cargo_ndk, ${packages[*]}"
