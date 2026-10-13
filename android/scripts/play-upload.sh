#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Sends a signed release bundle to Google Play, from the maintainer's
# machine (docs/releasing.md, "Google Play"). Uploads the bundle that
# sign-release.sh signed with the upload key, and its R8 mapping, then puts
# it on a track through the Google Play Developer API (one edit, committed
# at the end, so nothing changes on Play unless every step succeeds).
#
# No key file: the maintainer's own gcloud login asks Google's IAM
# Credentials API for a 10-minute token of the Play publisher service
# account (Terpoir-infra), scoped to the Play Developer API only.
#
# Usage: android/scripts/play-upload.sh vX.Y.Z
#
# Settings (environment):
#   TERPOIR_PLAY_SERVICE_ACCOUNT  the publisher service account's email
#                                 (required unless TERPOIR_PLAY_TOKEN is set)
#   TERPOIR_PLAY_TRACK   internal (default), alpha, beta or production
#   TERPOIR_PLAY_STATUS  draft (default: the release waits in the Play
#                        Console for "Start rollout") or completed
#   TERPOIR_WORK_DIR     where sign-release.sh wrote (default ~/terpoir-releases)
#   TERPOIR_REPO         default nolsen311/Terpoir (for the mapping file)
#   TERPOIR_PLAY_TOKEN   an access token to use instead (tests)
#   TERPOIR_PLAY_API     default https://androidpublisher.googleapis.com (tests)

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

tag=${1:-}
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "usage: $0 vX.Y.Z"
version=${tag#v}
package=app.terpoir
track=${TERPOIR_PLAY_TRACK:-internal}
status=${TERPOIR_PLAY_STATUS:-draft}
api=${TERPOIR_PLAY_API:-https://androidpublisher.googleapis.com}
repo=${TERPOIR_REPO:-nolsen311/Terpoir}
signed="${TERPOIR_WORK_DIR:-$HOME/terpoir-releases}/terpoir-release-$version/signed"
bundle="$signed/terpoir-play-$version.aab"

case "$track" in internal | alpha | beta | production) ;; *) die "unknown track $track" ;; esac
case "$status" in draft | completed) ;; *) die "TERPOIR_PLAY_STATUS must be draft or completed" ;; esac
for tool in curl jq; do
    command -v "$tool" >/dev/null || die "$tool not found"
done
[ -f "$bundle" ] || die "no signed bundle at $bundle: run sign-release.sh with TERPOIR_UPLOAD_SLOT first"
jarsigner -verify "$bundle" 2>/dev/null | grep -q '^jar verified\.' || die "$bundle isn't signed"

# The R8 mapping, so Play's crash reports show real names. Kept on the
# GitHub release; optional.
mapping="$signed/terpoir-play-$version-mapping.txt"
if [ ! -f "$mapping" ] && command -v gh >/dev/null; then
    gh release download "$tag" --repo "$repo" --dir "$signed" --pattern "terpoir-play-$version-mapping.txt" 2>/dev/null || true
fi

token=${TERPOIR_PLAY_TOKEN:-}
if [ -z "$token" ]; then
    account=${TERPOIR_PLAY_SERVICE_ACCOUNT:-}
    [ -n "$account" ] || die "set TERPOIR_PLAY_SERVICE_ACCOUNT (the Play publisher service account's email)"
    command -v gcloud >/dev/null || die "gcloud not found"
    # gcloud can't scope an impersonated token, and the Play API needs its
    # own scope, so ask the IAM Credentials API directly.
    token=$(curl -sSf -X POST \
        -H "Authorization: Bearer $(gcloud auth print-access-token)" \
        -H "Content-Type: application/json" \
        -d '{"scope":["https://www.googleapis.com/auth/androidpublisher"],"lifetime":"600s"}' \
        "https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/$account:generateAccessToken" \
        | jq -er .accessToken) || die "couldn't get a token for $account (gcloud auth login; Token Creator role on it)"
fi

call() { # method path [curl args...]
    local method=$1 path=$2
    shift 2
    local out
    out=$(curl -sS -X "$method" -H "Authorization: Bearer $token" -w '\n%{http_code}' "$@" "$api$path") || die "$method $path failed"
    local code=${out##*$'\n'}
    local body=${out%$'\n'*}
    [[ "$code" == 2* ]] || die "$method $path: HTTP $code: $(jq -r '.error.message // .' <<<"$body" 2>/dev/null || echo "$body")"
    printf '%s' "$body"
}

base="/androidpublisher/v3/applications/$package/edits"
echo "Opening an edit for $package"
edit=$(call POST "$base" -H "Content-Type: application/json" -d '{}' | jq -er .id)

echo "Uploading $(basename "$bundle")"
code=$(call POST "/upload$base/$edit/bundles?uploadType=media" \
    -H "Content-Type: application/octet-stream" --data-binary "@$bundle" | jq -er .versionCode)
echo "  versionCode $code"

if [ -f "$mapping" ]; then
    echo "Uploading the R8 mapping"
    call POST "/upload$base/$edit/apks/$code/deobfuscationFiles/proguard?uploadType=media" \
        -H "Content-Type: application/octet-stream" --data-binary "@$mapping" >/dev/null
else
    echo "No R8 mapping found; Play's crash reports will show obfuscated names"
fi

echo "Putting it on the $track track ($status)"
release=$(jq -n --arg code "$code" --arg name "$version" --arg status "$status" --arg track "$track" \
    '{track: $track, releases: [{name: $name, versionCodes: [$code], status: $status}]}')
call PUT "$base/$edit/tracks/$track" -H "Content-Type: application/json" -d "$release" >/dev/null

call POST "$base/$edit:commit" >/dev/null
echo "Done: Terpoir $version (versionCode $code) is on Google Play's $track track as a $status release."
[ "$status" = draft ] && echo "Roll it out in the Play Console: Test and release > $track."
exit 0
