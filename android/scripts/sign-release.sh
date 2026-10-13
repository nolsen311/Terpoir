#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Signs a release on the maintainer's machine with the YubiKey
# (docs/releasing.md). Downloads the unsigned builds from the tag's draft
# GitHub release, checks them against the hashes CI published, signs the
# APKs with the app signing key and the Play bundle with the upload key
# (both through the YubiKey's PKCS#11 module, PIN and touch required),
# verifies the signatures and certificates, and writes SHA256SUMS.
# With --upload, it also puts the signed APKs on the draft release and
# removes the unsigned ones; publishing stays a separate, manual step.
#
# Usage: android/scripts/sign-release.sh v0.1.0 [--upload]
#
# Settings (environment):
#   TERPOIR_PKCS11_LIB   path to libykcs11 (found in the usual places)
#   TERPOIR_APP_SLOT     PIV slot of the app signing key (default 9a)
#   TERPOIR_UPLOAD_SLOT  PIV slot of the Play upload key; unset: the bundle
#                        is not signed
#   TERPOIR_SSH_KEY      an ed25519-sk key to sign SHA256SUMS with (optional)
#   TERPOIR_REPO         default nolsen311/Terpoir
#   TERPOIR_UNSIGNED_DIR use the unsigned builds and UNSIGNED-SHA256SUMS in
#                        this directory instead of downloading them

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

tag=${1:-}
upload=false
[ "${2:-}" = "--upload" ] && upload=true
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "usage: $0 vX.Y.Z [--upload]"
version=${tag#v}
repo=${TERPOIR_REPO:-nolsen311/Terpoir}
here=$(cd "$(dirname "$0")" && pwd)

for tool in apksigner zipalign jarsigner keytool sha256sum; do
    command -v "$tool" >/dev/null || die "$tool not found (apksigner and zipalign are in the Android build-tools; jarsigner and keytool in a JDK 17+)"
done

if [ -z "${TERPOIR_UNSIGNED_DIR:-}" ] || $upload; then
    command -v gh >/dev/null || die "gh not found"
fi

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
# the label as the alias.
alias_of() {
    case "$1" in
        9a) echo "X.509 Certificate for PIV Authentication" ;;
        9c) echo "X.509 Certificate for Digital Signature" ;;
        9d) echo "X.509 Certificate for Key Management" ;;
        9e) echo "X.509 Certificate for Card Authentication" ;;
        8[2-9]|9[0-5]) echo "X.509 Certificate for Retired Key $((16#$1 - 16#81))" ;;
        *) die "unknown PIV slot $1" ;;
    esac
}
app_alias=$(alias_of "${TERPOIR_APP_SLOT:-9a}")
upload_alias=""
[ -n "${TERPOIR_UPLOAD_SLOT:-}" ] && upload_alias=$(alias_of "$TERPOIR_UPLOAD_SLOT")

work="$PWD/terpoir-release-$version"
[ -e "$work" ] && die "$work exists: move it away first"
mkdir -p "$work/unsigned" "$work/signed"
chmod 700 "$work"
cfg="$work/pkcs11.cfg"
printf 'name = TerpoirYubiKey\nlibrary = %s\nslotListIndex = 0\n' "$lib" > "$cfg"

