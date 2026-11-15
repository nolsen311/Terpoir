// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class TrackingTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-tracking-\(UUID().uuidString)"))
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

    private func product(brand: String?) throws -> String {
        try repo.saveProduct(id: nil, ProductData(
            name: "Gummies", form: "flower", brand: brand, strains: [], thcMilli: nil, thcUnit: nil, cbdMilli: nil, cbdUnit: nil,
            terpenes: [], batch: nil, packageDateDay: nil, identifiers: [], shops: []
        ))
    }

    func testEverythingIsTrackedAtFirst() {
        let tracking = repo.tracking()
        XCTAssertTrue(DataPoint.allCases.allSatisfy(tracking.shows))
        XCTAssertEqual(tracking.strainDetail, .terpenes)
        // Every slug here is one the core knows.
        for point in DataPoint.allCases {
            XCTAssertNoThrow(try repo.setTracked(point, true), point.rawValue)
        }
    }

    func testHidingKeepsTheValuesAndShowingBringsThemBack() throws {
        let brand = try repo.saveBrand(id: nil, BrandData(name: "Kiva"))
        let id = try product(brand: brand)

        try repo.setTracked(.brand, false)
        XCTAssertFalse(repo.tracking().shows(.brand))
        XCTAssertEqual(repo.product(id)?.brand, brand)
        XCTAssertNil(CatalogNames(repo).brands[brand])

        try repo.setTracked(.brand, true)
        XCTAssertTrue(repo.tracking().shows(.brand))
        XCTAssertEqual(CatalogNames(repo).brands[brand], "Kiva")
    }

    func testDeletingADataPointRemovesItsValues() throws {
        let brand = try repo.saveBrand(id: nil, BrandData(name: "Kiva"))
        let id = try product(brand: brand)

        try repo.setTracked(.brand, false)
        XCTAssertGreaterThan(try repo.deleteDataPoint(.brand), 0)
        XCTAssertNil(repo.product(id)?.brand)
        XCTAssertTrue(repo.brands().isEmpty)
    }

    func testHidingPurchasesHidesPricesAndShops() throws {
        try repo.setTracked(.purchases, false)
        XCTAssertFalse(repo.tracking().shows(.priceAndShop))
    }

    func testTheStrainDetailStepDecidesWhatIsSaved() throws {
        let parent = try repo.saveStrain(id: nil, .named("Blueberry"))
        let myrcene = TagRefData.builtin(slug: "myrcene")
        var fields = StrainFields()
        fields.name = "Blue Dream"
        fields.terpenes = [myrcene]
        fields.levels = [myrcene: "dominant"]
        fields.parents = [parent]
        fields.cannabinoids = ["cbg"]
        fields.breeder = "DJ Short"

        try repo.setStrainDetail(.detailed)
        let id = try repo.saveStrain(id: nil, fields.data(keeping: nil, tracking: repo.tracking()))
        let detailed = try XCTUnwrap(repo.strain(id))
        XCTAssertEqual(detailed.terpeneLevels, [TerpeneLevelData(terpene: myrcene, level: "dominant")])
        XCTAssertEqual(detailed.parents, [parent])
        XCTAssertEqual(detailed.cannabinoids, ["cbg"])

        // Simple shows only the type; everything else is kept as saved.
        try repo.setStrainDetail(.simple)
        XCTAssertEqual(repo.tracking().strainDetail, .simple)
        var simple = StrainFields()
        simple.name = "Blue Dream"
        simple.type = "hybrid"
        try repo.saveStrain(id: id, simple.data(keeping: detailed, tracking: repo.tracking()))
        let saved = try XCTUnwrap(repo.strain(id))
        XCTAssertEqual(saved.strainType, "hybrid")
        XCTAssertEqual(saved.terpenes, [myrcene])
        XCTAssertEqual(saved.terpeneLevels, detailed.terpeneLevels)
        XCTAssertEqual(saved.parents, [parent])
        XCTAssertEqual(saved.breeder, "DJ Short")
    }
}
