// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// The strain editor's fields. Which ones show is the journal's strain
/// detail step ("What I track"); a field a step hides is saved as it was,
/// as on Android.
struct StrainFields: Equatable {
    var name = ""
    var type: String?
    var terpenes: [TagRefData] = []
    var levels: [TagRefData: String] = [:]
    var parents: [String] = []
    var cannabinoids: [String] = []
    var lineage = ""
    var breeder = ""

    init() {}

    init(_ strain: StrainData) {
        name = strain.name
        type = strain.strainType
        terpenes = strain.terpenes
        levels = Dictionary(strain.terpeneLevels.map { ($0.terpene, $0.level) }, uniquingKeysWith: { first, _ in first })
        parents = strain.parents
        cannabinoids = strain.cannabinoids
        lineage = strain.lineage ?? ""
        breeder = strain.breeder ?? ""
    }

    var canSave: Bool { !name.trimmingCharacters(in: .whitespaces).isEmpty }

    /// Whether the terpene fields show: a step above Simple, with terpenes tracked.
    static func showsTerpenes(_ tracking: Tracking) -> Bool {
        tracking.shows(.terpenes) && tracking.strainDetail != .simple
    }

    /// The strain to save: what `tracking` shows from these fields, the rest
    /// from `kept`.
    func data(keeping kept: StrainData?, tracking: Tracking) -> StrainData {
        func trimmed(_ text: String) -> String? {
            let t = text.trimmingCharacters(in: .whitespaces)
            return t.isEmpty ? nil : t
        }
        let step = tracking.strainDetail
        let simple = step == .simple
        let detailed = step == .detailed
        let terpenesShown = Self.showsTerpenes(tracking)
        return StrainData(
            name: name.trimmingCharacters(in: .whitespaces),
            lineage: simple ? kept?.lineage : trimmed(lineage),
            terpenes: terpenesShown ? terpenes : kept?.terpenes ?? [],
            strainType: type,
            // The core drops levels for terpenes no longer listed.
            terpeneLevels: terpenesShown && detailed
                ? terpenes.compactMap { t in levels[t].map { TerpeneLevelData(terpene: t, level: $0) } }
                : kept?.terpeneLevels ?? [],
            parents: detailed ? parents : kept?.parents ?? [],
            cannabinoids: detailed ? cannabinoids : kept?.cannabinoids ?? [],
            breeder: simple ? kept?.breeder : trimmed(breeder)
        )
    }
}

struct StrainEditor: View {
    @ObservedObject var repo: JournalRepository
    let id: String?
    @Environment(\.dismiss) private var dismiss

    @State private var kept: StrainData?
    @State private var fields = StrainFields()
    @State private var loaded = false
    @State private var error: String?
    @State private var confirmDelete = false

