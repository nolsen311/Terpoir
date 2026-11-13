// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation

/// A vocabulary slug as words ("vape_cart" → "Vape cart"). Android has a
/// written label for each; until iOS has the same table, the slug reads
/// well enough.
func label(_ slug: String) -> String {
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
