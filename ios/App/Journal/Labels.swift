// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation

/// A vocabulary slug's display name, as the Android app shows it
/// (`vocabularyLabels`, generated from Android's). A slug this version
/// doesn't know (from a newer app) reads as words ("new_thing" → "New
/// thing").
func label(_ slug: String) -> String {
    if let known = vocabularyLabels[slug] { return known }
    let words = slug.replacingOccurrences(of: "_", with: " ")
    return words.prefix(1).uppercased() + words.dropFirst()
}

/// How a product shows in the catalog (as Android's `productTitle`): its
/// strains as the title, its brand in the corner, and below its type and
/// own name. With no strain, its own name is the title; with neither, its
/// brand, then its type.
struct ProductTitle: Equatable {
    let title: String
    let brand: String
    let detail: String
}

func productTitle(name: String, strains: [String], brand: String?, form: String?) -> ProductTitle {
    let ownName = name.trimmingCharacters(in: .whitespaces)
    let strainText = strains.filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty }.joined(separator: " / ")
    let b = brand?.trimmingCharacters(in: .whitespaces) ?? ""
    let f = form?.trimmingCharacters(in: .whitespaces) ?? ""
    let title = [strainText, ownName, b, f].first { !$0.isEmpty } ?? ""
    var detail: [String] = []
    for part in [f, ownName] where !part.isEmpty && part != title && !detail.contains(part) {
        detail.append(part)
    }
    return ProductTitle(title: title, brand: b == title ? "" : b, detail: detail.joined(separator: " · "))
}

/// How many digits a currency's minor unit has (2 for USD, 0 for JPY).
private func minorDigits(_ currencyCode: String) -> Int {
    let formatter = NumberFormatter()
    formatter.numberStyle = .currency
    formatter.currencyCode = currencyCode
    return max(0, min(3, formatter.maximumFractionDigits))
}

/// A price in minor units, formatted in its currency (as Android's
/// `formatPrice`).
func formatPrice(_ minorUnits: Int64, currencyCode: String?) -> String {
    let digits = currencyCode.map(minorDigits) ?? 2
    let amount = Decimal(minorUnits) / pow(Decimal(10), digits)
    guard let currencyCode else { return "\(amount)" }
    return amount.formatted(.currency(code: currencyCode))
}

/// Minor units from typed text in `currencyCode`, or nil if it isn't an
/// amount with at most that currency's decimals (as Android's `parsePrice`).
func parsePrice(_ text: String, currencyCode: String) -> Int64? {
    let cleaned = text.trimmingCharacters(in: .whitespaces).replacingOccurrences(of: ",", with: ".")
    guard !cleaned.isEmpty, cleaned.allSatisfy({ $0.isNumber || $0 == "." }),
          cleaned.filter({ $0 == "." }).count <= 1,
          let amount = Decimal(string: cleaned, locale: Locale(identifier: "en_US_POSIX"))
    else { return nil }
    let digits = minorDigits(currencyCode)
    var minor = amount * pow(Decimal(10), digits)
    var rounded = Decimal()
    NSDecimalRound(&rounded, &minor, 0, .plain)
    guard rounded == minor else { return nil }
    return NSDecimalNumber(decimal: rounded).int64Value
}

/// The currency of this phone's region, as Android's `localCurrency`.
var localCurrency: String { Locale.current.currency?.identifier ?? "USD" }
