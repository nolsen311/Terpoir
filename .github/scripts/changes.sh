#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Usage: changes.sh REGEX
#
# Prints "run=true" when the pull request being built changes a file whose
# path matches REGEX (Markdown files aside), and for every other event
# (pushes to main, schedules, manual runs); prints "run=false" otherwise.
# Workflows append it to $GITHUB_OUTPUT and skip their heavy jobs on
# "false": a skipped job still reports, so it can be a required check.
# Needs GH_TOKEN with pull-requests: read, and PR_NUMBER.

set -eu

regex=$1
if [ "${GITHUB_EVENT_NAME}" != "pull_request" ]; then
    echo "run=true"
    exit 0
fi
files=$(gh api --paginate "repos/${GITHUB_REPOSITORY}/pulls/${PR_NUMBER}/files" --jq '.[].filename')
if printf '%s\n' "$files" | grep -v '\.md$' | grep -qE "$regex"; then
    echo "run=true"
else
    echo "run=false"
    echo "No changes matching ${regex} (Markdown aside): skipping." >&2
fi
