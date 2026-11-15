// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// A note in your own words (as Android's note screen, typed): saved as a
/// standalone stream snippet. Keyword tag suggestions, matched on the
/// device by the core, appear as the text changes, each quoting the words
/// it came from; only the tags the user accepts or adds are saved, with
/// where they came from (docs/design-decisions.md, "Tag extraction").
/// Voice notes come later.
struct NoteEditor: View {
    @ObservedObject var repo: JournalRepository
    let id: String?
    @Environment(\.dismiss) private var dismiss

    @State private var loaded: SnippetData?
    @State private var text = ""
    @State private var tags: [TagProvenance] = []
    @State private var rejected: Set<TagRefData> = []
    @State private var suggestions: [TagSuggestion] = []
    @State private var picking = false
    @State private var confirmDelete = false
    @State private var error: String?

    var body: some View {
        let custom = noteTagCategories.flatMap { repo.tags(category: $0) }
        let names = Dictionary(uniqueKeysWithValues: custom.map { ($0.id, $0.data.name) })
        Form {
            Section("Note") {
                TextEditor(text: $text)
                    .frame(minHeight: 160)
            }
            Section("Tags") {
                ForEach(tags, id: \.tag) { tag in
                    HStack {
                        Text(tagLabel(tag.tag, names))
                        Spacer()
                        Button {
                            tags.removeAll { $0.tag == tag.tag }
                        } label: {
                            Image(systemName: "xmark.circle.fill").foregroundStyle(.secondary)
                        }
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Remove \(tagLabel(tag.tag, names))")
                    }
                }
                Button("Add a tag") { picking = true }
            }
            if !suggestions.isEmpty {
                Section("Suggested from your words: check before adding") {
                    ForEach(suggestions, id: \.tag) { suggestion in
                        VStack(alignment: .leading, spacing: 4) {
                            HStack {
                                Text(tagLabel(suggestion.tag, names)).font(.headline)
                                Spacer()
                                Button("Add") { tags.append(TagProvenance(tag: suggestion.tag, provenance: "keyword")) }
                                    .buttonStyle(.borderless)
                                Button("No") { rejected.insert(suggestion.tag) }
                                    .buttonStyle(.borderless)
                            }
                            Text("“\(phrase(of: suggestion, in: text))”")
                                .font(.footnote).foregroundStyle(.secondary)
                        }
                    }
                }
            }
            if let error {
                Text(error).foregroundStyle(.red)
            }
            if id != nil {
                Button("Delete note", role: .destructive) { confirmDelete = true }
            }
        }
        .navigationTitle(id == nil ? "New note" : "Edit note")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) {
                Button("Save", action: save).disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
        }
        // Suggestions follow the text, after a short pause in typing.
        .task(id: SuggestKey(text: text, tags: tags.map(\.tag), rejected: rejected)) {
            try? await Task.sleep(for: .milliseconds(400))
            guard !Task.isCancelled else { return }
            let have = Set(tags.map(\.tag)).union(rejected)
            suggestions = text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                ? [] : repo.suggestTags(text).filter { !have.contains($0.tag) }
        }
        .sheet(isPresented: $picking) {
            NavigationStack {
                TagPicker(chosen: Set(tags.map(\.tag)), custom: custom) { tag in
                    tags.append(TagProvenance(tag: tag, provenance: "user"))
                    picking = false
                }
            }
        }
        .confirmationDialog("Delete this note?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) {
                if let id { try? repo.delete(id) }
                dismiss()
            }
        } message: {
            Text("Its key is destroyed, so it can't be recovered.")
        }
        .onAppear(perform: load)
    }

    private func load() {
        guard let id, loaded == nil, let note = repo.snippet(id) else { return }
        loaded = note
        text = note.text
        tags = note.tags
    }

    private func save() {
        let now = Date()
        var note = loaded ?? SnippetData(
            session: nil,
            at: LocalTimeData(
                unixMs: Int64(now.timeIntervalSince1970 * 1000),
                utcOffsetMin: Int16(TimeZone.current.secondsFromGMT(for: now) / 60)
            ),
            text: "", tags: []
        )
        note.text = text.trimmingCharacters(in: .whitespacesAndNewlines)
        note.tags = tags
        do {
            try repo.saveNote(id: id, note)
            dismiss()
        } catch {
            self.error = "The note couldn't be saved. Try again."
        }
    }
}

private struct SuggestKey: Equatable {
    let text: String
    let tags: [TagRefData]
    let rejected: Set<TagRefData>
}

/// The custom tag categories a note can carry, as Android's picker groups.
let noteTagCategories = ["effect", "flavor", "mood", "terpene"]

/// The words a suggestion came from, quoted from `text` by its UTF-16 span.
func phrase(of suggestion: TagSuggestion, in text: String) -> String {
    let utf16 = text.utf16
    let start = min(Int(suggestion.start), utf16.count)
    let end = max(start, min(Int(suggestion.end), utf16.count))
    let from = utf16.index(utf16.startIndex, offsetBy: start)
    let to = utf16.index(utf16.startIndex, offsetBy: end)
    return String(utf16[from..<to]) ?? ""
}

/// Every tag a note can take, in Android's groups: built-in and custom.
private struct TagPicker: View {
    let chosen: Set<TagRefData>
    let custom: [Entry<TagData>]
    let pick: (TagRefData) -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        let vocab = vocabulary()
        let names = Dictionary(uniqueKeysWithValues: custom.map { ($0.id, $0.data.name) })
        let groups: [(String, [String], String)] = [
            ("Effects", vocab.coreEffects, "effect"),
            ("Flavors", vocab.flavors, "flavor"),
            ("Moods", vocab.moods, "mood"),
            ("Terpenes", vocab.terpenes, "terpene"),
        ]
        List {
            ForEach(groups, id: \.0) { title, slugs, category in
                Section(title) {
                    let tags = slugs.map { TagRefData.builtin(slug: $0) }
                        + custom.filter { $0.data.category == category }.map { TagRefData.custom(id: $0.id) }
                    ForEach(tags, id: \.self) { tag in
                        Button(tagLabel(tag, names)) { pick(tag) }
                            .disabled(chosen.contains(tag))
                    }
                }
            }
        }
        .navigationTitle("Add a tag")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Close") { dismiss() } }
        }
    }
}
