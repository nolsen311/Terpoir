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

    /// Custom tags of one category (`terpene`, `effect`...), by name (as
    /// Android's `tags`).
    func tags(category: String) -> [Entry<TagData>] {
        all(.tag) { try self.journal.getTag(id: $0) }
            .filter { $0.data.category == category }
            .sorted { $0.data.name.localizedCaseInsensitiveCompare($1.data.name) == .orderedAscending }
    }

    func session(_ id: String) -> SessionData? { try? journal.getSession(id: id) }
    func product(_ id: String) -> ProductData? { try? journal.getProduct(id: id) }
    func strain(_ id: String) -> StrainData? { try? journal.getStrain(id: id) }
    func brand(_ id: String) -> BrandData? { try? journal.getBrand(id: id) }

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

    @discardableResult
    func saveTag(id: String?, _ data: TagData) throws -> String {
        defer { revision += 1 }
        return try journal.saveTag(id: id, tag: data, nowMs: now)
    }

    @discardableResult
    func saveStrain(id: String?, _ data: StrainData) throws -> String {
        defer { revision += 1 }
        return try journal.saveStrain(id: id, strain: data, nowMs: now)
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

    /// The whole journal as an encrypted export; only the recovery key
    /// opens it.
    func exportArchive(masterWrappedByRecovery: Data) throws -> Data {
        try journal.exportArchive(masterWrappedByRecovery: masterWrappedByRecovery)
    }

    /// The whole journal as readable files (`terpoir.json` and CSVs) in
    /// one ZIP, made by the core, the same file Android saves.
    /// Unencrypted: offered only behind a warning.
    func exportPlaintextZip() throws -> Data {
        try journal.exportPlaintextZip(nowMs: now)
    }

    /// An export file was from a newer app version; nothing was imported.
    struct NewerAppNeeded: Error {}

    /// Merges an export of this same journal into it (as on Android);
    /// returns how many records were added or updated.
    func importArchive(_ bytes: Data) throws -> UInt64 {
        let archive = try readExportArchive(archive: bytes)
        // Checked before anything changes, so a file from a newer app is
        // refused whole rather than imported partly.
        if try journal.checkExportArchive(archive: archive).newerFormatBlobs > 0 { throw NewerAppNeeded() }
        defer { revision += 1 }
        return try archive.blobs.reduce(0) { $0 + (try journal.importSyncBlob(bytes: $1, nowMs: now)).records }
    }

    /// Purchases that include `product`, newest first (as Android's
    /// `purchasesOf`).
    func purchases(of product: String) -> [Entry<PurchaseData>] {
        all(.purchase) { try self.journal.getPurchase(id: $0) }
            .filter { $0.data.lines.contains { $0.product == product } }
            .sorted { $0.data.when.unixMs > $1.data.when.unixMs }
    }

    @discardableResult
    func savePurchase(id: String?, _ data: PurchaseData) throws -> String {
        defer { revision += 1 }
        return try journal.savePurchase(id: id, purchase: data, nowMs: now)
    }

    /// By name.
    func spots() -> [Entry<SpotData>] {
        all(.spot) { try self.journal.getSpot(id: $0) }
            .sorted { $0.data.name.localizedCaseInsensitiveCompare($1.data.name) == .orderedAscending }
    }

    /// A shop known only by its name: a spot with Android's "Re-up" purpose
    /// and no location (as a receipt's new shop is saved on Android).
    @discardableResult
    func saveShop(named name: String) throws -> String {
        defer { revision += 1 }
        let shop = SpotData(
            name: name, purposes: [.builtin(slug: "re_up")], latE7: nil, lonE7: nil,
            accuracyM: nil, radiusM: nil, purchasePrompts: false
        )
        return try journal.saveSpot(id: nil, spot: shop, nowMs: now)
    }

    func delete(_ id: String) throws {
        defer { revision += 1 }
        _ = try journal.deleteRecord(id: id)
    }

    /// "What I track": the data points the user has hidden, and the strain
    /// detail step.
    func tracking() -> Tracking {
        Tracking(
            hidden: Set((try? journal.hiddenDataPoints()) ?? []),
            strainDetail: StrainDetail.of((try? journal.strainDetail()) ?? StrainDetail.terpenes.rawValue)
        )
    }

    /// Shows or hides a data point; hiding keeps its values. The choice syncs.
    func setTracked(_ point: DataPoint, _ tracked: Bool) throws {
        defer { revision += 1 }
        try journal.setDataPointTracked(dataPoint: point.rawValue, tracked: tracked, nowMs: now)
    }

    /// Sets the strain detail step; lower steps hide fields, never delete them.
    func setStrainDetail(_ step: StrainDetail) throws {
        defer { revision += 1 }
        try journal.setStrainDetail(step: step.rawValue, nowMs: now)
    }

    /// Deletes a data point's values from every record; returns how many
    /// records changed.
    @discardableResult
    func deleteDataPoint(_ point: DataPoint) throws -> UInt32 {
        defer { revision += 1 }
        return try journal.deleteDataPoint(dataPoint: point.rawValue, nowMs: now)
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
