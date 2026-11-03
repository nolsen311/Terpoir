#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Drives a whole release from the maintainer's machine (docs/releasing.md):
# signs and pushes the tag, waits for the Release workflow's draft, runs
# sign-release.sh (YubiKey PIN once, a touch per signature), asks before
# publishing on GitHub, then before opening a pull request that publishes
# the signed hashes on terpoir.app (publish-hashes.sh), then before
# sending the bundle to Google Play (play-upload.sh). Nothing is signed or stored in CI; this only chains
# the manual steps, and each outward-facing one asks first.
#
# Usage: android/scripts/release.sh vX.Y.Z
#   On a desktop whose SSH agent can't use the YubiKey's ed25519-sk key
#   (the maintainer's), blank it so tag and SHA256SUMS signing reach the
#   key directly: SSH_AUTH_SOCK= android/scripts/release.sh vX.Y.Z
#
# Safe to re-run after an interruption: a tag that is already on the remote
# is not recreated, and a finished CI run is not waited for again.
#
# Settings (environment): the ones sign-release.sh and play-upload.sh take,
# plus
#   TERPOIR_REPO      default nolsen311/Terpoir
#   TERPOIR_SSH_KEY   default: git's user.signingkey (the tag's key)
#   TERPOIR_WORK_DIR  where signed builds are written
#                     (default ~/terpoir-releases, outside the repository)

set -euo pipefail
# A failing command ends the script (set -e): say where, never stop silently.
trap 'echo "error: release.sh stopped at line $LINENO (exit $?)" >&2' ERR

die() { echo "error: $*" >&2; exit 1; }
confirm() { read -rp "$1 [y/N] " answer; [[ "$answer" =~ ^[Yy]$ ]]; }

