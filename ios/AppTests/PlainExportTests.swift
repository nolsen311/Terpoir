// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class PlainExportTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-plain-\(UUID().uuidString)"))
        vault = MasterKeyVault(label: "app.terpoir.tests.\(UUID().uuidString)", wrappedFile: files.wrappedMasterKey, secureEnclave: false)
        let session = JournalSession(files: files, vault: vault)
        _ = try session.create(protection: .unprotectedForTests)
        guard case let .unlocked(journal) = session.state else { return XCTFail("not unlocked") }
        repo = JournalRepository(journal: journal)
    }

    override func tearDown() async throws {
        vault.delete()
        try? FileManager.default.removeItem(at: files.root)
    }

    func testTheZipHoldsTheCoresFilesWithTheJournalInThem() throws {
        try repo.saveBrand(id: nil, BrandData(name: "Kiva"))
        let zip = try repo.exportPlaintextZip()
        XCTAssertEqual(zip.prefix(4), Data("PK\u{03}\u{04}".utf8))
        let text = String(decoding: zip, as: UTF8.self)
        for name in ["terpoir.json", "sessions.csv", "products.csv", "purchases.csv"] {
            XCTAssertTrue(text.contains(name), name)
        }
        // Stored, not compressed, so the journal's text is in it as is.
        XCTAssertTrue(text.contains("Kiva"))
    }

    func testTheZipEndsWithItsDirectoryOfFourFiles() throws {
        let zip = [UInt8](try repo.exportPlaintextZip())
        let end = zip.count - 22
        XCTAssertEqual(Array(zip[end..<end + 4]), [0x50, 0x4B, 0x05, 0x06])
        XCTAssertEqual(UInt16(zip[end + 10]) | UInt16(zip[end + 11]) << 8, 4)
    }
}
