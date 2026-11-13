// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The journal (docs/roadmap.md, "iOS", I3): sessions and products, read and
// written through the core.

import SwiftUI
import TerpoirCore

struct JournalHome: View {
    @StateObject private var repo: JournalRepository
    let onLock: () -> Void

    init(journal: Journal, onLock: @escaping () -> Void) {
        _repo = StateObject(wrappedValue: JournalRepository(journal: journal))
        self.onLock = onLock
    }

    var body: some View {
        TabView {
            NavigationStack { SessionsList(repo: repo, onLock: onLock) }
                .tabItem { Label("Sessions", systemImage: "list.bullet") }
            NavigationStack { ProductsList(repo: repo, onLock: onLock) }
                .tabItem { Label("Products", systemImage: "leaf") }
        }
    }
}

/// The names products are shown with: brands and strains by ID.
struct CatalogNames {
    let brands: [String: String]
    let strains: [String: String]

    @MainActor
    init(_ repo: JournalRepository) {
        brands = Dictionary(uniqueKeysWithValues: repo.brands().map { ($0.id, $0.data.name) })
        strains = Dictionary(uniqueKeysWithValues: repo.strains().map { ($0.id, $0.data.name) })
    }

    func title(_ product: ProductData) -> ProductTitle {
        productTitle(
            name: product.name,
            strains: product.strains.compactMap { strains[$0] },
            brand: product.brand.flatMap { brands[$0] },
            form: product.form.map(label)
        )
    }
}

struct LockButton: View {
    let onLock: () -> Void
    var body: some View {
        Button(action: onLock) { Image(systemName: "lock") }
            .accessibilityLabel("Lock")
    }
}

// MARK: - Sessions

struct SessionsList: View {
    @ObservedObject var repo: JournalRepository
    let onLock: () -> Void
    @State private var editing: EditTarget?

    var body: some View {
        let _ = repo.revision
        let names = CatalogNames(repo)
        let products = Dictionary(uniqueKeysWithValues: repo.products().map { ($0.id, $0.data) })
        let sessions = repo.sessions()
        List {
            if sessions.isEmpty {
                Text("No sessions yet. Tap + to log one.").foregroundStyle(.secondary)
            }
            ForEach(sessions, id: \.id) { entry in
                Button { editing = EditTarget(id: entry.id) } label: {
                    SessionRow(session: entry.data, products: products, names: names)
                }
                .foregroundStyle(.primary)
            }
        }
        .navigationTitle("Sessions")
        .toolbar {
            ToolbarItem(placement: .topBarLeading) { LockButton(onLock: onLock) }
            ToolbarItem(placement: .topBarTrailing) {
                Button { editing = EditTarget(id: nil) } label: { Image(systemName: "plus") }
                    .accessibilityLabel("New session")
            }
        }
        .sheet(item: $editing) { target in
            NavigationStack { SessionEditor(repo: repo, id: target.id) }
        }
    }
}

struct SessionRow: View {
    let session: SessionData
    let products: [String: ProductData]
    let names: CatalogNames

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(Self.when(session.when)).font(.headline)
            let had = session.lines.compactMap { $0.product.flatMap { products[$0] }.map { names.title($0).title } }
            if !had.isEmpty {
                Text(had.joined(separator: ", ")).font(.subheadline)
            }
            if let rating = session.rating {
                Text(String(repeating: "★", count: Int(rating))).font(.caption).foregroundStyle(.secondary)
                    .accessibilityLabel("Rated \(rating) of 5")
            }
        }
    }

    static func when(_ time: LocalTimeData) -> String {
        let date = Date(timeIntervalSince1970: Double(time.unixMs) / 1000)
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        formatter.timeZone = TimeZone(secondsFromGMT: Int(time.utcOffsetMin) * 60)
        return formatter.string(from: date)
    }
}

// MARK: - Products

struct ProductsList: View {
    @ObservedObject var repo: JournalRepository
    let onLock: () -> Void
    @State private var editing: EditTarget?

    var body: some View {
        let _ = repo.revision
        let names = CatalogNames(repo)
        // The Products page's order: by card title (maintainer, 2026-11-13).
        let products = repo.products()
            .filter { !$0.data.archived }
            .map { ($0, names.title($0.data)) }
            .sorted { $0.1.title.localizedCaseInsensitiveCompare($1.1.title) == .orderedAscending }
        List {
            if products.isEmpty {
                Text("No products yet. Tap + to add one.").foregroundStyle(.secondary)
            }
            ForEach(products, id: \.0.id) { item in
                let (entry, title) = item
                Button { editing = EditTarget(id: entry.id) } label: {
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(alignment: .firstTextBaseline) {
                            Text(title.title).font(.headline)
                            Spacer()
                            if !title.brand.isEmpty {
                                Text(title.brand).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        if !title.detail.isEmpty {
                            Text(title.detail).font(.subheadline).foregroundStyle(.secondary)
                        }
                    }
                }
                .foregroundStyle(.primary)
            }
        }
        .navigationTitle("Products")
        .toolbar {
            ToolbarItem(placement: .topBarLeading) { LockButton(onLock: onLock) }
            ToolbarItem(placement: .topBarTrailing) {
                Button { editing = EditTarget(id: nil) } label: { Image(systemName: "plus") }
                    .accessibilityLabel("New product")
            }
        }
        .sheet(item: $editing) { target in
            NavigationStack { ProductEditor(repo: repo, id: target.id) }
        }
    }
}

/// What an editor sheet opens: a record by ID, or a new one (nil).
struct EditTarget: Identifiable {
    let id: String?
}
