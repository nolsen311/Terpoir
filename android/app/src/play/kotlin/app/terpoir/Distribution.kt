// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import app.terpoir.onboarding.AgeSignals

/** The Google Play distribution. */
object Distribution {
    /** Short name, for the about screen. */
    const val NAME = "play"

    /** Google-dependent features this build may offer (each still opt-in). */
    val capabilities: Set<Capability> = Capability.entries.toSet()

    /**
     * Platform age signal. None yet: Google Play Age Signals is a
     * proprietary library (Android SDK License) and needs a licensing
     * decision first; see "Age gating" in docs/design-decisions.md.
     * Self-attestation is the gate until then.
     */
    val ageSignals: AgeSignals = AgeSignals.NONE
}
