// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import LocalAuthentication
import TerpoirCore
import XCTest
@testable import Terpoir

final class LabelTests: XCTestCase {
    func testASlugReadsAsWords() {
        XCTAssertEqual(label("vape_cart"), "Vape cart")
        XCTAssertEqual(label("flower"), "Flower")
    }

    func testProductTitlesFollowTheCards() {
        XCTAssertEqual(
            productTitle(name: "Block Berry", strains: ["Blue Dream"], brand: "Fire Bros", form: "Cart"),
            ProductTitle(title: "Blue Dream", brand: "Fire Bros", detail: "Cart · Block Berry")
        )
        XCTAssertEqual(
            productTitle(name: "", strains: [], brand: "Fire Bros", form: nil),
            ProductTitle(title: "Fire Bros", brand: "", detail: "")
        )
        XCTAssertEqual(
            productTitle(name: "Doozies", strains: [], brand: nil, form: "Edible"),
            ProductTitle(title: "Doozies", brand: "", detail: "Edible")
        )
    }
}

@MainActor
final class JournalRepositoryTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-repo-\(UUID().uuidString)"))
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

    func testStrainsAreFoundOrMade() throws {
        let first = try repo.strainIds(named: ["Blue Dream"])
        let again = try repo.strainIds(named: ["blue dream", "Crater Noir"])
        XCTAssertEqual(again.first, first.first)
        XCTAssertEqual(repo.strains().map(\.data.name), ["Blue Dream", "Crater Noir"])
    }

    func testSessionsAreNewestFirst() throws {
        func session(_ ms: Int64) -> SessionData {
            SessionData(
                when: LocalTimeData(unixMs: ms, utcOffsetMin: 0), lines: [], activity: nil, intention: nil, moodBefore: nil,
                social: nil, effects: [], onsetMin: nil, peakMin: nil, durationMin: nil, moodAfter: nil, rating: 4,
                wouldRepeat: nil, spot: nil, symptoms: [], attachments: []
            )
        }
        let older = try repo.saveSession(id: nil, session(1_000))
        let newer = try repo.saveSession(id: nil, session(2_000))
        XCTAssertEqual(repo.sessions().map(\.id), [newer, older])
        try repo.delete(older)
        XCTAssertEqual(repo.sessions().map(\.id), [newer])
    }
}
