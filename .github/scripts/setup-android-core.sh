#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Installs what the Android build needs to compile the Rust core:
# the pinned Rust toolchain with the Android targets, cargo-ndk (pinned,
# built from crates.io with its lockfile), and the NDK version named in
# android/app/build.gradle.kts. Run from the repository root on a GitHub
# Ubuntu runner (ANDROID_HOME set).

set -eu

CARGO_NDK_VERSION=4.1.2

(cd core && rustup toolchain install && rustup target add aarch64-linux-android x86_64-linux-android)
# Restored from the dependency cache on most runs (see android.yml).
if ! cargo ndk --version 2>/dev/null | grep -qx "cargo-ndk ${CARGO_NDK_VERSION}"; then
    cargo install --locked "cargo-ndk@${CARGO_NDK_VERSION}"
fi

ndk=$(sed -n 's/^ *ndkVersion = "\(.*\)"$/\1/p' android/app/build.gradle.kts)
[ -n "$ndk" ] || { echo "ndkVersion not found" >&2; exit 1; }
{ yes || true; } | "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" --licenses >/dev/null
"$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" "ndk;${ndk}" >/dev/null
echo "Rust Android targets, cargo-ndk ${CARGO_NDK_VERSION} and NDK ${ndk} ready"
# The NDK version keys the Rust build cache (android.yml).
if [ -n "${GITHUB_OUTPUT:-}" ]; then
    echo "ndk=${ndk}" >> "$GITHUB_OUTPUT"
fi
