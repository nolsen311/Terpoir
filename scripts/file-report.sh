#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Turns a problem report someone emailed (the app's "Send a report") into a
# public GitHub issue, for people without a GitHub account
# (docs/design-decisions.md, "Problem reports").
#
# Usage: scripts/file-report.sh [FILE]
#   Reads the email's text from FILE, or from standard input, e.g.
#   `pbpaste | scripts/file-report.sh` or `wl-paste | scripts/file-report.sh`.
#   Paste the message body only, not its headers.
#
# It keeps the reporter's description and the app's details block, drops
# quoted replies and signatures, and never adds a name or address. The
# draft opens in $EDITOR (default nano) so you can remove anything
# personal before it is posted; posting asks once more.
#
# Settings (environment):
#   TERPOIR_REPO  default nolsen311/Terpoir

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

repo=${TERPOIR_REPO:-nolsen311/Terpoir}
command -v gh >/dev/null || die "gh not found (https://cli.github.com), then: gh auth login"

text=$(tr -d '\r' <"${1:-/dev/stdin}")
[ -n "$text" ] || die "no report text: paste the email body, or give a file"

# The app's details block sits between two "----" lines.
details=$(awk '/^----$/ { n++; next } n == 1' <<<"$text")
[ -n "$details" ] || die "no details block found: is this a Terpoir report (lines between two \"----\" lines)?"
problem=$(sed -n 's/^Problem: //p' <<<"$details" | head -n 1)

# What the reporter wrote: the text before the details block, without the
# app's prompt line, quoted text or a signature.
description=$(awk '/^----$/ { exit } { print }' <<<"$text" \
    | grep -v '^What were you doing when this happened' \
    | grep -v '^>' \
    | sed '/^-- $/,$d' \
    | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}' \
    | sed '/./,$!d' || true)

draft=$(mktemp "${TMPDIR:-/tmp}/terpoir-report.XXXXXX")
trap 'rm -f "$draft"' EXIT
{
    echo "Title: User report: ${problem:-problem}"
    echo "# Everything below the title line becomes the issue. Remove anything"
    echo "# personal (names, places, products) before saving. Lines starting"
    echo "# with '#' are dropped."
    echo
    echo "Reported by email through the app (reporter not named)."
    echo
    echo "### What happened"
    echo
    # shellcheck disable=SC2001
    if [ -n "$description" ]; then sed 's/^/> /' <<<"$description"; else echo "_No description given._"; fi
    echo
    echo "### Details from the app"
    echo
    echo '```'
    echo "$details"
    echo '```'
} >"$draft"

"${EDITOR:-nano}" "$draft"

title=$(sed -n 's/^Title: //p' "$draft" | head -n 1)
body=$(grep -v '^Title: ' "$draft" | grep -v '^#[^#]' | grep -v '^#$')
[ -n "$title" ] || die "the draft has no 'Title:' line"

# An email address in the body is almost always a mistake.
if grep -Eq '[[:alnum:]._%+-]+@[[:alnum:].-]+\.[[:alpha:]]{2,}' <<<"$body"; then
    echo "The draft still contains an email address:" >&2
    grep -En '[[:alnum:]._%+-]+@[[:alnum:].-]+\.[[:alpha:]]{2,}' <<<"$body" >&2
    die "remove it and run this again"
fi

echo
echo "Title: $title"
echo "$body"
echo
read -r -p "Post this as a public issue on $repo? [y/N] " answer
[ "$answer" = y ] || [ "$answer" = Y ] || { echo "Not posted."; exit 0; }

# The labels, created the first time (--force leaves existing ones alone
# apart from their colour and description).
gh label create bug --repo "$repo" --color d73a4a --description "Something isn't working" --force >/dev/null
gh label create user-report --repo "$repo" --color 5319e7 --description "Reported by email through the app" --force >/dev/null
gh issue create --repo "$repo" --title "$title" --body "$body" --label bug --label user-report
