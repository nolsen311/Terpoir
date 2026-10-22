#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: check-ocr-libs.sh ARCHIVE...
#
# Checks that the Tesseract libraries in each APK or app bundle are the
# from-source build (android/ocr-native/): none links liblog or calls
# Android's logger, which the JitPack build does (docs/design-decisions.md,
# "Logs and crash reports"). Each archive must hold all four libraries
# for both ABIs. Run from the repository root (ANDROID_HOME set).

set -euo pipefail

ndk=$(sed -n 's/^ *ndkVersion = "\(.*\)"$/\1/p' android/app/build.gradle.kts)
bin="$ANDROID_HOME/ndk/$ndk/toolchains/llvm/prebuilt/linux-x86_64/bin"
for archive in "$@"; do
    libs=$(mktemp -d)
    unzip -q -o "$archive" '*lib/*/libtesseract.so' '*lib/*/libleptonica.so' \
        '*lib/*/libjpeg.so' '*lib/*/libpngx.so' -d "$libs"
    found=0
    while IFS= read -r lib; do
        found=$((found + 1))
        if "$bin/llvm-nm" -D --undefined-only "$lib" | grep -q '__android_log'; then
            echo "::error::$archive: ${lib#"$libs"/} calls Android's logger: not the from-source build."
            exit 1
        fi
        if "$bin/llvm-readelf" -d "$lib" | grep -q 'liblog.so'; then
            echo "::error::$archive: ${lib#"$libs"/} links liblog: not the from-source build."
            exit 1
        fi
    done < <(find "$libs" -name '*.so')
    [ "$found" -eq 8 ] || { echo "::error::$archive: expected 8 OCR libraries, found $found"; exit 1; }
    rm -rf "$libs"
    echo "$archive ships the from-source OCR libraries, none of which logs."
done
