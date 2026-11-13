// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import XCTest
@testable import Terpoir

final class OnboardingTests: XCTestCase {
    func testTheStepsInOrder() {
        XCTAssertEqual(onboardingStep(hasJournal: false, ageGatePassed: false, backupPending: false), .age)
        XCTAssertEqual(onboardingStep(hasJournal: false, ageGatePassed: true, backupPending: false), .create)
        XCTAssertEqual(onboardingStep(hasJournal: true, ageGatePassed: true, backupPending: true), .backup)
        XCTAssertEqual(onboardingStep(hasJournal: true, ageGatePassed: true, backupPending: false), .done)
    }

    func testAJournalSkipsTheAgeGate() {
        // A journal set up before the gate existed isn't asked again.
        XCTAssertEqual(onboardingStep(hasJournal: true, ageGatePassed: false, backupPending: false), .done)
    }

    func testTheFlagsPersist() throws {
        let suite = "app.terpoir.tests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let flags = OnboardingFlags(defaults: defaults)
        XCTAssertFalse(flags.ageGatePassed)
        flags.ageGatePassed = true
        flags.backupPending = true
        let again = OnboardingFlags(defaults: defaults)
        XCTAssertTrue(again.ageGatePassed)
        XCTAssertTrue(again.backupPending)
    }
}
