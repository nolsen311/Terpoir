#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Tests the TerpoirCore package on the iOS simulator (after build-core.sh).
# The minimum number of tests lives here, beside the tests, so adding one
# changes only ios/.

set -euo pipefail

# SmokeTests (3) and CompatFixtureTests (1).
MIN_TESTS=4

cd "$(dirname "$0")/../TerpoirCore"
# Xcode names the package's scheme after it, or "<name>-Package" when it
# makes one per product.
scheme=TerpoirCore
if xcodebuild -list | grep -q 'TerpoirCore-Package'; then scheme=TerpoirCore-Package; fi
../scripts/test-on-simulator.sh "$MIN_TESTS" -scheme "$scheme"
