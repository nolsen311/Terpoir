// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class SpotNamesTest {
    @Test
    fun defaultNamesFillTheFirstGap() {
        assertEquals("Spot 1", nextSpotName(emptyList(), "Spot"))
        assertEquals("Spot 3", nextSpotName(listOf("Spot 1", "Spot 2", "Beach"), "Spot"))
        assertEquals("Spot 2", nextSpotName(listOf("Spot 1", "Spot 3"), "Spot"))
        // Names that only look similar don't count.
        assertEquals("Spot 1", nextSpotName(listOf("Spotted 1", "Spot one", "Spot 1x"), "Spot"))
    }

    @Test
    fun onlyTerpoirsOwnNamesCountAsDefault() {
        // An address search's pick replaces these...
        for (name in listOf("Spot 1", "Spot 12", " Spot 3 ", "", "  ")) assertTrue(name, isDefaultSpotName(name, "Spot"))
        // ...and never a name the user chose.
        for (name in listOf("Beach", "Spot", "Spot one", "Spot 1x", "My Spot 2", "Spotted 1")) {
            assertFalse(name, isDefaultSpotName(name, "Spot"))
        }
    }
}
