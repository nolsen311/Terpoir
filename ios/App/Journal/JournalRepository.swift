// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation
import TerpoirCore

/// A record and its ID.
struct Entry<T> {
    let id: String
    let data: T
}

/// The open journal, for the screens (as Android's `JournalRepository`).
/// Every read and write goes to the core; this only lists and sorts.
@MainActor
final class JournalRepository: ObservableObject {
    private let journal: Journal

    /// Bumped after every write, so screens read again.
    @Published private(set) var revision = 0

    init(journal: Journal) {
        self.journal = journal
    }

    private var now: UInt64 { UInt64(Date().timeIntervalSince1970 * 1000) }

    private func all<T>(_ type: RecordType, _ get: (String) throws -> T?) -> [Entry<T>] {
        ((try? journal.list(recordType: type)) ?? []).compactMap { id in
            guard let data = try? get(id) else { return nil }
            return Entry(id: id, data: data)
        }
    }

    /// Newest first.
    func sessions() -> [Entry<SessionData>] {
        all(.session) { try self.journal.getSession(id: $0) }.sorted { $0.data.when.unixMs > $1.data.when.unixMs }
    }

    func products() -> [Entry<ProductData>] { all(.product) { try self.journal.getProduct(id: $0) } }

    /// By name.
    func brands() -> [Entry<BrandData>] {
        all(.brand) { try self.journal.getBrand(id: $0) }.sorted { $0.data.name.localizedCaseInsensitiveCompare($1.data.name) == .orderedAscending }
    }

    /// By name.
    func strains() -> [Entry<StrainData>] {
        all(.strain) { try self.journal.getStrain(id: $0) }.sorted { $0.data.name.localizedCaseInsensitiveCompare($1.data.name) == .orderedAscending }
    }

    func session(_ id: String) -> SessionData? { try? journal.getSession(id: id) }
    func product(_ id: String) -> ProductData? { try? journal.getProduct(id: id) }

    @discardableResult
    func saveSession(id: String?, _ data: SessionData) throws -> String {
        defer { revision += 1 }
        return try journal.saveSession(id: id, session: data, nowMs: now)
    }

    @discardableResult
    func saveProduct(id: String?, _ data: ProductData) throws -> String {
        defer { revision += 1 }
        return try journal.saveProduct(id: id, product: data, nowMs: now)
    }

    @discardableResult
    func saveBrand(id: String?, _ data: BrandData) throws -> String {
        defer { revision += 1 }
        return try journal.saveBrand(id: id, brand: data, nowMs: now)
    }

    /// Strain IDs for `names`: existing strains by the core's name matching,
    /// new ones saved.
    func strainIds(named names: [String]) throws -> [String] {
        let found = try journal.strainIdsNamed(names: names)
        defer { revision += 1 }
        return try zip(names, found).map { name, id in
            id.isEmpty ? try journal.saveStrain(id: nil, strain: StrainData.named(name), nowMs: now) : id
        }
    }

    func delete(_ id: String) throws {
        defer { revision += 1 }
        _ = try journal.deleteRecord(id: id)
    }

    /// The method usually used for a product of this form (the core's rule).
    func usualMethod(form: String?) -> String? {
        try? journal.usualMethod(form: form, subtype: nil)
    }
}

extension StrainData {
    /// A strain known only by its name.
    static func named(_ name: String) -> StrainData {
        StrainData(name: name, lineage: nil, terpenes: [], strainType: nil, terpeneLevels: [], parents: [], cannabinoids: [])
    }
}
