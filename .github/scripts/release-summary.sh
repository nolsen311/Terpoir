#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-summary.sh VERSION
#
# Prints release VERSION's summary for public posts (the Mastodon post),
# one point per line: the "Summary:" line under its "**X.Y.Z**" heading in
# docs/play-listing.md, whose points are separated by " ; ". Short on
# purpose: the full notes are on the release page. Fails if there is none.

set -eu

version=$1
listing=docs/play-listing.md

summary=$(awk -v v="**${version}**" '
    index($0, v) { found = 1; next }
    found && /^[ \t]*Summary:/ { sub(/^[ \t]*Summary:[ \t]*/, ""); print; exit }
    found && /<en-US>/ { exit }
' "$listing")
[ -n "$summary" ] || { echo "no Summary: line for ${version} in ${listing}" >&2; exit 1; }
printf '%s\n' "$summary" | awk 'BEGIN { RS = " ; " } { sub(/\n$/, ""); if ($0 != "") print }'
