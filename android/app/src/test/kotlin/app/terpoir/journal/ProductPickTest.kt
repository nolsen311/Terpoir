// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Test

/** Pickers name and order products as the Products page's cards do. */
class ProductPickTest {
    private fun card(name: String, strains: List<String>, brand: String?, kind: String?) =
        productTitle(name, strains, brand, kind)

    @Test
    fun aPickIsTitledAsItsCard() {
        val pick = picksOf(listOf("p" to card("Block Berry", listOf("Blue Dream"), "Fire Bros", "Cart"))).single()
        assertEquals("Blue Dream", pick.title)
        assertEquals("Fire Bros · Cart · Block Berry", pick.detail)
        assertEquals("Blue Dream · Fire Bros · Cart · Block Berry", pick.line)
    }

    @Test
    fun picksFollowTheProductsPageOrderNotTheBrand() {
        val picks = picksOf(
            listOf(
                "a" to card("Doozies", emptyList(), "Green Revolution", "Edible"),
                "b" to card("", listOf("Block Berry"), "Fire Bros", "Flower"),
                "c" to card("", listOf("Crater Noir"), "Maui Grown", "Flower"),
            ),
        )
        assertEquals(listOf("Block Berry", "Crater Noir", "Doozies"), picks.map { it.title })
    }

    @Test
    fun theSameStrainIsToldApartByBrandAndKind() {
        val picks = picksOf(
            listOf(
                "cart" to card("", listOf("Blue Dream"), "Knots", "Cart"),
                "flower" to card("", listOf("Blue Dream"), "Fire Bros", "Flower"),
            ),
        )
        assertEquals(listOf("flower", "cart"), picks.map { it.id })
        assertEquals(listOf("Fire Bros · Flower", "Knots · Cart"), picks.map { it.detail })
    }

    @Test
    fun aProductWithNothingButABrandIsTitledByIt() {
        val pick = picksOf(listOf("p" to card("", emptyList(), "Fire Bros", null))).single()
        assertEquals("Fire Bros", pick.title)
        assertEquals("", pick.detail)
        assertEquals("Fire Bros", pick.line)
    }
}
