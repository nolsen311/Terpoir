// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import org.junit.Assert.assertEquals
import org.junit.Test

class AgeCategoryTest {
    @Test
    fun underEighteenIsAMinor() {
        assertEquals(AgeCategory.MINOR, ageCategoryFromRange(13, 15))
        assertEquals(AgeCategory.MINOR, ageCategoryFromRange(16, 17))
        assertEquals(AgeCategory.MINOR, ageCategoryFromRange(null, 12))
    }

    @Test
    fun eighteenPlusIsAnAdult() {
        assertEquals(AgeCategory.ADULT, ageCategoryFromRange(18, null))
    }

    @Test
    fun noRangeIsUnknown() {
        assertEquals(AgeCategory.UNKNOWN, ageCategoryFromRange(null, null))
    }

    @Test
    fun aRangeStraddlingEighteenIsUnknown() {
        assertEquals(AgeCategory.UNKNOWN, ageCategoryFromRange(16, 20))
    }
}
