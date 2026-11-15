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
    let masterWrappedByRecovery: @MainActor () throws -> Data

    init(journal: Journal, onLock: @escaping () -> Void, masterWrappedByRecovery: @escaping @MainActor () throws -> Data) {
        _repo = StateObject(wrappedValue: JournalRepository(journal: journal))
        self.onLock = onLock
        self.masterWrappedByRecovery = masterWrappedByRecovery
    }

    var body: some View {
        TabView {
            NavigationStack { SessionsList(repo: repo, onLock: onLock) }
                .tabItem { Label("Sessions", systemImage: "list.bullet") }
            NavigationStack { ProductsList(repo: repo, onLock: onLock) }
                .tabItem { Label("Products", systemImage: "leaf") }
            NavigationStack { DataView(repo: repo, masterWrappedByRecovery: masterWrappedByRecovery) }
                .tabItem { Label("Your data", systemImage: "externaldrive") }
        }
    }
}

/// The names products are shown with: brands and strains by ID.
struct CatalogNames {
    let brands: [String: String]
    let strains: [String: String]

    /// Hidden brands or strains are left out of titles (as on Android).
    @MainActor
    init(_ repo: JournalRepository) {
        let tracking = repo.tracking()
        brands = tracking.shows(.brand) ? Dictionary(uniqueKeysWithValues: repo.brands().map { ($0.id, $0.data.name) }) : [:]
        strains = tracking.shows(.strain) ? Dictionary(uniqueKeysWithValues: repo.strains().map { ($0.id, $0.data.name) }) : [:]
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
        // One timeline, as on Android: sessions and standalone notes, newest first.
        let timeline: [TimelineItem] = (
            repo.sessions().map { TimelineItem.session($0) } + repo.standaloneNotes().map { TimelineItem.note($0) }
        ).sorted { $0.unixMs > $1.unixMs }
        List {
            if timeline.isEmpty {
                Text("Nothing yet. Tap + to log a session or write a note.").foregroundStyle(.secondary)
            }
            ForEach(timeline) { item in
                switch item {
                case let .session(entry):
                    Button { editing = EditTarget(id: entry.id) } label: {
                        SessionRow(session: entry.data, products: products, names: names)
                    }
                    .foregroundStyle(.primary)
                case let .note(entry):
                    Button { editing = EditTarget(id: entry.id, note: true) } label: {
                        NoteRow(note: entry.data)
                    }
                    .foregroundStyle(.primary)
                }
            }
        }
        .navigationTitle("Sessions")
        .toolbar {
            ToolbarItem(placement: .topBarLeading) { LockButton(onLock: onLock) }
            ToolbarItem(placement: .topBarTrailing) {
                Menu {
                    Button("New session") { editing = EditTarget(id: nil) }
                    Button("New note") { editing = EditTarget(id: nil, note: true) }
                } label: {
                    Image(systemName: "plus")
                }
                .accessibilityLabel("New entry")
            }
        }
        .sheet(item: $editing) { target in
            NavigationStack {
                if target.note {
                    NoteEditor(repo: repo, id: target.id)
                } else {
                    SessionEditor(repo: repo, id: target.id)
                }
            }
        }
    }
}

/// A session or a standalone note in the journal's timeline.
enum TimelineItem: Identifiable {
    case session(Entry<SessionData>)
    case note(Entry<SnippetData>)

    var id: String {
        switch self {
        case let .session(entry): return entry.id
        case let .note(entry): return entry.id
        }
    }

    var unixMs: Int64 {
        switch self {
        case let .session(entry): return entry.data.when.unixMs
        case let .note(entry): return entry.data.at.unixMs
        }
    }
}

struct NoteRow: View {
    let note: SnippetData

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(SessionRow.when(note.at)).font(.headline)
            Text(note.text).font(.subheadline).lineLimit(2)
        }
        .accessibilityLabel("Note, \(SessionRow.when(note.at)): \(note.text)")
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

/// The Products tab's lists, as Android's catalog switcher (Spots come later).
enum Catalog: String, CaseIterable, Identifiable {
    case products = "Products", strains = "Strains", brands = "Brands"
    var id: Self { self }
}

