#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-headline.sh VERSION
#
# Prints release VERSION's headline: the "Headline:" line under its
# "**X.Y.Z**" heading in docs/play-listing.md, shown on the release image
# (announce.yml). Fails if there is none.

set -eu

version=$1
listing=docs/play-listing.md

headline=$(awk -v v="**${version}**" '
    index($0, v) { found = 1; next }
    found && /^[ \t]*Headline:/ { sub(/^[ \t]*Headline:[ \t]*/, ""); print; exit }
    found && /<en-US>/ { exit }
' "$listing")
[ -n "$headline" ] || { echo "no Headline: line for ${version} in ${listing}" >&2; exit 1; }
printf '%s\n' "$headline"
