// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import android.content.Context

/** What a platform age signal says about the user. Read only; never stored. */
enum class AgeCategory {
    /** The platform says the user is an adult. */
    ADULT,

    /** The platform says the user is a minor: show only "Terpoir is for adults". */
    MINOR,

    /** No signal (unsupported, unavailable, or no answer): self-attestation decides. */
    UNKNOWN,
}

/**
 * A platform age signal, checked on the device. See "Age gating" in
 * docs/design-decisions.md. The result is read once and never stored or
 * sent anywhere.
 */
fun interface AgeSignals {
    /** The user's age category; [AgeCategory.UNKNOWN] whenever unsure. */
    suspend fun category(context: Context): AgeCategory

    companion object {
        /** No platform signal: self-attestation is the only gate. */
        val NONE = AgeSignals { AgeCategory.UNKNOWN }
    }
}
