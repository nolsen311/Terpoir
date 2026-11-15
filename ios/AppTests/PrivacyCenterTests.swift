// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import UIKit
import XCTest
@testable import Terpoir

final class PrivacyCenterTests: XCTestCase {
    func testKeyboardProtectionsAreOffUntilTurnedOnAndKept() throws {
        let name = "app.terpoir.tests.privacy.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        let settings = PrivacySettings(defaults: defaults)
        for toggle in KeyboardProtection.toggles {
            XCTAssertFalse(settings.isOn(toggle), toggle.id)
            settings.set(toggle, on: true)
            XCTAssertTrue(PrivacySettings(defaults: defaults).isOn(toggle), toggle.id)
        }
        // The keys the app delegate and the fields read are the toggles'.
        XCTAssertTrue(defaults.bool(forKey: KeyboardProtection.blockKeyboardsKey))
        XCTAssertTrue(defaults.bool(forKey: KeyboardProtection.noAutocorrectKey))
        // Symptoms are health information: hidden until turned on.
        XCTAssertFalse(settings.isOn(JournalToggles.symptoms))
        settings.set(JournalToggles.symptoms, on: true)
        XCTAssertTrue(defaults.bool(forKey: JournalToggles.symptomsKey))
    }

    func testOnlyKeyboardsAreRefusedAndOnlyWhileBlocked() {
        XCTAssertFalse(KeyboardProtection.allowsExtension(.keyboard, blockKeyboards: true))
        XCTAssertTrue(KeyboardProtection.allowsExtension(.keyboard, blockKeyboards: false))
    }

    func testEveryToggleHasItsWholeCard() {
        let toggles = KeyboardProtection.toggles + [JournalToggles.symptoms]
        XCTAssertEqual(Set(toggles.map(\.id)).count, toggles.count)
        for toggle in toggles {
            for text in [toggle.title, toggle.does, toggle.leaves, toggle.who, toggle.undo] {
                XCTAssertFalse(text.isEmpty, toggle.id)
            }
        }
    }
}
