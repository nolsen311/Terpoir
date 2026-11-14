#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
# Copyright (C) 2026 The Terpoir Authors
# See LICENSING.md for the additional permission under GPLv3 section 7.
#
# Generates the Xcode project (XcodeGen) and tests the app on the iOS
# simulator (after build-core.sh). The minimum number of tests lives here,
# beside the tests, so adding one changes only ios/.

set -euo pipefail

# AppInfoTests (1), MasterKeyVaultTests (5), JournalSessionTests (5).
MIN_TESTS=11

cd "$(dirname "$0")/.."
xcodegen generate --spec project.yml
scripts/test-on-simulator.sh "$MIN_TESTS" -project Terpoir.xcodeproj -scheme Terpoir
