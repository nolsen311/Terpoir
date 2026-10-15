// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import androidx.compose.ui.graphics.Color
import kotlin.math.pow
import org.junit.Assert.assertTrue
import org.junit.Test

/** The unlock screen's colors on the brand green meet WCAG AA (4.5:1). */
class BrandColorsTest {
    @Test
    fun textAndButtonsReadOnTheGreen() {
        val c = BrandColors
        val pairs = listOf(
            "onSurface" to (c.onSurface to c.surface),
            "onSurfaceVariant" to (c.onSurfaceVariant to c.surface),
            "primary (text buttons)" to (c.primary to c.surface),
            "error" to (c.error to c.surface),
            "onPrimary (filled buttons)" to (c.onPrimary to c.primary),
        )
        val failures = pairs.filter { (_, p) -> contrast(p.first, p.second) < 4.5 }.map { it.first }
        assertTrue("below 4.5:1: $failures", failures.isEmpty())
        assertTrue(c.surface == BrandGreen)
    }

    private fun contrast(a: Color, b: Color): Double {
        val (light, dark) = listOf(luminance(a), luminance(b)).sortedDescending()
        return (light + 0.05) / (dark + 0.05)
    }

    private fun luminance(c: Color): Double {
        fun channel(v: Float): Double = if (v <= 0.04045f) v / 12.92 else ((v + 0.055) / 1.055).pow(2.4)
        return 0.2126 * channel(c.red) + 0.7152 * channel(c.green) + 0.0722 * channel(c.blue)
    }
}
