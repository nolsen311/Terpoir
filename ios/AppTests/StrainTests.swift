// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class StrainTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-strain-\(UUID().uuidString)"))
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

    func testAStrainIsSavedWithWhatTheEditorShows() throws {
        var fields = StrainFields()
        fields.name = "  Blue Dream "
        fields.type = "hybrid"
        fields.terpenes = [.builtin(slug: "myrcene")]
        fields.lineage = " "
        fields.breeder = "DJ Short"
        let id = try repo.saveStrain(id: nil, fields.data(keeping: nil, tracking: .everything))

        let saved = try XCTUnwrap(repo.strain(id))
        XCTAssertEqual(saved.name, "Blue Dream")
        XCTAssertEqual(saved.strainType, "hybrid")
        XCTAssertEqual(saved.terpenes, [.builtin(slug: "myrcene")])
        XCTAssertNil(saved.lineage)
        XCTAssertEqual(saved.breeder, "DJ Short")
        XCTAssertEqual(StrainFields(saved).name, "Blue Dream")
    }

    func testFieldsTheEditorDoesntShowAreKept() throws {
        let parent = try repo.saveStrain(id: nil, .named("Blueberry"))
        let myrcene = TagRefData.builtin(slug: "myrcene")
        let id = try repo.saveStrain(id: nil, StrainData(
            name: "Blue Dream", lineage: nil, terpenes: [myrcene], strainType: nil,
            terpeneLevels: [TerpeneLevelData(terpene: myrcene, level: "dominant")],
            parents: [parent], cannabinoids: ["cbg"]
        ))
        let kept = try XCTUnwrap(repo.strain(id))

        var fields = StrainFields(kept)
        fields.type = "sativa"
        try repo.saveStrain(id: id, fields.data(keeping: kept, tracking: .everything))

        let saved = try XCTUnwrap(repo.strain(id))
        XCTAssertEqual(saved.strainType, "sativa")
        XCTAssertEqual(saved.parents, [parent])
        XCTAssertEqual(saved.cannabinoids, ["cbg"])
        XCTAssertEqual(saved.terpeneLevels, [TerpeneLevelData(terpene: myrcene, level: "dominant")])
    }

    func testCustomTerpenesAreListedByName() throws {
        let zeta = try repo.saveTag(id: nil, TagData(name: "Zeta-ene", category: "terpene"))
        let alpha = try repo.saveTag(id: nil, TagData(name: "alpha-thing", category: "terpene"))
        try repo.saveTag(id: nil, TagData(name: "Calm", category: "effect"))

        XCTAssertEqual(repo.tags(category: "terpene").map(\.id), [alpha, zeta])
        XCTAssertEqual(tagLabel(.custom(id: zeta), [zeta: "Zeta-ene"]), "Zeta-ene")
        XCTAssertEqual(tagLabel(.builtin(slug: "alpha_pinene"), [:]), "α-Pinene")
    }

    func testABrandIsRenamedAndDeleted() throws {
        let id = try repo.saveBrand(id: nil, BrandData(name: "Old name"))
        try repo.saveBrand(id: id, BrandData(name: "New name"))
        XCTAssertEqual(repo.brand(id)?.name, "New name")
        XCTAssertEqual(repo.brands().count, 1)

        try repo.delete(id)
        XCTAssertNil(repo.brand(id))
        XCTAssertTrue(repo.brands().isEmpty)
    }
}
