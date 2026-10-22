#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: ocr-from-source.sh OUTPUT_DIR
#
# Builds the OCR native libraries from source (android/ocr-native/build.sh)
# for a release-style build, and checks them against the hashes committed
# in android/ocr-native/SHA256SUMS, which the OCR workflow proves
# reproducible. Prints the AAR's path, for -Pterpoir.ocr.aar. Run from the
# repository root on a GitHub Ubuntu runner (ANDROID_HOME and
# JAVA_HOME_21_X64 set).

set -euo pipefail

out=$1
sdk="$ANDROID_HOME/cmdline-tools/latest/bin"
{ yes || true; } | "$sdk/sdkmanager" --licenses >/dev/null
read -ra packages <<< "$(android/ocr-native/build.sh --sdk-packages)"
"$sdk/sdkmanager" "${packages[@]}" >/dev/null

JAVA_HOME="$JAVA_HOME_21_X64" android/ocr-native/build.sh "$out" >&2
if ! grep -v '^#' android/ocr-native/SHA256SUMS | diff -u - "$out/SHA256SUMS" >&2; then
    echo "::error::The OCR libraries differ from android/ocr-native/SHA256SUMS." >&2
    exit 1
fi
aar=$(ls "$out"/tesseract4android-*.aar)
[ "$(printf '%s\n' "$aar" | wc -l)" -eq 1 ] || { echo "::error::expected one AAR in $out" >&2; exit 1; }
echo "$aar"
