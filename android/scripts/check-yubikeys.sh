#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Read-only check of the release YubiKeys (docs/releasing.md, "Checking
# the YubiKeys"). Asks for no PIN, uses no PIN attempt, and changes
# nothing on the keys.
#
# For each connected YubiKey it prints:
#   - the PIV PIN and PUK tries left, and the FIDO2 PIN tries left;
#   - for each signing slot: the key's algorithm, PIN and touch policy,
#     whether it was generated on the key or imported, and its
#     certificate's subject and SHA-256 and SHA-1 fingerprints;
#   - whether the app signing certificate matches the pinned one
#     (android/release/app-certificate.sha256).
#
# It also saves each key's slot details (not its serial or tries) under
# TERPOIR_YUBIKEY_DIR, and compares every saved key with the others. So
# the keys can be checked one at a time, plugged in one after another.
#
# Usage: android/scripts/check-yubikeys.sh
# Exits 0 when every saved key matches, 1 when any differs.
#
# Settings (environment):
#   TERPOIR_APP_SLOT     PIV slot of the app signing key (default 9a)
#   TERPOIR_UPLOAD_SLOT  PIV slot of the Play upload key (default 9d)
#   TERPOIR_YUBIKEY_DIR  where the saved details go
#                        (default ~/terpoir-releases/yubikey-check)

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

here=$(cd "$(dirname "$0")" && pwd)
pinned_file="$here/../release/app-certificate.sha256"
app_slot=${TERPOIR_APP_SLOT:-9a}
upload_slot=${TERPOIR_UPLOAD_SLOT:-9d}
dir=${TERPOIR_YUBIKEY_DIR:-$HOME/terpoir-releases/yubikey-check}

for tool in ykman openssl diff; do
    command -v "$tool" >/dev/null || die "$tool not found (ykman comes with yubikey-manager)"
done
pinned=$(tr -d ' \n:' <"$pinned_file" | tr '[:upper:]' '[:lower:]')

mapfile -t serials < <(ykman list --serials)
[ "${#serials[@]}" -gt 0 ] || die "no YubiKey found: plug one in"
mkdir -p "$dir"

# Slots worth reporting: the two configured ones, and the usual signing
# slots, so a key in an unexpected slot still shows up.
slots=("$app_slot" "$upload_slot")
for s in 9a 9c 9d 82; do
    [[ " ${slots[*]} " == *" $s "* ]] || slots+=("$s")
done

# The line of `text` that starts with `label`, without the label.
field() { # text label
    sed -n "s/^[[:space:]]*$2[[:space:]]*//p" <<<"$1" | head -n 1
}

slot_details() { # serial slot
    local serial=$1 slot=$2 info pem
    echo "slot $slot"
    if info=$(ykman --device "$serial" piv keys info "$slot" 2>/dev/null); then
        # `piv keys info` needs firmware 5.3 or newer; older keys skip it.
        sed 's/^/  /' <<<"$info"
    fi
    if pem=$(ykman --device "$serial" piv certificates export "$slot" - 2>/dev/null) && [ -n "$pem" ]; then
        echo "  certificate: $(openssl x509 -noout -subject <<<"$pem" | sed 's/^subject= *//')"
        echo "  SHA-256: $(openssl x509 -noout -fingerprint -sha256 <<<"$pem" | cut -d= -f2)"
        echo "  SHA-1: $(openssl x509 -noout -fingerprint -sha1 <<<"$pem" | cut -d= -f2)"
    else
        echo "  no certificate"
    fi
}

for serial in "${serials[@]}"; do
    echo "== YubiKey $serial"
    device=$(ykman --device "$serial" info)
    piv=$(ykman --device "$serial" piv info)
    fido=$(ykman --device "$serial" fido info 2>/dev/null || true)
    echo "firmware: $(field "$device" 'Firmware version:')"
    pin_tries=$(field "$piv" 'PIN tries remaining:')
    puk_tries=$(field "$piv" 'PUK tries remaining:')
    echo "PIV PIN tries left: $pin_tries"
    echo "PIV PUK tries left: $puk_tries"
    fido_pin=$(grep -i 'PIN' <<<"$fido" | head -n 1 | sed 's/^[[:space:]]*//; s/^PIN:[[:space:]]*//')
    echo "FIDO2 PIN: ${fido_pin:-not reported}"
    # "2/3": a wrong PIN was entered since the last correct one. The count
    # resets after one correct entry; at 0 the PIN is blocked.
    if [[ "$pin_tries" =~ ^([0-9]+)/([0-9]+)$ ]] && [ "${BASH_REMATCH[1]}" != "${BASH_REMATCH[2]}" ]; then
        echo "!! PIV PIN entered wrongly $((BASH_REMATCH[2] - BASH_REMATCH[1])) time(s) since the last correct entry; ${BASH_REMATCH[1]} tries left before it locks"
    fi
    if [[ "$puk_tries" =~ ^([0-9]+)/([0-9]+)$ ]] && [ "${BASH_REMATCH[1]}" != "${BASH_REMATCH[2]}" ]; then
        echo "!! the PUK has been entered wrongly; ${BASH_REMATCH[1]} tries left before it locks"
    fi

    details=$(for slot in "${slots[@]}"; do slot_details "$serial" "$slot"; done)
    echo "$details"

    app_fp=$(sed -n "/^slot $app_slot\$/,/^slot /s/^  SHA-256: //p" <<<"$details" | head -n 1 | tr -d ':' | tr '[:upper:]' '[:lower:]')
    if [ -z "$app_fp" ]; then
        echo "!! no certificate in the app signing slot $app_slot"
    elif [ "$app_fp" = "$pinned" ]; then
        echo "ok: slot $app_slot holds the pinned app signing certificate"
    else
        echo "!! slot $app_slot's certificate does not match android/release/app-certificate.sha256"
    fi
    if ! sed -n "/^slot $upload_slot\$/,/^slot /p" <<<"$details" | grep -q 'SHA-256:'; then
        echo "!! no certificate in the upload key slot $upload_slot: this key can't sign Play bundles"
    fi
    printf '%s\n' "$details" >"$dir/$serial.txt"
    echo
done

# Compare every saved key with the first, slot by slot.
mapfile -t saved < <(find "$dir" -maxdepth 1 -name '*.txt' | sort)
if [ "${#saved[@]}" -lt 2 ]; then
    echo "Saved details for ${#saved[@]} key in $dir. Plug in the other key and run this again to compare them."
    exit 0
fi
first=${saved[0]}
same=true
for other in "${saved[@]:1}"; do
    if diff -q "$first" "$other" >/dev/null; then
        echo "same: $(basename "$first" .txt) and $(basename "$other" .txt)"
    else
        same=false
        echo "DIFFERENT: $(basename "$first" .txt) and $(basename "$other" .txt)"
        diff -u --label "$(basename "$first" .txt)" --label "$(basename "$other" .txt)" "$first" "$other" | tail -n +3 || true
    fi
done
if $same; then
    echo "All saved keys hold the same keys and certificates, with the same policies."
else
    exit 1
fi
