#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: release-post.sh VERSION URL
#
# Prints the Mastodon post announcing release VERSION (for example 0.5.2):
# a first line, that version's summary as points (release-summary.sh; short
# on purpose, the details are on the release page), URL (the GitHub
# release) and the hashtags (in CamelCase, which screen readers read as
# words). Mastodon allows 500 characters and counts any link as 23; a post
# that doesn't fit fails, so the summary is shortened in
# docs/play-listing.md rather than cut here.

set -eu

version=$1
url=$2

points=$(.github/scripts/release-summary.sh "$version" | sed 's/^/• /')

head="🌿 Terpoir ${version} is out."
tags="#Terpoir #Android #FOSS #Privacy #DataPrivacy #SelfAwareness"
post=$(printf '%s\n\n%s\n\n%s\n\n%s' "$head" "$points" "$url" "$tags")
# Mastodon counts characters (code points), and any link as 23.
length=$(printf '%s' "$post" | python3 -c '
import sys
url = sys.argv[1]
text = sys.stdin.read()
print(len(text) - len(url) + 23)
' "$url")
[ "$length" -le 500 ] || { echo "post for ${version} is ${length} characters; shorten its Summary: line" >&2; exit 1; }

printf '%s\n' "$post"
