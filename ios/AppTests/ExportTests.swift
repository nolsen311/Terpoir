// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// Export, import and restore through the core: the file a new phone
// restores from, and merging an export back in.

import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class ExportTests: XCTestCase {
    private var cleanup: [() -> Void] = []

    override func tearDown() async throws {
        cleanup.forEach { $0() }
        cleanup = []
    }

    /// A new journal on a "phone" of its own (directory and vault key).
    private func phone() -> JournalSession {
        let files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-export-\(UUID().uuidString)"))
        let vault = MasterKeyVault(label: "app.terpoir.tests.\(UUID().uuidString)", wrappedFile: files.wrappedMasterKey, secureEnclave: false)
        cleanup.append { vault.delete(); try? FileManager.default.removeItem(at: files.root) }
        return JournalSession(files: files, vault: vault)
    }

    private func repo(_ session: JournalSession) throws -> JournalRepository {
        guard case let .unlocked(journal) = session.state else {
            XCTFail("expected the journal unlocked")
            throw CoreError.Locked(message: "")
        }
        return JournalRepository(journal: journal)
    }

    func testANewPhoneRestoresAnExport() throws {
        let old = phone()
        let recoveryKey = try old.create(protection: .unprotectedForTests)
        let oldRepo = try repo(old)
        let brand = try oldRepo.saveBrand(id: nil, BrandData(name: "Acme"))
        let file = try oldRepo.exportArchive(masterWrappedByRecovery: old.masterWrappedByRecovery())

        let new = phone()
        let restored = try new.restore(recoveryKey: recoveryKey, archive: readExportArchive(archive: file), protection: .unprotectedForTests)
        XCTAssertGreaterThan(restored, 0)
        XCTAssertEqual(try repo(new).brands().first { $0.id == brand }?.data, BrandData(name: "Acme"))
        // The restored journal keeps working: locked and unlocked again.
        new.lock()
        XCTAssertFalse(new.isUnlocked)
    }

    func testTheWrongRecoveryKeyKeepsNothing() throws {
        let old = phone()
        _ = try old.create(protection: .unprotectedForTests)
        let file = try repo(old).exportArchive(masterWrappedByRecovery: old.masterWrappedByRecovery())
        let other = phone()
        let otherKey = try other.create(protection: .unprotectedForTests)

        let new = phone()
        XCTAssertThrowsError(try new.restore(recoveryKey: otherKey, archive: readExportArchive(archive: file), protection: .unprotectedForTests))
        XCTAssertFalse(new.hasJournal)
    }

    func testAnExportOfThisJournalImportsBack() throws {
        let session = phone()
        _ = try session.create(protection: .unprotectedForTests)
        let repository = try repo(session)
        let brand = try repository.saveBrand(id: nil, BrandData(name: "Acme"))
        let file = try repository.exportArchive(masterWrappedByRecovery: session.masterWrappedByRecovery())
        // Merging the journal's own export changes nothing and adds no copies.
        _ = try repository.importArchive(file)
        XCTAssertEqual(repository.brands().map(\.id), [brand])
    }

    func testAnotherJournalsExportIsRefused() throws {
        let one = phone()
        _ = try one.create(protection: .unprotectedForTests)
        let file = try repo(one).exportArchive(masterWrappedByRecovery: one.masterWrappedByRecovery())
        let two = phone()
        _ = try two.create(protection: .unprotectedForTests)
        XCTAssertThrowsError(try repo(two).importArchive(file))
    }
}
