// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class RatingStepperTest {
    @Test
    fun plusStartsAboveTheMiddleAndStopsAtTheTop() {
        assertEquals(4, ratingUp(null))
        assertEquals(4, ratingUp(3))
        assertEquals(5, ratingUp(5))
    }

    @Test
    fun minusAtTheLowestClearsTheRating() {
        assertEquals(2, ratingDown(3))
        assertNull(ratingDown(1))
    }

    @Test
    fun minusWhenUnratedStartsBelowTheMiddle() {
        assertEquals(2, ratingDown(null))
    }

    @Test
    fun fiveFacesFromMehToMindBlown() {
        assertEquals(listOf("🫤", "😑", "😀", "🫠", "🤯"), RATING_FACES)
    }
}
