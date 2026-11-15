// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import TerpoirCore
import XCTest
@testable import Terpoir

final class PriceTests: XCTestCase {
    func testTypedPricesBecomeMinorUnits() {
        XCTAssertEqual(parsePrice("12.34", currencyCode: "USD"), 1234)
        XCTAssertEqual(parsePrice("12,5", currencyCode: "EUR"), 1250)
        XCTAssertEqual(parsePrice("12", currencyCode: "JPY"), 12)
    }

    func testWhatIsntAnAmountIsRefused() {
        XCTAssertNil(parsePrice("1.5", currencyCode: "JPY"))
        XCTAssertNil(parsePrice("1.234", currencyCode: "USD"))
        XCTAssertNil(parsePrice("abc", currencyCode: "USD"))
        XCTAssertNil(parsePrice("1.2.3", currencyCode: "USD"))
        XCTAssertNil(parsePrice("", currencyCode: "USD"))
    }

    func testPricesShowInTheirCurrency() {
        XCTAssertTrue(formatPrice(1234, currencyCode: "USD").contains("12.34"), formatPrice(1234, currencyCode: "USD"))
        XCTAssertFalse(formatPrice(1200, currencyCode: "JPY").contains("."), formatPrice(1200, currencyCode: "JPY"))
    }
}

@MainActor
final class PurchaseTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-purchase-\(UUID().uuidString)"))
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

    private func product(_ name: String) throws -> String {
        try repo.saveProduct(id: nil, ProductData(
            name: name, form: "flower", brand: nil, strains: [], thcMilli: nil, thcUnit: nil, cbdMilli: nil, cbdUnit: nil,
            terpenes: [], batch: nil, packageDateDay: nil, identifiers: [], shops: []
        ))
    }

    private func purchase(_ product: String, at ms: Int64, shop: String?, price: Int64?) throws -> String {
        try repo.savePurchase(id: nil, PurchaseData(
            when: LocalTimeData(unixMs: ms, utcOffsetMin: 0),
            spot: shop,
            lines: [PurchaseLineData(lineId: nil, product: product, batch: nil, priceMinorUnits: price, priceCurrency: price == nil ? nil : "USD")]
        ))
    }

    func testAProductsPurchasesAreNewestFirst() throws {
        let gummies = try product("Gummies")
        let cart = try product("Cart")
        let shop = try repo.saveShop(named: "Green Leaf")
        let older = try purchase(gummies, at: 1_000, shop: shop, price: 2_500)
        let newer = try purchase(gummies, at: 2_000, shop: nil, price: nil)
        _ = try purchase(cart, at: 3_000, shop: shop, price: 4_000)
        XCTAssertEqual(repo.purchases(of: gummies).map(\.id), [newer, older])
    }

    func testANewShopIsAReUpSpot() throws {
        let shop = try repo.saveShop(named: "Green Leaf")
        let spot = try XCTUnwrap(repo.spots().first { $0.id == shop })
        XCTAssertEqual(spot.data.name, "Green Leaf")
        XCTAssertEqual(spot.data.purposes, [.builtin(slug: "re_up")])
        XCTAssertNil(spot.data.latE7)
    }

    func testAPurchaseReadsAsWhenWhereAndPrice() throws {
        let gummies = try product("Gummies")
        let shop = try repo.saveShop(named: "Green Leaf")
        _ = try purchase(gummies, at: 1_700_000_000_000, shop: shop, price: 2_500)
        let entry = try XCTUnwrap(repo.purchases(of: gummies).first)
        let text = purchaseLine(entry.data, product: gummies, shops: [shop: "Green Leaf"])
        XCTAssertTrue(text.contains("Green Leaf"), text)
        XCTAssertTrue(text.contains("25.00"), text)
    }
}
