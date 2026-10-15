#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-post.sh VERSION URL
#
# Prints the Mastodon post announcing release VERSION (for example 0.5.2):
# a first line, that version's Play release notes from
# docs/play-listing.md, and URL (the GitHub release). Mastodon allows 500
# characters and counts any link as 23, so notes that don't fit are cut at
# a word and end with "…". Fails if docs/play-listing.md has no notes for
# VERSION.

set -eu

version=$1
url=$2
listing=docs/play-listing.md

# The notes are the line after "<en-US>" in the block that follows the
# version's "**X.Y.Z**" heading.
notes=$(awk -v v="**${version}**" '
    index($0, v) { found = 1; next }
    found && /<en-US>/ { getline; sub(/^[ \t]+/, ""); print; exit }
' "$listing")
[ -n "$notes" ] || { echo "no release notes for ${version} in ${listing}" >&2; exit 1; }

head="Terpoir ${version} is out."
# 500, less the first line, two blank lines and the link (23).
room=$((500 - ${#head} - 4 - 23))
notes=$(printf '%s' "$notes" | python3 -c '
import sys
room = int(sys.argv[1])
text = sys.stdin.read().strip()
if len(text) > room:
    text = text[: room - 1].rsplit(" ", 1)[0].rstrip(" ,;:") + "…"
print(text, end="")
' "$room")

printf '%s\n\n%s\n\n%s\n' "$head" "$notes" "$url"
