// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

/** The Google Play distribution. */
object Distribution {
    /** Short name, for the about screen. */
    const val NAME = "play"

    /** Google-dependent features this build may offer (each still opt-in). */
    val capabilities: Set<Capability> = Capability.entries.toSet()
}
