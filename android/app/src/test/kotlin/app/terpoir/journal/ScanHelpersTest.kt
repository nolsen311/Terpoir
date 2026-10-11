// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.IdentifierData
import app.terpoir.core.ProductData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ScanHelpersTest {
    private val existing = ProductData(
        name = "Blue Dream",
        form = "flower",
        brand = "ab".repeat(16),
        strains = listOf("cd".repeat(16)),
        thcMilli = 21_000,
        thcUnit = "percent",
        cbdMilli = null,
        cbdUnit = null,
        terpenes = listOf(TagIntensity(TagRefData.Builtin("myrcene"), 500)),
        batch = "OLD-1",
        packageDateDay = 20_000,
        identifiers = listOf(IdentifierData("0123456789012", "barcode")),
    )

    @Test
    fun sameProductNewBatchKeepsIdentityAndTakesTheNewBatch() {
        val scanned = existing.copy(
            name = "BLUE DREAM 3.5G",
            brand = null,
            strains = emptyList(),
            thcMilli = 23_500,
            terpenes = emptyList(),
            batch = "NEW-2",
            packageDateDay = 20_700,
            identifiers = listOf(IdentifierData("0123456789012", "barcode"), IdentifierData("1A4060300002EE1000012345", "package_tag")),
        )
        val merged = mergeBatch(existing, scanned)
        assertEquals("Blue Dream", merged.name)
        assertEquals(existing.brand, merged.brand)
        assertEquals(existing.strains, merged.strains)
        assertEquals(23_500L, merged.thcMilli)
        assertEquals(existing.terpenes, merged.terpenes)
        assertEquals("NEW-2", merged.batch)
        assertEquals(20_700L, merged.packageDateDay)
        assertEquals(listOf("0123456789012", "1A4060300002EE1000012345"), merged.identifiers.map { it.code })
    }

    @Test
    fun pricesFollowTheCurrencysMinorUnit() {
        assertEquals(3_500L, parsePrice("35", "USD"))
        assertEquals(3_550L, parsePrice("35,50", "EUR"))
        assertEquals(1_200L, parsePrice("1200", "JPY"))
        assertNull(parsePrice("12.5", "JPY"))
        assertNull(parsePrice("12.555", "USD"))
        assertNull(parsePrice("abc", "USD"))
    }
}
