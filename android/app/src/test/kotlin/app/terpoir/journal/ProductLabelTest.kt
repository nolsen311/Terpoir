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

    @Test
    fun anUnnamedProductIsKnownByItsStrains() {
        assertEquals("Fire Bros Blue Dream", productLabel("", "Fire Bros", listOf("Blue Dream")))
        assertEquals("Blue Dream / Haze", productLabel(" ", null, listOf("Blue Dream", "Haze")))
        // Its own name wins over its strains.
        assertEquals("Fire Bros Cart", productLabel("Cart", "Fire Bros", listOf("Blue Dream")))
    }

    @Test
    fun theCatalogTitlesAProductByItsStrain() {
        assertEquals(
            ProductTitle("Blue Dream", "Fire Bros · Vape cart · Block Berry · THC 85%"),
            productTitle("Block Berry", listOf("Blue Dream"), "Fire Bros", "Vape cart", listOf("THC 85%")),
        )
        // No strain: its own name, then brand, then type, as the title.
        assertEquals(ProductTitle("Block Berry", "Fire Bros · Flower"), productTitle("Block Berry", emptyList(), "Fire Bros", "Flower"))
        assertEquals(ProductTitle("Fire Bros", "Flower"), productTitle("", emptyList(), "Fire Bros", "Flower"))
        assertEquals(ProductTitle("Flower", ""), productTitle("", emptyList(), null, "Flower"))
    }
}
