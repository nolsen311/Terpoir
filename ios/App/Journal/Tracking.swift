// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI

/// A data point the user can hide in "What I track" (as Android's
/// `DataPoint`). The slugs are the core's (`trackable_data_points`). When,
/// product, method, effects and rating are always tracked, so they are not
/// here.
enum DataPoint: String, CaseIterable, Identifiable {
    case brand, strain
    case priceAndShop = "price_and_shop"
    case purchases, potency, terpenes, batch, dose, timeline, mood, intention, social

    var id: Self { self }

    var title: String {
        switch self {
        case .brand: return "Brands"
        case .strain: return "Strains"
        case .priceAndShop: return "Prices and shops"
        case .purchases: return "Purchases"
        case .potency: return "THC and CBD"
        case .terpenes: return "Terpenes"
        case .batch: return "Batch and package date"
        case .dose: return "Dose"
        case .timeline: return "Timeline"
        case .mood: return "Mood"
        case .intention: return "Intention"
        case .social: return "Solo or with others"
        }
    }

    var does: String {
        switch self {
        case .brand: return "Who made a product. Products show without a brand when this is off."
        case .strain: return "Strains and their types (indica, sativa, hybrid)."
        case .priceAndShop: return "What you paid and where. Receipts then record only what you bought and when."
        case .purchases: return "Receipts and purchase history. Turning this off also turns off prices and shops."
        case .potency: return "Amounts from the label."
        case .terpenes: return "Terpene amounts from the label, and the terpene insights built on them."
        case .batch: return "Batch or lot numbers and package dates, and freshness built on them."
        case .dose: return "How much you had."
        case .timeline: return "How strong it felt over time."
        case .mood: return "How you felt before and after."
        case .intention: return "Why you had a session."
        case .social: return "Who you were with, roughly."
        }
    }
}

/// The journal's strain detail step (as Android's `StrainDetail`); slugs
/// are the core's (`vocabulary().strainDetails`).
enum StrainDetail: String, CaseIterable, Identifiable {
    case simple, terpenes, detailed

    var id: Self { self }

    /// The step for a core slug; one from a newer app shows everything.
    static func of(_ slug: String) -> StrainDetail { StrainDetail(rawValue: slug) ?? .detailed }

    var title: String {
        switch self {
        case .simple: return "Simple"
        case .terpenes: return "Terpenes"
        case .detailed: return "Detailed"
        }
    }

    var does: String {
        switch self {
        case .simple: return "Indica, sativa or hybrid."
        case .terpenes: return "Also the terpenes it's known for."
        case .detailed: return "Also how much of each terpene, parent strains, and cannabinoids like CBN or CBG."
        }
    }
}

/// Which data points the journal shows. Editors leave hidden fields out but
/// carry their saved values unchanged. Hiding purchases also hides prices
/// and shops (the core reports both).
struct Tracking: Equatable {
    var hidden: Set<String> = []
    var strainDetail = StrainDetail.terpenes

    func shows(_ point: DataPoint) -> Bool { !hidden.contains(point.rawValue) }

    /// Everything shown: the default, and before the journal answers.
    static let everything = Tracking()
}
