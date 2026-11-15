// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// A session's targeted symptoms (docs/design-decisions.md, "Targeted
/// symptoms"), shown while the Privacy Center's toggle is on: what you
/// hoped it would help with, and whether it did. Self-tracking, never
/// medical advice.
struct SymptomsSection: View {
    @ObservedObject var repo: JournalRepository
    @Binding var symptoms: [TagRefData: String]
    @State private var newName = ""

    var body: some View {
        let custom = repo.tags(category: symptomCategory)
        let names = Dictionary(uniqueKeysWithValues: custom.map { ($0.id, $0.data.name) })
        let offered = vocabulary().symptoms.map { TagRefData.builtin(slug: $0) } + custom.map { TagRefData.custom(id: $0.id) }
        // Symptoms saved earlier that aren't offered (a tag since deleted) still show.
        let all = offered + symptoms.keys.filter { !offered.contains($0) }.sorted { tagLabel($0, names) < tagLabel($1, names) }
        Section {
            ForEach(all, id: \.self) { tag in
                Picker(tagLabel(tag, names), selection: Binding(
                    get: { symptoms[tag] },
                    set: { symptoms[tag] = $0 }
                )) {
                    Text("–").tag(String?.none)
                    ForEach(symptomResults) { result in
                        Text(result.label).tag(Optional(result.slug))
                    }
                }
            }
            HStack {
                TextField("New symptom", text: $newName)
                    .textInputAutocapitalization(.sentences)
                Button("Add", action: add)
                    .disabled(newName.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        } header: {
            Text("Targeted symptoms")
        } footer: {
            Text("What were you hoping this would help with? For each one, did it help? Not medical advice; for that, ask a healthcare professional.")
        }
    }

    private func add() {
        let name = newName.trimmingCharacters(in: .whitespaces)
        guard !name.isEmpty, let id = try? repo.saveTag(id: nil, TagData(name: name, category: symptomCategory)) else { return }
        symptoms[.custom(id: id)] = "not_rated"
        newName = ""
    }
}

let symptomCategory = "symptom"

/// A result a symptom can have, as Android shows it.
struct SymptomResult: Identifiable {
    let slug: String
    let label: String
    var id: String { slug }
}

let symptomResults = [
    SymptomResult(slug: "not_rated", label: "Not sure"),
    SymptomResult(slug: "helped", label: "👍 Helped"),
    SymptomResult(slug: "did_not_help", label: "👎 Didn't help"),
]
