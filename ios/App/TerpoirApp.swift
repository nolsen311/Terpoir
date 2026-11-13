// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The iOS app (docs/roadmap.md, "iOS", I2): SwiftUI and OS integration
// only. Everything the journal is (crypto, the data model, sync, merging)
// lives in the Rust core, reached through TerpoirCore.

import SwiftUI
import TerpoirCore

@main
struct TerpoirApp: App {
    init() {
        // The core's panic hook, which never reports panic messages.
        TerpoirCore.`init`()
    }

    var body: some Scene {
        WindowGroup {
            RootView()
        }
    }
}
