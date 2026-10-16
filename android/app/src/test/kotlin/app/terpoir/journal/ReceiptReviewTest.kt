// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.ItemBreakdownData
import app.terpoir.core.PackageSizeData
import app.terpoir.core.ReceiptShopMatch
import app.terpoir.core.ScanSuggestion
import app.terpoir.core.TagRefData
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ReceiptReviewTest {
    private fun breakdown(
        name: String = "BD",
        milli: Long? = null,
        unit: String? = null,
        form: String? = null,
        quantity: UInt? = null,
        pack: UInt? = null,
        subtype: String? = null,
    ) = ItemBreakdownData(name, milli, unit, form, null, quantity, pack, emptyList(), subtype)

    private fun suggestion(id: String, type: String, kind: String, close: Boolean = false, learned: Boolean = false) =
        ScanSuggestion(id, type, id.uppercase(), kind, 50u, false, close, learned)

    @Test
    fun quantityComesFromTheLeadingCountThenTheName() {
        assertEquals(3u, lineQuantity(3u, breakdown(quantity = 2u)))
        assertEquals(2u, lineQuantity(1u, breakdown(quantity = 2u)))
        assertEquals(1u, lineQuantity(1u, breakdown()))
        assertEquals(1u, lineQuantity(0u, breakdown(quantity = 0u)))
    }

    @Test
    fun pricesShowInTheCurrencysMinorUnit() {
        assertEquals("35.00", priceText(3_500, "USD"))
        assertEquals("1200", priceText(1_200, "JPY"))
        assertEquals("1.250", priceText(1_250, "KWD"))
        // What the field shows reads back as the same amount.
        assertEquals(3_500L, parsePrice(priceText(3_500, "USD"), "USD"))
    }

    @Test
    fun sizesShowAsPrinted() {
        assertEquals("3.5 g", sizeAmountText(3_500, "g"))
        assertEquals("100 mg", sizeAmountText(100_000, "mg"))
        // A pack and a size stay apart: whether 0.5 g is per piece isn't decided.
        assertEquals(PackageSizeData(500, "g", 5u), breakdown(milli = 500, unit = "g", pack = 5u).packageSize())
    }

    @Test
    fun onlyAnUnambiguousLearnedProductLinksOnItsOwn() {
        assertEquals("p1", autoLinked(listOf(suggestion("p1", "product", "learned", learned = true))))
        // Two products learned the same line (after a sync): offered, not linked.
        assertNull(
            autoLinked(
                listOf(
                    suggestion("p1", "product", "learned", close = true, learned = true),
                    suggestion("p2", "product", "learned", close = true, learned = true),
                ),
            ),
        )
        // An exact name match is still only a suggestion.
        assertNull(autoLinked(listOf(suggestion("p1", "product", "exact"))))
        assertNull(autoLinked(emptyList()))
    }

    @Test
    fun cardsOfferTheBestFewSuggestions() {
        val list = (1..6).map { suggestion("s$it", "strain", "letters") } + suggestion("x", "other", "exact")
        assertEquals(listOf("s1", "s2", "s3", "s4"), offeredSuggestions(list).map { it.id })
    }

    @Test
    fun aNewProductIsPrefilledFromNamesTheLinePrintsInFull() {
        val list = listOf(
            suggestion("brand1", "brand", "words"),
            suggestion("strain1", "strain", "exact"),
            suggestion("strain2", "strain", "initials"),
        )
        val line = breakdown(name = "ACME BLUE DREAM")
        assertEquals(NewProductPrefill("brand1", "strain1", ""), newProductPrefill(line, list, brands = true, strains = true))
        // Hidden in "What I track": nothing pre-filled, and the line names it.
        assertEquals(NewProductPrefill(null, null, "ACME BLUE DREAM"), newProductPrefill(line, list, brands = false, strains = false))
        // Initials alone (BD) are a guess for the user to pick, never a pre-fill.
        val initials = listOf(suggestion("strain2", "strain", "initials"))
        assertEquals(NewProductPrefill(null, null, "BD"), newProductPrefill(breakdown(), initials, brands = true, strains = true))
    }

    @Test
    fun aNewProductTakesItsTypeSubtypeAndSizeFromTheLine() {
        val line = breakdown(name = "GG4", milli = 1_000, unit = "g", form = "concentrate", subtype = "rosin")
        val product = newProductData("", null, "strain1", line, "GG4 ROSIN 1G")
        assertEquals("", product.name)
        assertEquals(listOf("strain1"), product.strains)
        assertEquals("concentrate", product.form)
        assertEquals(TagRefData.Builtin("rosin"), product.subtype)
        assertEquals(PackageSizeData(1_000, "g", null), product.defaultSize)
        // With nothing else to name it, the printed line does.
        assertEquals("MYSTERY 12", newProductData(" ", null, null, breakdown(name = "MYSTERY 12"), "MYSTERY 12").name)
        // A subtype needs its type.
        assertNull(newProductData("x", null, null, breakdown(subtype = "rosin"), "x").subtype)
    }

    private fun match(spot: String?, strong: Boolean) =
        ReceiptShopMatch(spot, if (spot == null) null else if (strong) "phone" else "name", strong, null, "GREEN LEAF", 0u, emptyList(), emptyList())

    @Test
    fun theShopCardStartsFromTheCoresMatch() {
        assertEquals(ShopState.Recognised("s1"), shopState(match("s1", strong = true)))
        assertEquals(ShopState.Guessed("s1"), shopState(match("s1", strong = false)))
        assertEquals(ShopState.Unknown, shopState(match(null, strong = false)))
    }

    @Test
    fun onlyAShopTheUserConfirmedLearnsTheReceiptsNumbers() {
        assertTrue(ShopState.Confirmed("s1").remembers())
        assertTrue(ShopState.New("Green Leaf").remembers())
        assertFalse(ShopState.Guessed("s1").remembers())
        assertFalse(ShopState.Recognised("s1").remembers())
        assertFalse(ShopState.None.remembers())
        assertEquals("s1", ShopState.Guessed("s1").spot())
        assertNull(ShopState.New("Green Leaf").spot())
    }

    @Test
    fun joinWithNextTakesTheRowBelowOrItsWholeCard() {
        val groups = listOf(listOf(2), listOf(3, 4), listOf(6))
        assertEquals(listOf(listOf(2, 3, 4), listOf(6)), joinWithNext(groups, 0, rowCount = 8))
        // A row in no card joins on its own.
        assertEquals(listOf(listOf(2), listOf(3, 4, 5), listOf(6)), joinWithNext(groups, 1, rowCount = 8))
        assertEquals(listOf(listOf(2), listOf(3, 4), listOf(6, 7)), joinWithNext(groups, 2, rowCount = 8))
        // Nothing below, or the shop's name below.
        assertNull(joinWithNext(groups, 2, rowCount = 7))
        assertNull(joinWithNext(groups, 1, rowCount = 8, reserved = 5))
    }

    @Test
    fun splitHereMakesTwoCards() {
        val groups = listOf(listOf(1), listOf(3, 4, 5))
        assertEquals(listOf(listOf(1), listOf(3), listOf(4, 5)), splitGroup(groups, 1, 1))
        assertEquals(listOf(listOf(1), listOf(3, 4), listOf(5)), splitGroup(groups, 1, 2))
        assertNull(splitGroup(groups, 1, 0))
        assertNull(splitGroup(groups, 0, 1))
    }

    @Test
    fun aMissingLineIsAddedInPrintedOrder() {
        val groups = listOf(listOf(2), listOf(6, 7))
        assertEquals(listOf(listOf(0), listOf(2), listOf(6, 7)), addRowGroup(groups, 0))
        assertEquals(listOf(listOf(2), listOf(4), listOf(6, 7)), addRowGroup(groups, 4))
        assertEquals(listOf(listOf(2), listOf(6, 7), listOf(9)), addRowGroup(groups, 9))
        assertEquals(groups, addRowGroup(groups, 7))
    }

    @Test
    fun zoomKeepsThePointUnderTheFingerAndStaysInside() {
        val w = 400f
        val h = 800f
        val zoomed = PhotoZoom().zoomed(2f, 100f, 200f, w, h)
        assertEquals(2f, zoomed.scale)
        // The content point under (100, 200) is still there.
        assertEquals(100f to 200f, zoomed.toContent(100f, 200f))
        // Panning never shows past the edges.
        val panned = zoomed.panned(1_000f, -10_000f, w, h)
        assertEquals(0f, panned.x)
        assertEquals(h - 2 * h, panned.y)
        // Never smaller than Fit, never past the most.
        assertEquals(PhotoZoom(), zoomed.zoomed(0.1f, 0f, 0f, w, h))
        assertEquals(PhotoZoom.MAX_ZOOM, PhotoZoom().zoomed(100f, 0f, 0f, w, h).scale)
    }

    @Test
    fun theShopsNameStartsZoomedToTheTop() {
        // A tall receipt fitted in the middle of a wide viewport, 100 px wide.
        val top = PhotoZoom.top(imageLeft = 150f, imageTop = 0f, imageWidth = 100f, width = 400f, height = 800f)
        assertEquals(4f, top.scale, 0f)
        assertEquals(-600f, top.x, 0f)
        assertEquals(0f, top.y, 0f)
        // The fitted photo's left edge is at the viewport's left edge.
        assertEquals(150f, top.toContent(0f, 0f).first, 0f)
    }
}
