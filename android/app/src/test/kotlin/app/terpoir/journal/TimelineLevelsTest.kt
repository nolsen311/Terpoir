// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class TimelineLevelsTest {
    private val smoked = listOf(30, 60, 90, 120)

    @Test
    fun aLongerHoldIsStronger() {
        assertEquals(1, holdLevel(0))
        assertEquals(3, holdLevel(1_000))
        assertEquals(5, holdLevel(2_000))
        assertEquals(5, holdLevel(9_000))
    }

    @Test
    fun theFlickSaysWhichWayItIsHeading() {
        assertEquals(0, flickStep(100f))
        assertEquals(1, flickStep(400f))
        assertEquals(3, flickStep(5_000f))
        assertEquals(-3, flickStep(-5_000f))
    }

    @Test
    fun theGestureSetsTheFirstPointsOnly() {
        assertEquals(mapOf(30 to 3, 60 to 5), gestureLevels(smoked, 2, 1_000, 1_200f))
        // A dab sets one; a flick can't go below 0.
        assertEquals(mapOf(20 to 1), gestureLevels(listOf(20, 40, 60), 1, 0, -5_000f))
        assertEquals(mapOf(30 to 1, 60 to 0), gestureLevels(smoked, 2, 0, -5_000f))
    }

    @Test
    fun unsetPointsAreGuessedAndMarked() {
        val points = timelinePoints(smoked, mapOf(30 to 2, 60 to 4))
        assertEquals(listOf(true, true, false, false), points.map { it.set })
        // Still climbing after two rising points, then easing off.
        assertEquals(5, points[2].level)
        assertTrue(points[3].level < points[2].level)
    }

    @Test
    fun gapsAreFilledInBetween() {
        val points = timelinePoints(smoked, mapOf(30 to 1, 90 to 5))
        assertEquals(3, points[1].level)
        assertEquals(false, points[1].set)
    }

    @Test
    fun pointsOutsideThePlanAreKept() {
        // Saved under another method's plan.
        val points = timelinePoints(listOf(20, 40, 60), mapOf(48 to 3))
        assertEquals(listOf(20, 40, 48, 60), points.map { it.minute })
        assertEquals(TimelinePoint(48, 3, set = true), points[2])
    }

    @Test
    fun nothingSetShowsNothing() {
        assertTrue(timelinePoints(smoked, emptyMap()).all { it.level == 0 && !it.set })
    }
}
