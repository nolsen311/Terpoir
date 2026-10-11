// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import android.content.Context

/** What a platform age signal says about the user. Read only; never stored. */
enum class AgeCategory {
    /**
     * The platform says 18 or over. That is below the legal age for
     * cannabis in many places (21 in most US states), so self-attestation
     * still decides.
     */
    ADULT,

    /** The platform says the user is under 18: show only "Terpoir is for adults". */
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

/**
 * The category for an age range from a platform signal, where either bound
 * may be missing. Only an upper bound under 18 makes a minor.
 */
fun ageCategoryFromRange(lower: Int?, upper: Int?): AgeCategory = when {
    upper != null && upper < ADULT_AGE -> AgeCategory.MINOR
    lower != null && lower >= ADULT_AGE -> AgeCategory.ADULT
    else -> AgeCategory.UNKNOWN
}

private const val ADULT_AGE = 18
