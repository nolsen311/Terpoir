// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ui.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.ui.graphics.Color
import kotlin.math.pow
import org.junit.Assert.assertTrue
import org.junit.Test

/** Every text color meets WCAG AA (4.5:1) on the backgrounds it is used on. */
class ThemeContrastTest {
    @Test
    fun lightSchemeMeetsAa() = assertAa("light", TerroirLight)

    @Test
    fun darkSchemeMeetsAa() = assertAa("dark", TerroirDark)

    private fun assertAa(name: String, s: ColorScheme) {
        val surfaces = listOf(
            "surface" to s.surface,
            "surfaceContainerLowest" to s.surfaceContainerLowest,
            "surfaceContainerLow" to s.surfaceContainerLow,
            "surfaceContainer" to s.surfaceContainer,
            "surfaceContainerHigh" to s.surfaceContainerHigh,
            "surfaceContainerHighest" to s.surfaceContainerHighest,
        )
        val pairs = listOf(
            Triple("onPrimary/primary", s.onPrimary, s.primary),
            Triple("onPrimaryContainer/primaryContainer", s.onPrimaryContainer, s.primaryContainer),
            Triple("onSecondary/secondary", s.onSecondary, s.secondary),
            Triple("onSecondaryContainer/secondaryContainer", s.onSecondaryContainer, s.secondaryContainer),
            Triple("onTertiary/tertiary", s.onTertiary, s.tertiary),
            Triple("onTertiaryContainer/tertiaryContainer", s.onTertiaryContainer, s.tertiaryContainer),
            Triple("onError/error", s.onError, s.error),
            Triple("onErrorContainer/errorContainer", s.onErrorContainer, s.errorContainer),
            Triple("inverseOnSurface/inverseSurface", s.inverseOnSurface, s.inverseSurface),
        ) + surfaces.flatMap { (bg, color) ->
            // Text on every surface level, including primary (links, text
            // buttons) and tertiary (ratings).
            listOf(
                Triple("onSurface/$bg", s.onSurface, color),
                Triple("onSurfaceVariant/$bg", s.onSurfaceVariant, color),
                Triple("primary/$bg", s.primary, color),
                Triple("tertiary/$bg", s.tertiary, color),
                Triple("error/$bg", s.error, color),
            )
        }
        val failures = pairs
            .map { (label, fg, bg) -> label to contrast(fg, bg) }
            .filter { it.second < AA }
        assertTrue("$name: below 4.5:1: $failures", failures.isEmpty())
    }

    private fun contrast(a: Color, b: Color): Double {
        val (light, dark) = listOf(luminance(a), luminance(b)).sortedDescending()
        return (light + 0.05) / (dark + 0.05)
    }

    /** WCAG 2 relative luminance of an sRGB color. */
    private fun luminance(c: Color): Double {
        fun channel(v: Float): Double = if (v <= 0.04045f) v / 12.92 else ((v + 0.055) / 1.055).pow(2.4)
        return 0.2126 * channel(c.red) + 0.7152 * channel(c.green) + 0.0722 * channel(c.blue)
    }

    private companion object {
        const val AA = 4.5
    }
}
