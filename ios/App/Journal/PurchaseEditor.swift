// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// Logs buying a product by hand: when, where, the price and how many.
/// (On Android purchases come from receipt scans; iOS gets those with its
/// scanning, later. Both write the same purchase record.)
struct PurchaseEditor: View {
    @ObservedObject var repo: JournalRepository
    let product: String
    @Environment(\.dismiss) private var dismiss

    @State private var when = Date()
    @State private var shop = ""
    @State private var newShop = ""
    @State private var price = ""
    @State private var currency = localCurrency
    @State private var quantity = 1
    @State private var error: String?

    private static let newShopTag = "+new"

    var body: some View {
        let shops = repo.spots()
        Form {
            DatePicker("When", selection: $when, displayedComponents: .date)
            Picker("Shop", selection: $shop) {
                Text("Not saying").tag("")
                ForEach(shops, id: \.id) { Text($0.data.name).tag($0.id) }
                Text("New shop…").tag(Self.newShopTag)
            }
            if shop == Self.newShopTag {
                TextField("Shop name", text: $newShop).textInputAutocapitalization(.words)
            }
            HStack {
                TextField("Price (optional)", text: $price).keyboardType(.decimalPad)
                TextField("Currency", text: $currency)
                    .textInputAutocapitalization(.characters)
                    .autocorrectionDisabled()
                    .frame(width: 64)
            }
            Stepper("How many: \(quantity)", value: $quantity, in: 1...99)
            if let error {
                Text(error).foregroundStyle(.red)
            }
        }
        .navigationTitle("Log a purchase")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Save", action: save) }
        }
    }

    private func save() {
        let code = currency.trimmingCharacters(in: .whitespaces).uppercased()
        var minor: Int64?
        if !price.trimmingCharacters(in: .whitespaces).isEmpty {
            guard code.count == 3, let parsed = parsePrice(price, currencyCode: code) else {
                error = "That price isn't an amount in \(code.isEmpty ? "that currency" : code)."
                return
            }
            minor = parsed
        }
        do {
            var spot: String? = shop.isEmpty ? nil : shop
            if shop == Self.newShopTag {
                let name = newShop.trimmingCharacters(in: .whitespaces)
                guard !name.isEmpty else {
                    error = "Give the shop a name."
                    return
                }
                spot = try repo.saveShop(named: name)
            }
            let line = PurchaseLineData(
                lineId: nil, product: product, batch: nil,
                priceMinorUnits: minor, priceCurrency: minor == nil ? nil : code,
                quantity: UInt32(quantity)
            )
            let time = LocalTimeData(
                unixMs: Int64(when.timeIntervalSince1970 * 1000),
                utcOffsetMin: Int16(TimeZone.current.secondsFromGMT(for: when) / 60)
            )
            try repo.savePurchase(id: nil, PurchaseData(when: time, spot: spot, lines: [line]))
            dismiss()
        } catch {
            self.error = "The purchase couldn't be saved. Try again."
        }
    }
}

/// A purchase as the product editor lists it (as Android): when, where, price.
func purchaseLine(_ purchase: PurchaseData, product: String, shops: [String: String]) -> String {
    let line = purchase.lines.first { $0.product == product }
    let price = line?.priceMinorUnits.map { formatPrice($0, currencyCode: line?.priceCurrency) }
    let quantity = (line?.quantity ?? 1) > 1 ? "×\(line?.quantity ?? 1)" : nil
    let date = Date(timeIntervalSince1970: Double(purchase.when.unixMs) / 1000)
        .formatted(date: .abbreviated, time: .omitted)
    return [date, purchase.spot.flatMap { shops[$0] }, quantity, price].compactMap { $0 }.joined(separator: " · ")
}
