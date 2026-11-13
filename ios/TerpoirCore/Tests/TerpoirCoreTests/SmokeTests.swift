// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The core on the iOS simulator, through the generated Swift bindings: the
// same calls as the Kotlin smoke test (core/crates/terpoir-ffi/tests/kotlin/
// Smoke.kt). Proves the static library, SQLCipher and OpenSSL work on iOS
// and the bindings are callable; the Rust tests cover behaviour.

import Foundation
import XCTest
@testable import TerpoirCore

final class SmokeTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        `init`()
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-swift-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: dir)
    }

    private func open(_ keys: NewKeys) throws -> Journal {
        try Journal.open(
            dbPath: dir.appendingPathComponent("journal.db").path,
            filesDir: dir.appendingPathComponent("files").path,
            masterKey: keys.masterKey,
            wrappedDatabaseKey: keys.wrappedDatabaseKey
        )
    }

    func testCreateSaveLockAndReopen() throws {
        let keys = try createKeys()
        try checkRecoveryKey(recoveryKey: keys.recoveryKey)

        let journal = try open(keys)
        let id = try journal.saveBrand(id: nil, brand: BrandData(name: "Acme"), nowMs: 1)
        XCTAssertEqual(try journal.getBrand(id: id), BrandData(name: "Acme"))

        journal.lock()
        XCTAssertTrue(journal.isLocked())
        XCTAssertThrowsError(try journal.getBrand(id: id)) { error in
            guard case CoreError.Locked = error else { return XCTFail("expected Locked, got \(error)") }
        }

        // The encrypted database opens again with the same keys.
        let reopened = try open(keys)
        XCTAssertEqual(try reopened.getBrand(id: id), BrandData(name: "Acme"))
        reopened.lock()
    }

    func testABadRecoveryKeyIsRefused() {
        XCTAssertThrowsError(try checkRecoveryKey(recoveryKey: "abc")) { error in
            guard case CoreError.RecoveryKeyLength = error else { return XCTFail("expected RecoveryKeyLength, got \(error)") }
        }
    }

    func testCoreVersion() {
        XCTAssertFalse(coreVersion().isEmpty)
    }
}
