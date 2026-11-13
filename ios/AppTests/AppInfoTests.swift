// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The app target links the core: the app, not only the package, can call it.

import XCTest
@testable import Terpoir

final class AppInfoTests: XCTestCase {
    func testTheAppReachesTheCore() {
        XCTAssertTrue(AppInfo.coreLine.hasPrefix("Core "))
        XCTAssertGreaterThan(AppInfo.coreLine.count, "Core ".count)
    }
}
