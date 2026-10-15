// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith

/** The default check-in delays come from the core, so this runs on a device. */
@RunWith(AndroidJUnit4::class)
class CheckInDefaultsTest {
    @Test
    fun slowMethodsGetACheckIn() {
        assertEquals(90, CheckIns.defaultMinutes(listOf("edible")))
        assertEquals(90, CheckIns.defaultMinutes(listOf("beverage")))
        assertEquals(45, CheckIns.defaultMinutes(listOf("tincture")))
        assertEquals(120, CheckIns.defaultMinutes(listOf("transdermal")))
    }

    @Test
    fun theSlowestMethodWins() {
        assertEquals(90, CheckIns.defaultMinutes(listOf("tincture", "edible", "smoked")))
        assertEquals(120, CheckIns.defaultMinutes(listOf("edible", "transdermal")))
    }

    @Test
    fun inhaledAndTopicalGetNone() {
        assertNull(CheckIns.defaultMinutes(listOf("smoked", "vape_cart", "dabbed", "topical")))
        assertNull(CheckIns.defaultMinutes(emptyList()))
    }
}
