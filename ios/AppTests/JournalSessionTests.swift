// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The journal's lifecycle on the simulator: create, lock, unlock through
// the vault (no prompt in tests), the recovery key, reset.

import LocalAuthentication
import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class JournalSessionTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-session-\(UUID().uuidString)"))
        vault = MasterKeyVault(label: "app.terpoir.tests.\(UUID().uuidString)", wrappedFile: files.wrappedMasterKey, secureEnclave: false)
    }

    override func tearDown() async throws {
        vault.delete()
        try? FileManager.default.removeItem(at: files.root)
    }

    private struct NotUnlocked: Error {}

    private func journal(_ session: JournalSession) throws -> Journal {
        guard case let .unlocked(journal) = session.state else {
            XCTFail("expected the journal unlocked")
            throw NotUnlocked()
        }
        return journal
    }

    func testANewDeviceHasNoJournal() {
        let session = JournalSession(files: files, vault: vault)
        guard case .noJournal = session.state else { return XCTFail("expected noJournal") }
    }

    func testCreateLockAndUnlockKeepTheEntries() async throws {
        let session = JournalSession(files: files, vault: vault)
        _ = try session.create(protection: .unprotectedForTests)
        let id = try journal(session).saveBrand(id: nil, brand: BrandData(name: "Acme"), nowMs: 1)
        session.lock()
        guard case .locked = session.state else { return XCTFail("expected locked") }

        // As after the app restarts: a journal exists, locked.
        let again = JournalSession(files: files, vault: vault)
        guard case .locked = again.state else { return XCTFail("expected locked after restart") }
        try await again.unlock(context: LAContext())
        XCTAssertEqual(try journal(again).getBrand(id: id), BrandData(name: "Acme"))
    }

    func testTheRecoveryKeyIsCheckedNotStored() throws {
        let session = JournalSession(files: files, vault: vault)
        let recoveryKey = try session.create(protection: .unprotectedForTests)
        XCTAssertEqual(session.checkRecoveryKey(recoveryKey), .matches)
        XCTAssertEqual(session.checkRecoveryKey("abc"), .incomplete)
        // Nothing on disk holds the recovery key.
        let enumerator = FileManager.default.enumerator(at: files.root, includingPropertiesForKeys: nil)!
        for case let file as URL in enumerator where !file.hasDirectoryPath {
            let bytes = try Data(contentsOf: file)
            XCTAssertNil(bytes.range(of: Data(recoveryKey.utf8)), file.lastPathComponent)
        }
    }

    func testTheJournalIsExcludedFromBackups() throws {
        let session = JournalSession(files: files, vault: vault)
        _ = try session.create(protection: .unprotectedForTests)
        let values = try files.root.resourceValues(forKeys: [.isExcludedFromBackupKey])
        XCTAssertEqual(values.isExcludedFromBackup, true)
    }

    func testResetRemovesEverything() throws {
        let session = JournalSession(files: files, vault: vault)
        _ = try session.create(protection: .unprotectedForTests)
        session.reset()
        guard case .noJournal = session.state else { return XCTFail("expected noJournal") }
        XCTAssertFalse(FileManager.default.fileExists(atPath: files.root.path))
        XCTAssertFalse(vault.exists())
    }
}