if [ -n "${TERPOIR_UNSIGNED_DIR:-}" ]; then
    cp "$TERPOIR_UNSIGNED_DIR"/*-unsigned.* "$TERPOIR_UNSIGNED_DIR/UNSIGNED-SHA256SUMS" "$work/unsigned/"
else
    echo "Downloading the unsigned builds of $tag from $repo"
    gh release download "$tag" --repo "$repo" --dir "$work/unsigned" \
        --pattern '*-unsigned.apk' --pattern '*-unsigned.aab' --pattern 'UNSIGNED-SHA256SUMS'
fi
(cd "$work/unsigned" && sha256sum -c UNSIGNED-SHA256SUMS) || die "the downloads don't match CI's hashes"

# One PIN entry for the whole run; the YubiKey still asks for a touch per
# signature if its touch policy says so.
read -rsp "YubiKey PIV PIN: " TERPOIR_PIN && echo
export TERPOIR_PIN
trap 'unset TERPOIR_PIN' EXIT

pkcs11=(--ks NONE --ks-type PKCS11 --provider-class sun.security.pkcs11.SunPKCS11 --provider-arg "$cfg" --ks-pass env:TERPOIR_PIN)

expected_cert=""
[ -s "$here/../release/app-certificate.sha256" ] && expected_cert=$(tr -d ' \n:' < "$here/../release/app-certificate.sha256" | tr 'A-F' 'a-f')

for flavor in foss play; do
    in="$work/unsigned/terpoir-$flavor-$version-unsigned.apk"
    out="$work/signed/terpoir-$flavor-$version.apk"
    zipalign -c -P 16 4 "$in" >/dev/null || die "$in is not aligned"
    echo "Signing $(basename "$out") (touch the YubiKey if it blinks)"
    # Java 17+ keeps the PKCS#11 provider's class to its module unless told.
    apksigner -J-add-exports=jdk.crypto.cryptoki/sun.security.pkcs11=ALL-UNNAMED \
        sign "${pkcs11[@]}" --ks-key-alias "$app_alias" --v4-signing-enabled false --in "$in" --out "$out"
    apksigner verify --verbose --print-certs "$out" > "$work/$flavor-verify.txt"
    # "Signer #1 certificate ..." or, for v3-only APKs, "V3.0 Signer: certificate ...".
    cert=$(sed -n 's/^.*[Ss]igner.* certificate SHA-256 digest: //p' "$work/$flavor-verify.txt" | head -n 1)
    [ -n "$cert" ] || die "no signer certificate in $out"
    if [ -n "$expected_cert" ] && [ "$cert" != "$expected_cert" ]; then
        die "$out is signed by $cert, not the app key in android/release/app-certificate.sha256"
    fi
    echo "  certificate SHA-256: $cert"
done

if [ -n "$upload_alias" ]; then
    in="$work/unsigned/terpoir-play-$version-unsigned.aab"
    out="$work/signed/terpoir-play-$version.aab"
    cp "$in" "$out"
    echo "Signing $(basename "$out") with the upload key (touch the YubiKey if it blinks)"
    jarsigner -keystore NONE -storetype PKCS11 -providerClass sun.security.pkcs11.SunPKCS11 \
        -providerArg "$cfg" -storepass:env TERPOIR_PIN "$out" "$upload_alias" >/dev/null
    # Not -strict: Android certificates are self-signed, which it rejects.
    jarsigner -verify "$out" | grep -q '^jar verified\.' || die "the bundle's signature doesn't verify"
    keytool -printcert -jarfile "$out" | sed -n 's/^\s*SHA256: /  upload certificate SHA-256: /p' | head -n 1
else
    echo "TERPOIR_UPLOAD_SLOT unset: the Play bundle is left unsigned"
fi

(cd "$work/signed" && sha256sum terpoir-*.apk > SHA256SUMS)
if [ -n "${TERPOIR_SSH_KEY:-}" ]; then
    ssh-keygen -Y sign -f "$TERPOIR_SSH_KEY" -n file "$work/signed/SHA256SUMS"
fi
cat "$work/signed/SHA256SUMS"

if $upload; then
    assets=("$work"/signed/*.apk "$work/signed/SHA256SUMS")
    [ -f "$work/signed/SHA256SUMS.sig" ] && assets+=("$work/signed/SHA256SUMS.sig")
    gh release upload "$tag" --repo "$repo" "${assets[@]}"
    for name in "terpoir-foss-$version-unsigned.apk" "terpoir-play-$version-unsigned.apk" \
        "terpoir-play-$version-unsigned.aab" UNSIGNED-SHA256SUMS; do
        gh release delete-asset "$tag" "$name" --repo "$repo" --yes
    done
    echo "Uploaded. Check the draft, then publish: gh release edit $tag --repo $repo --draft=false"
else
    echo "Signed builds are in $work/signed. Upload with --upload, or by hand."
fi
[ -f "$work/signed/terpoir-play-$version.aab" ] && echo "Upload the bundle to the Play Console: $work/signed/terpoir-play-$version.aab"
