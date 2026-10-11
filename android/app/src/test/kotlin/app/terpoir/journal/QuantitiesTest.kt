// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import org.junit.Assert.assertEquals
import org.junit.Test

class QuantitiesTest {
    @Test
    fun parsesAmountsToThousandths() {
        assertEquals(Parsed.Milli(1500), parseMilli("1.5"))
        assertEquals(Parsed.Milli(1500), parseMilli("1,5"))
        assertEquals(Parsed.Milli(22_400), parseMilli(" 22.4 "))
        assertEquals(Parsed.Milli(5), parseMilli("0.005"))
        assertEquals(Parsed.Milli(10_000), parseMilli("10"))
    }

    @Test
    fun rejectsWhatIsNotAnAmount() {
        assertEquals(Parsed.Empty, parseMilli("  "))
        for (bad in listOf("-1", "1.2345", "abc", "1.", ".5", "1e3", "1 000")) {
            assertEquals(bad, Parsed.Invalid, parseMilli(bad))
        }
    }

    @Test
    fun formatsWithoutTrailingZeros() {
        assertEquals("1.5", formatMilli(1500))
        assertEquals("2", formatMilli(2000))
        assertEquals("0.005", formatMilli(5))
        assertEquals("22.45", formatMilli(22_450))
        for (milli in listOf(0L, 1L, 999L, 1000L, 123_456L)) {
            assertEquals(Parsed.Milli(milli), parseMilli(formatMilli(milli)))
        }
    }

    @Test
    fun minutesAreWholeAndBounded() {
        assertEquals(Parsed.Milli(45), parseMinutes("45"))
        assertEquals(Parsed.Empty, parseMinutes(""))
        assertEquals(Parsed.Invalid, parseMinutes("1.5"))
        assertEquals(Parsed.Invalid, parseMinutes("-3"))
        assertEquals(Parsed.Invalid, parseMinutes("99999"))
    }
}
