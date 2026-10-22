#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Publishes releases' signed hashes on terpoir.app (docs/releasing.md):
# for each tag, downloads the published GitHub release's SHA256SUMS and
# SHA256SUMS.sig (and SBOM-SHA256SUMS when it has one), refuses them
# unless the signature verifies against android/release/allowed_signers,
# stores them in site/releases/<tag>/ with the publication date, and
# regenerates site/releases/index.html (site-releases.py). The site
# deploys from main, so they go live when that change merges.
#
# Usage: android/scripts/publish-hashes.sh [--pr] vX.Y.Z...
#   --pr   do it in a temporary worktree of origin/main instead of this
#          checkout (which is left as it is), commit it on a new branch
#          with your sign-off (git commit -s), push it and open a pull
#          request
#
# Run from anywhere in a checkout. Needs gh (signed in), ssh-keygen,
# python3 and sha256sum. release.sh runs it with --pr after publishing.

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

pr=false
if [ "${1:-}" = --pr ]; then
    pr=true
    shift
fi
[ $# -ge 1 ] || die "usage: $0 [--pr] vX.Y.Z..."

repo=${TERPOIR_REPO:-nolsen311/Terpoir}
root=$(git -C "$(dirname "$0")" rev-parse --show-toplevel)
cd "$root"

if $pr; then
    git fetch -q origin main
    branch="site-hashes-${*// /-}"
    tree=$(mktemp -d)
    git worktree add -q -b "$branch" "$tree" origin/main
    trap 'git -C "$root" worktree remove --force "$tree"' EXIT
    cd "$tree"
fi
signers=android/release/allowed_signers
principal=$(awk 'NF && $1 !~ /^#/ { print $1; exit }' "$signers")

asset() { # tag json name out: download one named asset of a release
    local id
    id=$(printf '%s' "$2" | python3 -c "import json,sys; print(next((a['id'] for a in json.load(sys.stdin)['assets'] if a['name']==sys.argv[1]), ''))" "$3")
    [ -n "$id" ] || return 1
    gh api -H "Accept: application/octet-stream" "repos/$repo/releases/assets/$id" > "$4"
}

for tag in "$@"; do
    [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "$tag is not a release tag (vX.Y.Z)"
    json=$(gh api "repos/$repo/releases/tags/$tag")
    draft=$(printf '%s' "$json" | python3 -c "import json,sys; print(json.load(sys.stdin)['draft'])")
    [ "$draft" = False ] || die "$tag is still a draft: publish it first"
    out="site/releases/$tag"
    work=$(mktemp -d)
    asset "$tag" "$json" SHA256SUMS "$work/SHA256SUMS" || die "$tag has no SHA256SUMS"
    asset "$tag" "$json" SHA256SUMS.sig "$work/SHA256SUMS.sig" || die "$tag has no SHA256SUMS.sig"
    ssh-keygen -Y verify -f "$signers" -I "$principal" -n file -s "$work/SHA256SUMS.sig" \
        < "$work/SHA256SUMS" >/dev/null || die "$tag: SHA256SUMS.sig doesn't verify against $signers"
    # From 0.9.0: the SBOMs' hashes as CI wrote them (not signed).
    asset "$tag" "$json" SBOM-SHA256SUMS "$work/SBOM-SHA256SUMS" || true
    mkdir -p "$out"
    cp "$work"/* "$out/"
    printf '%s' "$json" | python3 -c "import json,sys; print(json.load(sys.stdin)['published_at'][:10])" > "$out/PUBLISHED"
    rm -rf "$work"
    echo "$tag: signature verified; $(wc -l < "$out/SHA256SUMS") hash(es) in $out"
done

python3 android/scripts/site-releases.py

if $pr; then
    git add site/releases
    git commit -q -s -m "Release hashes on terpoir.app: $*

The signed SHA256SUMS of $* (signature checked against
android/release/allowed_signers), published by publish-hashes.sh."
    git push -q -u origin "$branch"
    gh pr create --repo "$repo" --base main --head "$branch" \
        --title "Release hashes on terpoir.app: $*" \
        --body "The signed \`SHA256SUMS\` of $*, verified against \`android/release/allowed_signers\` and copied to \`site/releases/\`, with \`terpoir.app/releases/\` regenerated. Made by \`android/scripts/publish-hashes.sh\`."
fi
