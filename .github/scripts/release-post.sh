#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-post.sh VERSION URL
#
# Prints the Mastodon post announcing release VERSION (for example 0.5.2):
# a first line, that version's Play release notes from
# docs/play-listing.md, URL (the GitHub release) and the hashtags (in
# CamelCase, which screen readers read as words). Mastodon allows 500
# characters and counts any link as 23, so notes that don't fit are cut at
# a word and end with "…". Fails if docs/play-listing.md has no notes for
# VERSION.

set -eu

version=$1
url=$2

notes=$(.github/scripts/release-notes.sh "$version")

head="🌿 Terpoir ${version} is out."
tags="#Terpoir #Android #FOSS #Privacy #DataPrivacy #SelfAwareness"
# 500, less the first line, the link (23), the hashtags and the three
# blank lines between the four parts.
room=$((500 - ${#head} - 23 - ${#tags} - 6))
notes=$(printf '%s' "$notes" | python3 -c '
import sys
room = int(sys.argv[1])
text = sys.stdin.read().strip()
if len(text) > room:
    text = text[: room - 1].rsplit(" ", 1)[0].rstrip(" ,;:") + "…"
print(text, end="")
' "$room")

printf '%s\n\n%s\n\n%s\n\n%s\n' "$head" "$notes" "$url" "$tags"
