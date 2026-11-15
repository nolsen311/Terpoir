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

    func testTheExportHasTheCoresFilesWithTheJournalInThem() throws {
        try repo.saveBrand(id: nil, BrandData(name: "Kiva"))
        let exported = try repo.exportPlaintext()
        let names = Set(exported.map(\.name))
        XCTAssertTrue(names.isSuperset(of: ["terpoir.json", "sessions.csv", "products.csv", "purchases.csv"]), "\(names)")
        let json = try XCTUnwrap(exported.first { $0.name == "terpoir.json" })
        XCTAssertTrue(String(decoding: json.bytes, as: UTF8.self).contains("Kiva"))
    }

    func testTheSavedFolderHoldsEveryFile() throws {
        let exported = try repo.exportPlaintext()
        let folder = PlainExportDocument.folder(exported)
        XCTAssertTrue(folder.isDirectory)
        XCTAssertEqual(Set(folder.fileWrappers.map { Array($0.keys) } ?? []), Set(exported.map(\.name)))
        for file in exported {
            XCTAssertEqual(folder.fileWrappers?[file.name]?.regularFileContents, file.bytes, file.name)
        }
    }
}
