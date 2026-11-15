// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI

/// When the journal locks after leaving the app (as Android's auto-lock
/// card), kept on this device.
struct AutoLockView: View {
    private let settings = LockSettings()
    @State private var selected = LockSettings().timeout

    var body: some View {
        Form {
            Section {
                Picker("App lock", selection: $selected) {
                    ForEach(LockTimeout.allCases) { Text($0.title).tag($0) }
                }
                .pickerStyle(.inline)
                .labelsHidden()
            } footer: {
                Text("Leaving the app always hides your journal from the app switcher, and locking your phone while Terpoir is open locks it at once. A timer is checked when you come back: until then, your journal's key stays in memory while Terpoir is in the background. If you share your phone, choose \"As soon as I leave Terpoir\".")
            }
        }
        .navigationTitle("App lock")
        .onChange(of: selected) { _, choice in settings.timeout = choice }
    }
}
