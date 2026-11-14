// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import SwiftUI

struct LockView: View {
    let error: String?
    let onUnlock: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Text("Terpoir").font(.largeTitle.bold())
            Text("Your journal is locked.")
            if let error {
                Text(error).foregroundStyle(.red).multilineTextAlignment(.center)
            }
            Button("Unlock", action: onUnlock).buttonStyle(.borderedProminent)
        }
        .padding(24)
    }
}

/// Shown over the app whenever it isn't in front (the app switcher, a
/// screen recording or mirroring), so the journal never shows there.
struct PrivacyCover: View {
    var body: some View {
        ZStack {
            Rectangle().fill(.background)
            Text("Terpoir").font(.largeTitle.bold())
        }
        .ignoresSafeArea()
    }
}