struct ProductsList: View {
    @ObservedObject var repo: JournalRepository
    let onLock: () -> Void
    @State private var catalog = Catalog.products
    @State private var editing: EditTarget?

    var body: some View {
        let _ = repo.revision
        List {
            Picker("Show", selection: $catalog) {
                ForEach(shownCatalogs) { Text($0.rawValue).tag($0) }
            }
            .pickerStyle(.segmented)
            .listRowBackground(Color.clear)
            .listRowInsets(EdgeInsets())
            switch catalog {
            case .products: productRows
            case .strains: strainRows
            case .brands: brandRows
            }
        }
        .navigationTitle(catalog.rawValue)
        // A list whose data point was just hidden goes back to Products.
        .onChange(of: shownCatalogs) { _, shown in
            if !shown.contains(catalog) { catalog = .products }
        }
        .toolbar {
            ToolbarItem(placement: .topBarLeading) { LockButton(onLock: onLock) }
            ToolbarItem(placement: .topBarTrailing) {
                Button { editing = EditTarget(id: nil, catalog: catalog) } label: { Image(systemName: "plus") }
                    .accessibilityLabel(newLabel)
            }
        }
        .sheet(item: $editing) { target in
            NavigationStack {
                switch target.catalog {
                case .products: ProductEditor(repo: repo, id: target.id)
                case .strains: StrainEditor(repo: repo, id: target.id)
                case .brands: BrandEditor(repo: repo, id: target.id)
                }
            }
        }
    }

    /// "What I track": hidden strains or brands lose their lists.
    @MainActor private var shownCatalogs: [Catalog] {
        let tracking = repo.tracking()
        return Catalog.allCases.filter {
            switch $0 {
            case .products: return true
            case .strains: return tracking.shows(.strain)
            case .brands: return tracking.shows(.brand)
            }
        }
    }

    private var newLabel: String {
        switch catalog {
        case .products: return "New product"
        case .strains: return "New strain"
        case .brands: return "New brand"
        }
    }

    @MainActor @ViewBuilder private var productRows: some View {
        let names = CatalogNames(repo)
        // The Products page's order: by card title (maintainer, 2026-11-13).
        let products = repo.products()
            .filter { !$0.data.archived }
            .map { ($0, names.title($0.data)) }
            .sorted { $0.1.title.localizedCaseInsensitiveCompare($1.1.title) == .orderedAscending }
        if products.isEmpty {
            Text("No products yet. Tap + to add one.").foregroundStyle(.secondary)
        }
        ForEach(products, id: \.0.id) { item in
            let (entry, title) = item
            Button { editing = EditTarget(id: entry.id, catalog: .products) } label: {
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

    @MainActor @ViewBuilder private var strainRows: some View {
        let strains = repo.strains()
        if strains.isEmpty {
            Text("No strains yet. Tap + to add one.").foregroundStyle(.secondary)
        }
        ForEach(strains, id: \.id) { strain in
            CatalogRow(name: strain.data.name, detail: strain.data.strainType.map(label) ?? "") {
                editing = EditTarget(id: strain.id, catalog: .strains)
            }
        }
    }

    @MainActor @ViewBuilder private var brandRows: some View {
        let brands = repo.brands()
        if brands.isEmpty {
            Text("No brands yet. Tap + to add one.").foregroundStyle(.secondary)
        }
        ForEach(brands, id: \.id) { brand in
            CatalogRow(name: brand.data.name, detail: "") {
                editing = EditTarget(id: brand.id, catalog: .brands)
            }
        }
    }
}

/// A strain or brand in its list: the name, and a detail on the right.
struct CatalogRow: View {
    let name: String
    let detail: String
    let open: () -> Void

    var body: some View {
        Button(action: open) {
            HStack {
                Text(name)
                Spacer()
                if !detail.isEmpty { Text(detail).foregroundStyle(.secondary) }
            }
        }
        .foregroundStyle(.primary)
    }
}

/// What an editor sheet opens: a record by ID, or a new one (nil).
struct EditTarget: Identifiable {
    let id: String?
    var catalog = Catalog.products
    /// Opens a note rather than a session.
    var note = false
}