tag=${1:-}
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "usage: $0 vX.Y.Z"
version=${tag#v}
repo=${TERPOIR_REPO:-nolsen311/Terpoir}
here=$(cd "$(dirname "$0")" && pwd)
root=$(git -C "$here" rev-parse --show-toplevel)

echo "Releasing $tag from $repo"
for tool in git gh; do
    command -v "$tool" >/dev/null || die "$tool not found"
done
echo "Checking the GitHub CLI login"
gh auth status >/dev/null 2>&1 || die "gh is not logged in (gh auth login)"

# Only main and this release's tag: never every tag, since CI moves the
# dev-build tag and fetching it over an older local copy fails. With an SSH
# remote on a YubiKey key, a fetch waits for a touch.
echo "Fetching origin/main (touch the YubiKey if it blinks)"
git -C "$root" fetch --quiet origin main || die "git fetch origin main failed"
main_sha=$(git -C "$root" rev-parse origin/main)

# Before anything is tagged: what sign-release.sh needs to sign every file.
# SHA256SUMS is signed with the key that signs the tag (git's
# user.signingkey, which allowed_signers trusts for files too) unless
# TERPOIR_SSH_KEY names another; publish-hashes.sh needs that signature.
if [ -z "${TERPOIR_SSH_KEY:-}" ]; then
    key=$(git -C "$root" config --get user.signingkey || true)
    key=${key/#\~/$HOME}
    [[ "$key" == *.pub && -f "${key%.pub}" ]] && key=${key%.pub}
    [ -f "$key" ] && export TERPOIR_SSH_KEY=$key
fi
if [ -n "${TERPOIR_SSH_KEY:-}" ]; then
    echo "SHA256SUMS will be signed with $TERPOIR_SSH_KEY"
else
    confirm "No SSH key for SHA256SUMS (TERPOIR_SSH_KEY, or git's user.signingkey as a file): it won't be signed, and publish-hashes.sh can't publish it. Continue anyway?" \
        || die "stopped; set TERPOIR_SSH_KEY and run again"
fi
# The Play bundle is signed only when the upload key's slot is named.
if [ -n "${TERPOIR_UPLOAD_SLOT:-}" ]; then
    echo "The Play bundle will be signed with the upload key in slot $TERPOIR_UPLOAD_SLOT"
else
    confirm "TERPOIR_UPLOAD_SLOT isn't set: the Play bundle won't be signed and can't go to Google Play. Continue anyway?" \
        || die "stopped; set TERPOIR_UPLOAD_SLOT (docs/releasing.md) and run again"
fi

# 1. The tag.
if git -C "$root" ls-remote --exit-code --tags origin "refs/tags/$tag" >/dev/null 2>&1; then
    echo "Tag $tag is already on origin; not creating it again."
    git -C "$root" fetch --quiet origin "refs/tags/$tag:refs/tags/$tag" \
        || die "can't fetch $tag: a local $tag that differs from origin's? Compare, then git tag -d $tag"
    tag_sha=$(git -C "$root" rev-parse "$tag^{commit}")
else
    current=$(git -C "$root" show origin/main:android/app/build.gradle.kts \
        | sed -n 's/^ *versionName = "\(.*\)"$/\1/p')
    [ "$current" = "$version" ] \
        || die "origin/main has versionName $current, not $version: merge the version bump first"
    [ "$(git -C "$root" config --get gpg.format)" = "ssh" ] && [ -n "$(git -C "$root" config --get user.signingkey)" ] \
        || die "git is not set up to sign tags with the YubiKey's SSH key (gpg.format ssh, user.signingkey; docs/releasing.md)"
    echo "About to sign tag $tag on origin/main ($(git -C "$root" log -1 --format='%h %s' "$main_sha"))."
    confirm "Create the tag and push it? That starts the Release workflow." || die "stopped; nothing was tagged"
    echo "Signing the tag (touch the YubiKey if it blinks)"
    git -C "$root" tag -s "$tag" -m "Terpoir $version" "$main_sha"
    git -C "$root" push origin "refs/tags/$tag"
    tag_sha=$main_sha
fi

# 2. The Release workflow.
echo "Waiting for the Release workflow on $tag"
run_id=""
for _ in $(seq 1 30); do
    run_id=$(gh run list --repo "$repo" --workflow release.yml --event push --branch "$tag" \
        --json databaseId,headSha --jq "[.[] | select(.headSha == \"$tag_sha\")][0].databaseId // empty")
    [ -n "$run_id" ] && break
    sleep 10
done
[ -n "$run_id" ] || die "no Release run for $tag after 5 minutes: check the Actions tab"
gh run watch "$run_id" --repo "$repo" --exit-status --interval 30 >/dev/null \
    || die "the Release workflow failed: gh run view $run_id --repo $repo --log-failed"
printf '\a'
echo "CI is done."

is_draft=$(gh release view "$tag" --repo "$repo" --json isDraft --jq .isDraft)
[ "$is_draft" = "true" ] || die "release $tag is not a draft (already published?): not signing it again"

# 3. Sign and upload.
workdir=${TERPOIR_WORK_DIR:-$HOME/terpoir-releases}
mkdir -p "$workdir"
cd "$workdir"
echo "Signing (YubiKey PIN, then a touch per signature)"
"$here/sign-release.sh" "$tag" --upload

# 4. Publish.
echo
gh release view "$tag" --repo "$repo" --json assets --jq '.assets[].name'
echo
published=false
if confirm "Publish $tag on GitHub now? Check the draft first: https://github.com/$repo/releases"; then
    gh release edit "$tag" --repo "$repo" --draft=false
    published=true
    echo "Published."
else
    echo "Left as a draft. Publish later: gh release edit $tag --repo $repo --draft=false"
fi

# The signed hashes on terpoir.app: a pull request, live once merged.
if $published && confirm "Open a pull request publishing $tag's signed hashes on terpoir.app?"; then
    "$here/publish-hashes.sh" --pr "$tag"
elif $published; then
    echo "Not opened. Later: android/scripts/publish-hashes.sh --pr $tag"
fi

# 5. Google Play.
bundle="$workdir/terpoir-release-$version/signed/terpoir-play-$version.aab"
if [ ! -f "$bundle" ]; then
    echo "No signed Play bundle (TERPOIR_UPLOAD_SLOT unset): nothing to send to Google Play."
elif [ -z "${TERPOIR_PLAY_SERVICE_ACCOUNT:-}" ]; then
    echo "The signed Play bundle is at $bundle"
    echo "Set TERPOIR_PLAY_SERVICE_ACCOUNT to send it automatically, or upload it in the Play Console."
elif confirm "Send $tag to Google Play's ${TERPOIR_PLAY_TRACK:-internal} track (${TERPOIR_PLAY_STATUS:-draft})?"; then
    TERPOIR_WORK_DIR=$workdir "$here/play-upload.sh" "$tag"
else
    echo "Not sent. Later: TERPOIR_WORK_DIR=$workdir android/scripts/play-upload.sh $tag"
fi
