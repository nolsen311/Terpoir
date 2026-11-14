// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// A product: its name (optional, as on Android), type, brand and strains.
/// Fields it doesn't show yet (potency, terpenes, sizes...) are kept as
/// they are.
struct ProductEditor: View {
    @ObservedObject var repo: JournalRepository
    let id: String?
    @Environment(\.dismiss) private var dismiss

    @State private var loaded: ProductData?
    @State private var name = ""
    @State private var form = ""
    @State private var brand = ""
    @State private var strains = ""
    @State private var error: String?
    @State private var confirmDelete = false

    var body: some View {
        Form {
            TextField("Name (optional)", text: $name)
            Picker("Type", selection: $form) {
                Text("None").tag("")
                ForEach(vocabulary().forms, id: \.self) { Text(label($0)).tag($0) }
            }
            TextField("Brand", text: $brand)
                .textInputAutocapitalization(.words)
            TextField("Strains, separated by commas", text: $strains)
                .textInputAutocapitalization(.words)
            if let error {
                Text(error).foregroundStyle(.red)
            }
            if id != nil {
                Button("Delete product", role: .destructive) { confirmDelete = true }
            }
        }
        .navigationTitle(id == nil ? "New product" : "Product")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Save", action: save) }
        }
        .confirmationDialog("Delete this product?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) {
                if let id { try? repo.delete(id) }
                dismiss()
            }
        }
        .onAppear(perform: load)
    }

    private func load() {
        guard let id, loaded == nil, let product = repo.product(id) else { return }
        loaded = product
        name = product.name
        form = product.form ?? ""
        brand = product.brand.flatMap { brandId in repo.brands().first { $0.id == brandId }?.data.name } ?? ""
        let strainNames = Dictionary(uniqueKeysWithValues: repo.strains().map { ($0.id, $0.data.name) })
        strains = product.strains.compactMap { strainNames[$0] }.joined(separator: ", ")
    }

    private func save() {
        let brandName = brand.trimmingCharacters(in: .whitespaces)
        let strainNames = strains.split(separator: ",").map { $0.trimmingCharacters(in: .whitespaces) }.filter { !$0.isEmpty }
        // As on Android: a product needs a name, a strain, a brand or a type.
        guard !name.trimmingCharacters(in: .whitespaces).isEmpty || !strainNames.isEmpty || !brandName.isEmpty || !form.isEmpty else {
            error = "Give it a name, a strain, a brand or a type."
            return
        }
        do {
            let brandId: String? = brandName.isEmpty ? nil
                : try repo.brands().first { $0.data.name.caseInsensitiveCompare(brandName) == .orderedSame }?.id
                    ?? repo.saveBrand(id: nil, BrandData(name: brandName))
            var product = loaded ?? ProductData(
                name: "", form: nil, brand: nil, strains: [], thcMilli: nil, thcUnit: nil, cbdMilli: nil, cbdUnit: nil,
                terpenes: [], batch: nil, packageDateDay: nil, identifiers: [], shops: []
            )
            product.name = name.trimmingCharacters(in: .whitespaces)
            product.form = form.isEmpty ? nil : form
            product.brand = brandId
            product.strains = try repo.strainIds(named: strainNames)
            try repo.saveProduct(id: id, product)
            dismiss()
        } catch {
            self.error = "The product couldn't be saved. Try again."
        }
    }
}
