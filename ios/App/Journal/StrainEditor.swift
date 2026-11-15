// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// What the strain editor shows: Android's default "Terpenes" step of
/// strain detail (type, terpenes, lineage, breeder). The "Detailed" step's
/// fields (levels, parents, cannabinoids) come with "What I track"; until
/// then they're saved as they were, as Android saves fields a step hides.
struct StrainFields: Equatable {
    var name = ""
    var type: String?
    var terpenes: [TagRefData] = []
    var lineage = ""
    var breeder = ""

    init() {}

    init(_ strain: StrainData) {
        name = strain.name
        type = strain.strainType
        terpenes = strain.terpenes
        lineage = strain.lineage ?? ""
        breeder = strain.breeder ?? ""
    }

    var canSave: Bool { !name.trimmingCharacters(in: .whitespaces).isEmpty }

    /// The strain to save, keeping what the editor doesn't show from `kept`.
    func data(keeping kept: StrainData?) -> StrainData {
        func trimmed(_ text: String) -> String? {
            let t = text.trimmingCharacters(in: .whitespaces)
            return t.isEmpty ? nil : t
        }
        return StrainData(
            name: name.trimmingCharacters(in: .whitespaces),
            lineage: trimmed(lineage),
            terpenes: terpenes,
            strainType: type,
            // The core drops levels for terpenes no longer listed.
            terpeneLevels: kept?.terpeneLevels ?? [],
            parents: kept?.parents ?? [],
            cannabinoids: kept?.cannabinoids ?? [],
            breeder: trimmed(breeder)
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
        let custom = repo.tags(category: "terpene")
        let customNames = Dictionary(uniqueKeysWithValues: custom.map { ($0.id, $0.data.name) })
        let offered = vocabulary().terpenes.map { TagRefData.builtin(slug: $0) } + custom.map { TagRefData.custom(id: $0.id) }
        // Terpenes saved earlier that aren't offered (a tag since deleted) still show.
        let terpeneOptions = offered + fields.terpenes.filter { !offered.contains($0) }
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
            Section {
                ForEach(terpeneOptions, id: \.self) { tag in
                    let on = fields.terpenes.contains(tag)
                    Button {
                        if on { fields.terpenes.removeAll { $0 == tag } } else { fields.terpenes.append(tag) }
                    } label: {
                        HStack {
                            Text(tagLabel(tag, customNames))
                            Spacer()
                            if on { Image(systemName: "checkmark").foregroundStyle(.tint) }
                        }
                    }
                    .foregroundStyle(.primary)
                    .accessibilityAddTraits(on ? .isSelected : [])
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
            Section {
                TextField("Lineage notes", text: $fields.lineage)
                TextField("Breeder", text: $fields.breeder)
                    .textInputAutocapitalization(.words)
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

    private func load() {
        guard !loaded else { return }
        loaded = true
        guard let id, let strain = repo.strain(id) else { return }
        kept = strain
        fields = StrainFields(strain)
    }

    private func save() {
        do {
            try repo.saveStrain(id: id, fields.data(keeping: kept))
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
