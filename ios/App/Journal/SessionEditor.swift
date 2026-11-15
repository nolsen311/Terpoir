// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// A session: when, what was had (one product, method and dose), going in
/// (intention, solo or with others, mood before), effects, the timeline,
/// mood after, a rating and whether you'd have it again. Fields it doesn't
/// show yet (symptoms, spot...) and fields hidden in "What I track" are
/// kept as they are.
struct SessionEditor: View {
    @ObservedObject var repo: JournalRepository
    let id: String?
    @Environment(\.dismiss) private var dismiss

    @State private var loaded: SessionData?
    @State private var when = Date()
    @State private var product = ""
    @State private var method = ""
    @State private var dose = ""
    @State private var doseUnit = ""
    @State private var effects: Set<String> = []
    @State private var rating = 0
    @State private var wouldRepeat: Bool?
    @State private var intention: String?
    @State private var social: String?
    @State private var moodBefore: UInt8?
    @State private var moodAfter: UInt8?
    /// The timeline's points the user set: minutes after consumption to a
    /// level, 0 to 5.
    @State private var timeline: [UInt32: UInt8] = [:]
    @State private var timelineUpdatedMs: Int64?
    @State private var error: String?
    @State private var confirmDelete = false

    private let vocab = vocabulary()

    var body: some View {
        let names = CatalogNames(repo)
        let products = repo.products()
            .filter { !$0.data.archived || $0.id == product }
            .map { ($0.id, names.title($0.data)) }
            .sorted { $0.1.title.localizedCaseInsensitiveCompare($1.1.title) == .orderedAscending }
        let tracking = repo.tracking()
        Form {
            DatePicker("When", selection: $when)
            Section("What you had") {
                Picker("Product", selection: $product) {
                    Text("No product").tag("")
                    ForEach(products, id: \.0) { item in
                        let (productId, title) = item
                        Text([title.title, title.brand].filter { !$0.isEmpty }.joined(separator: " · ")).tag(productId)
                    }
                }
                .onChange(of: product) { _, newProduct in
                    let form = repo.product(newProduct)?.form
                    if let usual = repo.usualMethod(form: form) { method = usual }
                }
                Picker("Method", selection: $method) {
                    ForEach(vocab.methods, id: \.self) { Text(label($0)).tag($0) }
                }
                // A hidden dose is saved as it was.
                if repo.tracking().shows(.dose) {
                    HStack {
                        TextField("Dose", text: $dose).keyboardType(.decimalPad)
                        Picker("Unit", selection: $doseUnit) {
                            Text("—").tag("")
                            ForEach(doseUnitsForMethod(method: method), id: \.self) { Text(label($0)).tag($0) }
                        }
                        .labelsHidden()
                    }
                }
            }
            if tracking.shows(.intention) || tracking.shows(.social) || tracking.shows(.mood) {
                Section("Going in") {
                    if tracking.shows(.intention) {
                        Picker("Intention", selection: $intention) {
                            Text("None").tag(String?.none)
                            ForEach(vocab.intentions, id: \.self) { Text(label($0)).tag(Optional($0)) }
                        }
                    }
                    if tracking.shows(.social) {
                        Picker("Who with", selection: $social) {
                            Text("Not saying").tag(String?.none)
                            ForEach(vocab.social, id: \.self) { Text(label($0)).tag(Optional($0)) }
                        }
                    }
                    if tracking.shows(.mood) {
                        MoodPicker(title: "Mood before", value: $moodBefore)
                    }
                }
            }
            Section("Effects") {
                ForEach(vocab.coreEffects, id: \.self) { effect in
                    Toggle(label(effect), isOn: Binding(
                        get: { effects.contains(effect) },
                        set: { on in if on { effects.insert(effect) } else { effects.remove(effect) } }
                    ))
                }
            }
            if tracking.shows(.timeline) {
                TimelineSection(method: method, levels: $timeline, updatedMs: $timelineUpdatedMs)
            }
            Section("How was it?") {
                if tracking.shows(.mood) {
                    MoodPicker(title: "Mood after", value: $moodAfter)
                }
                Stepper(rating == 0 ? "No rating" : String(repeating: "★", count: rating), value: $rating, in: 0...5)
                    .accessibilityValue(rating == 0 ? "No rating" : "\(rating) of 5")
                Picker("Have it again?", selection: $wouldRepeat) {
                    Text("Not saying").tag(Bool?.none)
                    Text("Yes").tag(Bool?.some(true))
                    Text("No").tag(Bool?.some(false))
                }
            }
            if let error {
                Text(error).foregroundStyle(.red)
            }
            if id != nil {
                Button("Delete session", role: .destructive) { confirmDelete = true }
            }
        }
        .navigationTitle(id == nil ? "New session" : "Session")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
            ToolbarItem(placement: .confirmationAction) { Button("Save", action: save) }
        }
        .confirmationDialog("Delete this session?", isPresented: $confirmDelete, titleVisibility: .visible) {
            Button("Delete", role: .destructive) {
                if let id { try? repo.delete(id) }
                dismiss()
            }
        }
        .onAppear(perform: load)
    }

    private func load() {
        if method.isEmpty { method = vocab.methods.first ?? "" }
        guard let id, loaded == nil, let session = repo.session(id) else { return }
        loaded = session
        when = Date(timeIntervalSince1970: Double(session.when.unixMs) / 1000)
        if let line = session.lines.first {
            product = line.product ?? ""
            method = line.method
            dose = line.doseMilli.map { Self.amount($0) } ?? ""
            doseUnit = line.doseUnit ?? ""
        }
        effects = Set(session.effects.compactMap { item -> String? in
            if case let .builtin(slug) = item.tag { return slug }
            return nil
        })
        rating = Int(session.rating ?? 0)
        wouldRepeat = session.wouldRepeat
        intention = session.intention
        social = session.social
        moodBefore = session.moodBefore
        moodAfter = session.moodAfter
        timeline = Dictionary(session.timelineLevels.map { ($0.minute, $0.level) }, uniquingKeysWith: { first, _ in first })
        timelineUpdatedMs = session.timelineUpdatedMs
    }

    private func save() {
        let doseText = dose.replacingOccurrences(of: ",", with: ".").trimmingCharacters(in: .whitespaces)
        var doseMilli: Int64?
        if !doseText.isEmpty {
            guard let value = Double(doseText), value >= 0 else {
                error = "The dose isn't a number."
                return
            }
            doseMilli = Int64((value * 1000).rounded())
        }
        var line = loaded?.lines.first ?? ProductLineData(
            lineId: nil, product: nil, method: method, doseMilli: nil, doseUnit: nil, purchaseId: nil, purchaseLineId: nil, shop: nil
        )
        line.product = product.isEmpty ? nil : product
        line.method = method
        line.doseMilli = doseMilli
        line.doseUnit = doseMilli == nil || doseUnit.isEmpty ? nil : doseUnit
        var session = loaded ?? SessionData(
            when: LocalTimeData(unixMs: 0, utcOffsetMin: 0), lines: [], activity: nil, intention: nil, moodBefore: nil,
            social: nil, effects: [], onsetMin: nil, peakMin: nil, durationMin: nil, moodAfter: nil, rating: nil,
            wouldRepeat: nil, spot: nil, symptoms: [], attachments: []
        )
        session.when = LocalTimeData(
            unixMs: Int64(when.timeIntervalSince1970 * 1000),
            utcOffsetMin: Int16(TimeZone.current.secondsFromGMT(for: when) / 60)
        )
        session.lines = [line] + (loaded?.lines.dropFirst().map { $0 } ?? [])
        // Effects this screen shows are set here; custom ones are kept.
        let kept = session.effects.filter { tagIntensity in
            if case let .builtin(slug) = tagIntensity.tag { return !vocab.coreEffects.contains(slug) }
            return true
        }
        let previous = Dictionary(session.effects.compactMap { item -> (String, Int64)? in
            if case let .builtin(slug) = item.tag { return (slug, item.value) }
            return nil
        }, uniquingKeysWith: { first, _ in first })
        session.effects = kept + vocab.coreEffects.filter(effects.contains).map {
            TagIntensity(tag: .builtin(slug: $0), value: previous[$0] ?? 3)
        }
        session.rating = rating == 0 ? nil : UInt8(rating)
        session.wouldRepeat = wouldRepeat
        // Hidden fields hold what was loaded, so they're saved unchanged.
        session.intention = intention
        session.social = social
        session.moodBefore = moodBefore
        session.moodAfter = moodAfter
        session.timelineLevels = timeline.keys.sorted().map { TimelineLevelData(minute: $0, level: timeline[$0] ?? 0) }
        session.timelineUpdatedMs = timeline.isEmpty ? nil : timelineUpdatedMs
        do {
            try repo.saveSession(id: id, session)
            dismiss()
        } catch {
            self.error = "The session couldn't be saved. Try again."
        }
    }

    static func amount(_ milli: Int64) -> String {
        let value = Double(milli) / 1000
        return value == value.rounded() ? String(Int64(value)) : String(value)
    }
}

/// A mood from 1 (low) to 5 (great), as Android's scale, or none.
struct MoodPicker: View {
    let title: String
    @Binding var value: UInt8?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
            Picker(title, selection: $value) {
                Text("–").tag(UInt8?.none)
                ForEach(UInt8(1)...UInt8(5), id: \.self) { Text("\($0)").tag(Optional($0)) }
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            HStack {
                Text("Low")
                Spacer()
                Text("Great")
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
    }
}