    var body: some View {
        let tracking = repo.tracking()
        Form {
            TextField("Name", text: $fields.name)
                .textInputAutocapitalization(.words)
            Section {
                Picker("Type on the label", selection: $fields.type) {
                    Text("None").tag(String?.none)
                    ForEach(vocabulary().strainTypes, id: \.self) { Text(label($0)).tag(Optional($0)) }
                }
            } footer: {
                Text("Indica and sativa labels are a rough guide; Terpoir's insights look at terpenes and cannabinoids instead.")
            }
            if StrainFields.showsTerpenes(tracking) {
                terpeneSections(detailed: tracking.strainDetail == .detailed)
            }
            if tracking.strainDetail == .detailed {
                detailedSections
            }
            if tracking.strainDetail != .simple {
                Section {
                    TextField("Lineage notes", text: $fields.lineage)
                    TextField("Breeder", text: $fields.breeder)
                        .textInputAutocapitalization(.words)
                }
            }
            if let error {
                Text(error).foregroundStyle(.red)
            }
            if id != nil {
                Button("Delete strain", role: .destructive) { confirmDelete = true }
            }
        }
        .navigationTitle(id == nil ? "New strain" : "Edit strain")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Save", action: save).disabled(!fields.canSave) }
        }
        .confirmationDialog("Delete this strain?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) {
                if let id { try? repo.delete(id) }
                dismiss()
            }
        } message: {
            Text("Its key is destroyed, so it can't be recovered. Products that listed it stay.")
        }
        .onAppear(perform: load)
    }

    @MainActor @ViewBuilder
    private func terpeneSections(detailed: Bool) -> some View {
        let custom = repo.tags(category: "terpene")
        let customNames = Dictionary(uniqueKeysWithValues: custom.map { ($0.id, $0.data.name) })
        let offered = vocabulary().terpenes.map { TagRefData.builtin(slug: $0) } + custom.map { TagRefData.custom(id: $0.id) }
        // Terpenes saved earlier that aren't offered (a tag since deleted) still show.
        let options = offered + fields.terpenes.filter { !offered.contains($0) }
        Section {
            ForEach(options, id: \.self) { tag in
                CheckRow(title: tagLabel(tag, customNames), on: fields.terpenes.contains(tag)) {
                    fields.terpenes.toggle(tag)
                }
            }
        } header: {
            Text("Terpenes it's known for")
        } footer: {
            if fields.terpenes.count > 3 {
                Text("That's more than most strains are known for. Keep them if they all stand out; levels below can say which lead.")
            } else {
                Text("Pick the ones that stand out. Most strains are known for one to three.")
            }
        }
        if detailed && !fields.terpenes.isEmpty {
            Section {
                ForEach(fields.terpenes, id: \.self) { tag in
                    Picker(tagLabel(tag, customNames), selection: Binding(
                        get: { fields.levels[tag] },
                        set: { fields.levels[tag] = $0 }
                    )) {
                        Text("Unrated").tag(String?.none)
                        ForEach(vocabulary().terpeneLevels, id: \.self) { Text(label($0)).tag(Optional($0)) }
                    }
                }
            } header: {
                Text("How much of each")
            } footer: {
                Text("Rough levels, not percentages: those belong on products, from their labels. Leave any unrated.")
            }
        }
    }

    @MainActor @ViewBuilder
    private var detailedSections: some View {
        let others = repo.strains().filter { $0.id != id }
        let names = Dictionary(uniqueKeysWithValues: others.map { ($0.id, $0.data.name) })
        Section("Parent strains") {
            ForEach(fields.parents, id: \.self) { parent in
                Text(names[parent] ?? "Deleted strain")
            }
            .onDelete { fields.parents.remove(atOffsets: $0) }
            Menu("Add a parent from your journal") {
                ForEach(others.filter { !fields.parents.contains($0.id) }, id: \.id) { strain in
                    Button(strain.data.name) { fields.parents.append(strain.id) }
                }
            }
            .disabled(others.allSatisfy { fields.parents.contains($0.id) })
        }
        Section {
            ForEach(vocabulary().cannabinoids + fields.cannabinoids.filter { !vocabulary().cannabinoids.contains($0) }, id: \.self) { c in
                CheckRow(title: label(c), on: fields.cannabinoids.contains(c)) { fields.cannabinoids.toggle(c) }
            }
        } header: {
            Text("Also known for")
        } footer: {
            Text("Cannabinoids besides THC and CBD. Amounts belong on products.")
        }
    }

    private func load() {
        guard !loaded else { return }
        loaded = true
        guard let id, let strain = repo.strain(id) else { return }
        kept = strain
        fields = StrainFields(strain)
    }

    private func save() {
        do {
            try repo.saveStrain(id: id, fields.data(keeping: kept, tracking: repo.tracking()))
            dismiss()
        } catch {
            self.error = "The strain couldn't be saved. Try again."
        }
    }
}

/// A terpene's name: built-in ones from the vocabulary's labels, custom
/// ones by their tag's name.
func tagLabel(_ tag: TagRefData, _ customNames: [String: String]) -> String {
    switch tag {
    case let .builtin(slug): return label(slug)
    case let .custom(id): return customNames[id] ?? "Deleted tag"
    }
}

struct BrandEditor: View {
    @ObservedObject var repo: JournalRepository
    let id: String?
    @Environment(\.dismiss) private var dismiss

    @State private var name = ""
    @State private var loaded = false
    @State private var error: String?
    @State private var confirmDelete = false

    var body: some View {
        Form {
            TextField("Name", text: $name)
                .textInputAutocapitalization(.words)
            if let error {
                Text(error).foregroundStyle(.red)
            }
            if id != nil {
                Button("Delete brand", role: .destructive) { confirmDelete = true }
            }
        }
        .navigationTitle(id == nil ? "New brand" : "Edit brand")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) {
                Button("Save", action: save).disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .confirmationDialog("Delete this brand?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) {
                if let id { try? repo.delete(id) }
                dismiss()
            }
        } message: {
            Text("Its key is destroyed, so it can't be recovered. Its products stay.")
        }
        .onAppear {
            guard !loaded else { return }
            loaded = true
            if let id, let brand = repo.brand(id) { name = brand.name }
        }
    }

    private func save() {
        do {
            try repo.saveBrand(id: id, BrandData(name: name.trimmingCharacters(in: .whitespaces)))
            dismiss()
        } catch {
            self.error = "The brand couldn't be saved. Try again."
        }
    }
}

/// A row that is ticked or not, as Android's filter chips.
struct CheckRow: View {
    let title: String
    let on: Bool
    let tap: () -> Void

    var body: some View {
        Button(action: tap) {
            HStack {
                Text(title)
                Spacer()
                if on { Image(systemName: "checkmark").foregroundStyle(.tint) }
            }
        }
        .foregroundStyle(.primary)
        .accessibilityAddTraits(on ? .isSelected : [])
    }
}

extension Array where Element: Equatable {
    /// Adds `element`, or removes it if it's there.
    mutating func toggle(_ element: Element) {
        if contains(element) { removeAll { $0 == element } } else { append(element) }
    }
}
