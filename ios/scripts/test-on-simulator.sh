#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Runs `xcodebuild test` on the first available iPhone simulator and fails
# unless at least MIN tests ran and passed, so green always means the
# tests ran. Prints each test case and the summary.
#
# Usage: ios/scripts/test-on-simulator.sh MIN <xcodebuild arguments...>
#   e.g. ios/scripts/test-on-simulator.sh 3 -scheme TerpoirCore

set -euo pipefail

min=$1
shift
udid=$(xcrun simctl list devices available --json | python3 -c '
import json, sys
devices = json.load(sys.stdin)["devices"]
print(next(d["udid"] for runtime, ds in sorted(devices.items(), reverse=True)
           if "iOS" in runtime for d in ds if d["name"].startswith("iPhone")))')
echo "Simulator: $udid"
log=$(mktemp)
# Simulator builds need no signing.
xcodebuild test "$@" -destination "id=$udid" CODE_SIGNING_ALLOWED=NO 2>&1 | tee "$log" \
    | grep -E "Test (Case|Suite)|Executed|error:|TEST (SUCCEEDED|FAILED)"
ran=$(sed -n 's/.*Executed \([0-9]*\) tests\{0,1\}, with 0 failures.*/\1/p' "$log" | tail -1)
rm -f "$log"
if [ "${ran:-0}" -lt "$min" ]; then
    echo "::error::expected at least $min passing tests, found ${ran:-none}"
    exit 1
fi
echo "Tests passed: $ran"
