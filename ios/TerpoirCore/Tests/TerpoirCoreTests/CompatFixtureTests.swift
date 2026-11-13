// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// Journals written by every released version open on iOS (docs/roadmap.md,
// "iOS", I1): the compatibility fixtures (core/crates/terpoir-ffi/tests/
// fixtures, copied into this test bundle by ios/scripts/build-core.sh),
// through the Swift bindings on the simulator. The Rust test
// (compat_fixtures.rs) checks every field; this one proves the iOS build
// reads the same files: SQLCipher databases, encrypted exports, the keys.

import Foundation
import XCTest
@testable import TerpoirCore

final class CompatFixtureTests: XCTestCase {
    private var scratch: URL!

    override func setUpWithError() throws {
        `init`()
        scratch = FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-compat-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: scratch, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: scratch)
    }

    func testEveryReleasedDatabaseAndExportOpens() throws {
        let root = try XCTUnwrap(Bundle.module.url(forResource: "Fixtures", withExtension: nil), "fixtures missing from the test bundle")
        let versions = try FileManager.default
            .contentsOfDirectory(at: root, includingPropertiesForKeys: [.isDirectoryKey])
            .filter { $0.hasDirectoryPath }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
        // v0.1.0 to v0.9.0 at the time of writing; more only get added.
        XCTAssertGreaterThanOrEqual(versions.count, 9)
        for version in versions {
            try check(version)
        }
    }

    private func check(_ fixture: URL) throws {
        let name = fixture.lastPathComponent
        let manifest = try XCTUnwrap(
            JSONSerialization.jsonObject(with: Data(contentsOf: fixture.appendingPathComponent("manifest.json"))) as? [String: Any]
        )
        let recoveryKey = try XCTUnwrap(manifest["recovery_key"] as? String)
        let ids = try XCTUnwrap(manifest["ids"] as? [String: String])
        func bytes(_ path: String) throws -> Data { try Data(contentsOf: fixture.appendingPathComponent(path)) }
        let masterWrapped = try bytes("keys/master_wrapped_by_recovery.bin")

        // The database as the release left it on the phone.
        let device = scratch.appendingPathComponent("\(name)-device")
        try FileManager.default.copyItem(at: fixture.appendingPathComponent("database"), to: device)
        let journal = try Journal.open(
            dbPath: device.appendingPathComponent("journal.db").path,
            filesDir: device.appendingPathComponent("files").path,
            masterKey: try masterKeyFromRecovery(recoveryKey: recoveryKey, masterWrappedByRecovery: masterWrapped),
            wrappedDatabaseKey: try bytes("keys/wrapped_database_key.bin")
        )
        XCTAssertNotNil(try journal.getBrand(id: XCTUnwrap(ids["brand"])), name)
        XCTAssertNotNil(try journal.getStrain(id: XCTUnwrap(ids["strain"])), name)
        XCTAssertNotNil(try journal.getProduct(id: XCTUnwrap(ids["product"])), name)
        XCTAssertNotNil(try journal.getSpot(id: XCTUnwrap(ids["spot"])), name)
        XCTAssertNotNil(try journal.getSession(id: XCTUnwrap(ids["session"])), name)
        XCTAssertNil(try journal.getBrand(id: XCTUnwrap(ids["deleted_brand"])), name)
        let fromDatabase = try journal.exportPlaintext(nowMs: 0)
        XCTAssertFalse(fromDatabase.isEmpty, name)

        // The release's encrypted export, restored into a new journal,
        // holds the same records.
        let archive = try readExportArchive(archive: bytes("export.terpoir"))
        let keys = try keysForRestore(recoveryKey: recoveryKey, masterWrappedByRecovery: masterWrapped)
        let restoredDir = scratch.appendingPathComponent("\(name)-restored")
        try FileManager.default.createDirectory(at: restoredDir, withIntermediateDirectories: true)
        let restored = try Journal.open(
            dbPath: restoredDir.appendingPathComponent("journal.db").path,
            filesDir: restoredDir.appendingPathComponent("files").path,
            masterKey: keys.masterKey,
            wrappedDatabaseKey: keys.wrappedDatabaseKey
        )
        for blob in archive.blobs {
            XCTAssertEqual(try restored.importSyncBlob(bytes: blob, nowMs: 2_000_000_000_000).deferred, 0, "\(name): a record needs a newer app")
        }
        XCTAssertEqual(try restored.exportPlaintext(nowMs: 0), fromDatabase, "\(name): database and export differ")

        journal.lock()
        restored.lock()
    }
}
