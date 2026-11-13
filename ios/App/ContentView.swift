// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI
import TerpoirCore

/// A placeholder until onboarding and unlock (docs/roadmap.md, "iOS", I2).
struct ContentView: View {
    var body: some View {
        VStack(spacing: 12) {
            Text("Terpoir")
                .font(.largeTitle)
            Text(AppInfo.coreLine)
                .font(.footnote)
                .foregroundStyle(.secondary)
        }
        .padding()
    }
}

/// What the about screen will show.
enum AppInfo {
    static var coreLine: String { "Core \(coreVersion())" }
}
