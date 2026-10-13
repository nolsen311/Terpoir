#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Proves to Google that the maintainer owns the package name app.terpoir
# (Android developer verification: the Android Developer Console gives a
# snippet file to put in an APK's assets, signed with the app signing key).
# Builds a minimal APK for app.terpoir (no code, no permissions, nothing but
# the snippet in assets/) and signs it with the YubiKey app key exactly as
# sign-release.sh signs releases: PKCS#11 through libykcs11, APK Signature
# Scheme v3, then checks the certificate against
# android/release/app-certificate.sha256.
#
# Usage: android/scripts/sign-verification-apk.sh <snippet file> [asset name]
#   The snippet goes in assets/ under its own file name unless [asset name]
#   gives another. Writes terpoir-verification.apk in the current directory.
#
# Settings (environment), as for sign-release.sh:
#   TERPOIR_PKCS11_LIB   path to libykcs11 (found in the usual places)
#   TERPOIR_APP_SLOT     PIV slot of the app signing key (default 9a)
#   ANDROID_HOME         the Android SDK (build-tools, and the API 37
#                        platform's android.jar)

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

snippet=${1:-}
if [ -z "$snippet" ] || [ ! -f "$snippet" ]; then
    die "usage: $0 <snippet file> [asset name]"
fi
asset=${2:-$(basename "$snippet")}
[[ "$asset" =~ ^[A-Za-z0-9._-]+$ ]] || die "the asset name may use only letters, digits, '.', '_' and '-'"
here=$(cd "$(dirname "$0")" && pwd)
out="$PWD/terpoir-verification.apk"
[ -e "$out" ] && die "$out exists: move it away first"

sdk=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}
[ -n "$sdk" ] || die "set ANDROID_HOME to the Android SDK"
# The SDK names it android-37 or android-37.0, depending on the version.
platform=$(find "$sdk/platforms" -maxdepth 2 -path '*/android-37*/android.jar' 2>/dev/null | sort | head -n 1)
[ -n "$platform" ] || die "no android.jar for API 37 in $sdk/platforms: install it (sdkmanager \"platforms;android-37\")"
tools=$(find "$sdk/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)
[ -n "$tools" ] || die "no Android build-tools in $sdk"
for tool in aapt2 zipalign apksigner; do
    [ -x "$tools/$tool" ] || die "$tool not found in $tools"
done

lib=${TERPOIR_PKCS11_LIB:-}
if [ -z "$lib" ]; then
    for candidate in /usr/lib/x86_64-linux-gnu/libykcs11.so /usr/lib/aarch64-linux-gnu/libykcs11.so \
        /usr/lib64/libykcs11.so /usr/local/lib/libykcs11.so /usr/local/lib/libykcs11.dylib \
        /opt/homebrew/lib/libykcs11.dylib; do
        [ -f "$candidate" ] && lib=$candidate && break
    done
fi
if [ -z "$lib" ] || [ ! -f "$lib" ]; then
    die "libykcs11 not found: install yubico-piv-tool or set TERPOIR_PKCS11_LIB"
fi

# ykcs11 labels each PIV slot's certificate; Java's PKCS#11 keystore uses
# the label as the alias (as in sign-release.sh).
case "${TERPOIR_APP_SLOT:-9a}" in
    9a) alias="X.509 Certificate for PIV Authentication" ;;
    9c) alias="X.509 Certificate for Digital Signature" ;;
    9d) alias="X.509 Certificate for Key Management" ;;
    9e) alias="X.509 Certificate for Card Authentication" ;;
    8[2-9] | 9[0-5]) alias="X.509 Certificate for Retired Key $((16#$TERPOIR_APP_SLOT - 16#81))" ;;
    *) die "unknown PIV slot $TERPOIR_APP_SLOT" ;;
esac

work=$(mktemp -d)
trap 'rm -rf "$work"; unset TERPOIR_PIN' EXIT
mkdir -p "$work/assets"
cp "$snippet" "$work/assets/$asset"
cat > "$work/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<!-- Package-name ownership proof only: no code, no components, no permissions. -->
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="app.terpoir"
    android:versionCode="1"
    android:versionName="verification">
    <uses-sdk android:minSdkVersion="30" android:targetSdkVersion="37" />
    <application android:hasCode="false" android:label="Terpoir" />
</manifest>
EOF
"$tools/aapt2" link -o "$work/unaligned.apk" -I "$platform" --manifest "$work/AndroidManifest.xml" -A "$work/assets"
"$tools/zipalign" -P 16 -f 4 "$work/unaligned.apk" "$work/unsigned.apk"
unzip -l "$work/unsigned.apk" | grep -q " assets/$asset\$" || die "the snippet is missing from the APK"

printf 'name = TerpoirYubiKey\nlibrary = %s\nslotListIndex = 0\n' "$lib" > "$work/pkcs11.cfg"
read -rsp "YubiKey PIV PIN: " TERPOIR_PIN && echo
export TERPOIR_PIN
echo "Signing $(basename "$out") (touch the YubiKey if it blinks)"
# Java 17+ keeps the PKCS#11 provider's class to its module unless told.
"$tools/apksigner" -J-add-exports=jdk.crypto.cryptoki/sun.security.pkcs11=ALL-UNNAMED \
    sign --ks NONE --ks-type PKCS11 --provider-class sun.security.pkcs11.SunPKCS11 \
    --provider-arg "$work/pkcs11.cfg" --ks-pass env:TERPOIR_PIN --ks-key-alias "$alias" \
    --v4-signing-enabled false --in "$work/unsigned.apk" --out "$out"

"$tools/apksigner" verify --verbose --print-certs "$out" > "$work/verify.txt"
cert=$(sed -n 's/^.*[Ss]igner.* certificate SHA-256 digest: //p' "$work/verify.txt" | head -n 1)
[ -n "$cert" ] || die "no signer certificate in $out"
pinned="$here/../release/app-certificate.sha256"
if [ -s "$pinned" ]; then
    expected=$(tr -d ' \n:' < "$pinned" | tr 'A-F' 'a-f')
    if [ "$cert" != "$expected" ]; then
        rm -f "$out"
        die "signed by $cert, not the app key in android/release/app-certificate.sha256"
    fi
fi
echo "Wrote $out"
echo "  package app.terpoir, assets/$asset, certificate SHA-256 $cert"
echo "Upload it in the Android Developer Console."
