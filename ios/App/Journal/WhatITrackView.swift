// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI

/// "What I track" (as Android's): the data points the journal has.
/// Everything starts shown. Turning one off asks whether to keep its
/// values, hidden, or delete them from every entry; the choice syncs, so
/// every paired device shows the same fields.
struct WhatITrackView: View {
    @ObservedObject var repo: JournalRepository
    @State private var turningOff: DataPoint?
    @State private var error: String?

    var body: some View {
        let _ = repo.revision
        let tracking = repo.tracking()
        Form {
            Section {
                ForEach(DataPoint.allCases) { point in
                    // Hiding purchases hides prices and shops with them.
                    let covered = point == .priceAndShop && !tracking.shows(.purchases)
                    Toggle(isOn: Binding(
                        get: { tracking.shows(point) },
                        set: { on in
                            if on { change { try repo.setTracked(point, true) } } else { turningOff = point }
                        }
                    )) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(point.title)
                            Text(covered ? "Off while purchases are off." : point.does)
                                .font(.footnote).foregroundStyle(.secondary)
                        }
                    }
                    .disabled(covered)
                }
            } header: {
                Text("Turn off what you don't care about, and Terpoir stops asking for it. Your choices apply on every device you sync with.")
                    .textCase(nil)
            } footer: {
                Text("When, product, method, effects and rating are always tracked: they're what makes the journal and its insights work.")
            }
            if tracking.shows(.strain) {
                Section {
                    Picker("Strain detail", selection: Binding(
                        get: { tracking.strainDetail },
                        set: { step in change { try repo.setStrainDetail(step) } }
                    )) {
                        ForEach(StrainDetail.allCases) { step in
                            VStack(alignment: .leading, spacing: 2) {
                                Text(step.title)
                                Text(step.does).font(.footnote).foregroundStyle(.secondary)
                            }
                            .tag(step)
                        }
                    }
                    .pickerStyle(.inline)
                    .labelsHidden()
                } header: {
                    Text("Strain detail")
                } footer: {
                    Text("How much a strain records. A lower step hides the extra fields; it never deletes what's saved.")
                }
            }
            if let error {
                Text(error).foregroundStyle(.red)
            }
        }
        .navigationTitle("What I track")
        .confirmationDialog(
            turningOff.map { "Stop tracking \($0.title)?" } ?? "",
            isPresented: Binding(get: { turningOff != nil }, set: { if !$0 { turningOff = nil } }),
            titleVisibility: .visible,
            presenting: turningOff
        ) { point in
            Button("Keep, hidden") { change { try repo.setTracked(point, false) } }
            Button("Delete", role: .destructive) {
                change {
                    try repo.setTracked(point, false)
                    try repo.deleteDataPoint(point)
                }
            }
            Button("Cancel", role: .cancel) {}
        } message: { point in
            Text("Keep it hidden: anything you saved stays, out of sight, and comes back if you turn \(point.title) on again. Delete: removes \(point.title) from every entry, on every device you sync with. Deleting can't be undone.")
        }
    }

    private func change(_ action: () throws -> Void) {
        do {
            try action()
            error = nil
        } catch {
            self.error = "That couldn't be changed. Try again."
        }
    }
}
