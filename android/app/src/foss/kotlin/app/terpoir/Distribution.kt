// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

/**
 * The FOSS distribution (F-Droid, GitHub Releases): no Google Play services,
 * no proprietary libraries, and every feature available under the GPL.
 */
object Distribution {
    /** Short name, for the about screen. */
    const val NAME = "foss"

    /** No Google-dependent features. */
    val capabilities: Set<Capability> = emptySet()
}
