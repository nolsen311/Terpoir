// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Test

class StrainDetailTest {
    @Test
    fun stepsMatchTheCore() {
        // The core's vocabulary().strainDetails, which its own tests pin.
        assertEquals(listOf("simple", "terpenes", "detailed"), StrainDetail.entries.map { it.slug })
    }

    @Test
    fun aStepFromANewerAppShowsEverything() {
        assertEquals(StrainDetail.SIMPLE, StrainDetail.of("simple"))
        assertEquals(StrainDetail.DETAILED, StrainDetail.of("lab_grade"))
    }

    @Test
    fun theDefaultIsTerpenes() {
        assertEquals(StrainDetail.TERPENES, Tracking.EVERYTHING.strainDetail)
    }
}
