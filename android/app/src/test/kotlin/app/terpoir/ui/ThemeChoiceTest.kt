// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ui

import app.terpoir.ui.theme.ThemeChoice
import org.junit.Assert.assertEquals
import org.junit.Test

class ThemeChoiceTest {
    @Test
    fun systemFollowsThePhoneAndTheOthersDont() {
        assertEquals(listOf(false, true), listOf(false, true).map { ThemeChoice.SYSTEM.isDark(it) })
        assertEquals(listOf(false, false), listOf(false, true).map { ThemeChoice.LIGHT.isDark(it) })
        assertEquals(listOf(true, true), listOf(false, true).map { ThemeChoice.DARK.isDark(it) })
    }

    @Test
    fun theDefaultIsTheFirstChoice() {
        // AppearanceSettings falls back to SYSTEM; it also leads the list.
        assertEquals(ThemeChoice.SYSTEM, ThemeChoice.entries.first())
    }
}
