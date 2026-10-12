// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Test

class ProductLabelTest {
    @Test
    fun brandThenName() {
        assertEquals("Fire Bros Block Berry Cart", productLabel("Block Berry Cart", "Fire Bros"))
    }

    @Test
    fun noBrandOrNoName() {
        assertEquals("Block Berry Cart", productLabel("Block Berry Cart", null))
        assertEquals("Block Berry Cart", productLabel(" Block Berry Cart ", " "))
        assertEquals("Fire Bros", productLabel("", "Fire Bros"))
    }

    @Test
    fun aNameStartingWithTheBrandIsNotDoubled() {
        assertEquals("fire bros block berry", productLabel("fire bros block berry", "Fire Bros"))
    }
}
