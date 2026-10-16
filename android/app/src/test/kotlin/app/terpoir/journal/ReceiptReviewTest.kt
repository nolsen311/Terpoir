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
        // A pack and a size stay apart: what 0.5 g covers is decided by withPer.
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

    @Test
    fun theSecondPhotoDrawsItsOwnRows() {
        // First photo: 6 rows, its last one cut, so 5 kept; 2 of them overlap.
        // Second photo: 1 cut row on top, the 2 overlapping rows, 4 below.
        val pairs = secondPhotoRows(secondCount = 7, joinedCount = 9, fromFirst = 5, overlap = 2)
        // The overlap is drawn on the second photo as the first photo's rows 3 and 4.
        assertEquals(listOf(1 to 3, 2 to 4, 3 to 5, 4 to 6, 5 to 7, 6 to 8), pairs)
        // Nothing below the overlap: only the overlap is drawn.
        assertEquals(listOf(0 to 4), secondPhotoRows(secondCount = 1, joinedCount = 5, fromFirst = 5, overlap = 1))
        // Counts that don't fit together draw nothing rather than the wrong rows.
        assertEquals(emptyList<Pair<Int, Int>>(), secondPhotoRows(secondCount = 2, joinedCount = 9, fromFirst = 5, overlap = 2))
    }

    @Test
    fun joiningTheSecondPhotoKeepsTheFirstPhotosCards() {
        // The user split rows 1 and 2, removed row 3's card, and joined 4 with
        // 5 (row 5 was the first photo's last kept row, fromFirst = 6).
        val user = listOf(listOf(1), listOf(2), listOf(4, 5), listOf(6))
        val parsed = listOf(listOf(1, 2), listOf(3), listOf(4), listOf(5, 6), listOf(7), listOf(8, 9))
        // Card [6] pointed past the first photo's kept rows: read again.
        // Parsed [5, 6] wraps across the seam, so it replaces the card [4, 5].
        assertEquals(listOf(listOf(1), listOf(2), listOf(5, 6), listOf(7), listOf(8, 9)), groupsAfterJoin(user, parsed, fromFirst = 6))
        // No wrap across the seam: the user's cards all stay.
        assertEquals(
            listOf(listOf(1), listOf(2), listOf(4, 5), listOf(6), listOf(7)),
            groupsAfterJoin(user.dropLast(1), listOf(listOf(6), listOf(7)), fromFirst = 6),
        )
    }

    @Test
    fun aPacksAmountIsEachPiecesUnlessChosenOtherwise() {
        val printed = PackageSizeData(500, "g", 5u)
        // The core's reading, then the user's choice.
        assertEquals("each", withPer(printed, null) { "each" }?.per)
        assertEquals("total", withPer(printed, "total") { "each" }?.per)
        // The core sees the size without a reading.
        withPer(printed.copy(per = "total"), null) { size -> assertNull(size.per); "each" }
        // Only a pack with an amount has one.
        assertNull(withPer(PackageSizeData(500, "g", null), "total") { "each" }?.per)
        assertNull(withPer(PackageSizeData(null, null, 5u), "total") { "each" }?.per)
        assertNull(withPer(null, "each") { "each" })
    }

    @Test
    fun aTypedSizeNeedsAPositiveAmountOrAWholePack() {
        assertEquals(PackageSizeData(500, "g", 5u), typedSize("0.5", "g", "5"))
        assertEquals(PackageSizeData(100_000, "mg", null), typedSize("100", "mg", ""))
        assertEquals(PackageSizeData(null, null, 10u), typedSize(" ", "ml", "10"))
        // Nothing typed: no size.
        assertNull(typedSize("", "g", ""))
        assertTrue(typedSizeValid("", ""))
        // Not a size.
        for ((amount, pack) in listOf("0" to "", "1.2345" to "", "abc" to "", "1" to "0", "1" to "2.5", "1" to "-3")) {
            assertFalse("$amount / $pack", typedSizeValid(amount, pack))
            assertNull(typedSize(amount, "g", pack))
        }
    }

    @Test
    fun twoProductsThatLearnedALineAreOfferedForMerging() {
        val both = listOf(
            suggestion("p1", "product", "learned", close = true, learned = true),
            suggestion("p2", "product", "learned", close = true, learned = true),
            suggestion("p3", "product", "exact"),
        )
        assertEquals("p1" to "p2", learnedPair(both))
        assertNull(learnedPair(both.drop(1)))
        assertNull(learnedPair(emptyList()))
    }

    @Test
    fun pendingMergesFollowEachKeeperAlong() {
        val merges = listOf(PendingMerge("a", "b"), PendingMerge("c", "a"))
        assertEquals("c", mergedInto("b", merges))
        assertEquals("c", mergedInto("a", merges))
        assertEquals("c", mergedInto("c", merges))
        assertEquals("d", mergedInto("d", merges))
        // Before the second merge, b only goes as far as a.
        assertEquals("a", mergedInto("b", merges.take(1)))
    }
}
