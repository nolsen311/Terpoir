// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CheckInDefaultsTest {
    @Test
    fun slowMethodsGetACheckIn() {
        assertEquals(90, CheckIns.defaultMinutes(listOf("edible")))
        assertEquals(90, CheckIns.defaultMinutes(listOf("beverage")))
        assertEquals(45, CheckIns.defaultMinutes(listOf("tincture")))
    }

    @Test
    fun theSlowestMethodWins() {
        assertEquals(90, CheckIns.defaultMinutes(listOf("tincture", "edible", "smoked")))
    }

    @Test
    fun inhaledAndTopicalGetNone() {
        assertNull(CheckIns.defaultMinutes(listOf("smoked", "vape_cart", "dabbed", "topical")))
        assertNull(CheckIns.defaultMinutes(emptyList()))
    }
}
