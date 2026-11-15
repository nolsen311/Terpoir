// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import XCTest
@testable import Terpoir

final class AutoLockTests: XCTestCase {
    func testImmediatelyLocksAsTheAppLeaves() {
        XCTAssertTrue(AutoLockPolicy(timeout: .immediately).lockOnBackground)
        XCTAssertFalse(AutoLockPolicy(timeout: .fiveMinutes).lockOnBackground)
    }

    func testATimerLocksOnlyOnceItHasPassed() {
        let policy = AutoLockPolicy(timeout: .fiveMinutes)
        XCTAssertFalse(policy.lockOnReturn(backgroundedAt: 1_000, now: 1_299, deviceLocked: false))
        XCTAssertTrue(policy.lockOnReturn(backgroundedAt: 1_000, now: 1_300, deviceLocked: false))
        XCTAssertFalse(policy.lockOnReturn(backgroundedAt: nil, now: 1_300, deviceLocked: false))
    }

    func testAClockThatWentBackwardsLocks() {
        XCTAssertTrue(AutoLockPolicy(timeout: .fifteenMinutes).lockOnReturn(backgroundedAt: 1_000, now: 999, deviceLocked: false))
    }

    func testALockedPhoneAlwaysLocks() {
        for timeout in LockTimeout.allCases {
            XCTAssertTrue(AutoLockPolicy(timeout: timeout).lockOnReturn(backgroundedAt: nil, now: 0, deviceLocked: true), timeout.rawValue)
        }
    }

    func testTheChoiceIsKeptAndDefaultsToFiveMinutes() throws {
        let name = "app.terpoir.tests.lock.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let settings = LockSettings(defaults: defaults)
        XCTAssertEqual(settings.timeout, .fiveMinutes)
        settings.timeout = .immediately
        XCTAssertEqual(LockSettings(defaults: defaults).timeout, .immediately)
        defaults.set("WITH_DEVICE", forKey: "lock_timeout")
        XCTAssertEqual(LockSettings(defaults: defaults).timeout, .fiveMinutes)
    }
}
