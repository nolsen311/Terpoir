#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-notes.sh VERSION
#
# Prints release VERSION's notes (for example 0.5.3): the Play "What's new"
# text in docs/play-listing.md, the line after "<en-US>" in the block that
# follows the version's "**X.Y.Z**" heading. The GitHub release page
# (release.yml) opens with them too. Public posts use the shorter
# Summary: line instead (release-summary.sh). Fails if there are none.

set -eu

version=$1
listing=docs/play-listing.md

notes=$(awk -v v="**${version}**" '
    index($0, v) { found = 1; next }
    found && /<en-US>/ { getline; sub(/^[ \t]+/, ""); print; exit }
' "$listing")
[ -n "$notes" ] || { echo "no release notes for ${version} in ${listing}" >&2; exit 1; }
printf '%s\n' "$notes"
