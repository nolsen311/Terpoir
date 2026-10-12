// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.share

import org.junit.Assert.assertEquals
import org.junit.Test

class ShareTextTest {
    private val lines = listOf(ShareLine("Product", "Fire Bros Block Berry Cart"), ShareLine("Rating", "4/5"))

    @Test
    fun plainTextHasExactlyTheChosenLines() {
        assertEquals(
            "A session\nProduct: Fire Bros Block Berry Cart\nRating: 4/5\n\nMade with Terpoir",
            ShareText("A session", lines, "Made with Terpoir").plain(),
        )
    }

    @Test
    fun withoutTheMarkNothingTrails() {
        assertEquals("A session\nProduct: Fire Bros Block Berry Cart\nRating: 4/5", ShareText("A session", lines, null).plain())
        assertEquals("A session", ShareText("A session", emptyList(), null).plain())
    }
}
