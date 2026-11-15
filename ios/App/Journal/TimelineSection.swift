// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// The session's timeline (docs/design-decisions.md, "Session timeline"):
/// how strong it felt at points after consumption, 0 to 5, at the points
/// the core plans for the method. This is Android's "Set by hand": a level
/// for each point, and only the points the user sets are saved. Android's
/// hold-and-flick indicator and guessed curve come later.
struct TimelineSection: View {
    let method: String
    @Binding var levels: [UInt32: UInt8]
    @Binding var updatedMs: Int64?

    var body: some View {
        let plan = timelinePlanForMethods(methods: [method])
        // The plan's points, and any set for a method the session had before.
        let minutes = Set(plan.pointMinutes).union(levels.keys).sorted()
        Section {
            ForEach(minutes, id: \.self) { minute in
                Picker(durationLabel(minute), selection: Binding(
                    get: { levels[minute] },
                    set: { level in
                        levels[minute] = level
                        updatedMs = Int64(Date().timeIntervalSince1970 * 1000)
                    }
                )) {
                    Text("Not set").tag(UInt8?.none)
                    ForEach(Array(timelineLevelNames.enumerated()), id: \.offset) { index, name in
                        Text(name).tag(Optional(UInt8(index)))
                    }
                }
            }
            if !levels.isEmpty {
                Button("Start over", role: .destructive) {
                    levels = [:]
                    updatedMs = nil
                }
            }
        } header: {
            Text("Timeline")
        } footer: {
            Text("How's it hitting? Set how strong it felt at each point after you had it. Points you leave unset aren't saved.")
        }
    }
}

/// The levels' names, 0 to 5, as Android's.
let timelineLevelNames = ["None", "Faint", "Mild", "Clear", "Strong", "Intense"]

/// A time after consumption, as Android writes it: "20 min", "1 h", "1 h 36".
func durationLabel(_ minutes: UInt32) -> String {
    if minutes < 60 { return "\(minutes) min" }
    if minutes % 60 == 0 { return "\(minutes / 60) h" }
    return "\(minutes / 60) h \(minutes % 60)"
}
