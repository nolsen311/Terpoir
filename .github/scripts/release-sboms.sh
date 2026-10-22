#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-sboms.sh OUTPUT_DIR VERSION OCR_AAR
#
# Writes a release's CycloneDX SBOMs to OUTPUT_DIR, with SBOM-SHA256SUMS:
#   terpoir-foss-VERSION.cdx.json, terpoir-play-VERSION.cdx.json
#       the app's release runtime dependencies (Gradle, cyclonedxBom);
#       Play's includes the on-demand OCR module
#   terpoir-core-VERSION-<target>.cdx.json
#       the Rust core's crates for each Android target (cargo-cyclonedx),
#       build-time crates whose code is compiled in (OpenSSL's) included
#   terpoir-ocr-native-VERSION.cdx.json
#       the OCR libraries' pinned sources (ocr-sbom.py)
# Run from the repository root after the release build, with the same
# OCR AAR, on a runner set up by setup-android-core.sh.

set -euo pipefail

CARGO_CYCLONEDX_VERSION=0.5.9

out=$1
version=$2
ocr_aar=$3
mkdir -p "$out"

for flavor in foss play; do
    (cd android && JAVA_HOME="${JAVA_HOME_21_X64:-$JAVA_HOME}" ./gradlew --no-daemon -q \
        -Pterpoir.reproducible=true -Pterpoir.ocr.aar="$ocr_aar" \
        -Pterpoir.sbom="$flavor" -Pterpoir.sbom.version="$version" cyclonedxBom)
    cp "android/build/reports/cyclonedx/terpoir-$flavor.cdx.json" "$out/terpoir-$flavor-$version.cdx.json"
done

if ! cargo cyclonedx --version 2>/dev/null | grep -q " ${CARGO_CYCLONEDX_VERSION}$"; then
    cargo install --locked "cargo-cyclonedx@${CARGO_CYCLONEDX_VERSION}"
fi
# The app's default ABIs (abiFilters), which these targets must match.
abis=$(grep -m1 '?: listOf(' android/app/build.gradle.kts | tr -d ' ')
[ "$abis" = '?:listOf("arm64-v8a","x86_64")' ] || { echo "::error::the app's ABIs changed; update release-sboms.sh" >&2; exit 1; }
for target in aarch64-linux-android x86_64-linux-android; do
    # It writes one file per workspace member; only terpoir-ffi's is the shipped library.
    (cd core && cargo cyclonedx -q --manifest-path crates/terpoir-ffi/Cargo.toml --format json \
        --spec-version 1.5 --target "$target" --override-filename "sbom-$target")
    cp "core/crates/terpoir-ffi/sbom-$target.json" "$out/terpoir-core-$version-$target.cdx.json"
    find core -name "sbom-$target.json" -delete
done

python3 .github/scripts/ocr-sbom.py "$ocr_aar" > "$out/terpoir-ocr-native-$version.cdx.json"

(cd "$out" && sha256sum ./*.cdx.json | sed 's| \./| |' > SBOM-SHA256SUMS && cat SBOM-SHA256SUMS)
