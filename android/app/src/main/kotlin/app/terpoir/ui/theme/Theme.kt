// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ui.theme

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

/** Terroir's corners: chips 8, fields 12, cards 16, sheets 28. */
val TerroirShapes = Shapes(
    extraSmall = RoundedCornerShape(4.dp),
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(12.dp),
    large = RoundedCornerShape(16.dp),
    extraLarge = RoundedCornerShape(28.dp),
)

/** Whether the Terroir theme in use is dark. */
private val LocalDark = staticCompositionLocalOf { false }

/**
 * The app's theme: Terroir, light or dark (the phone's setting unless the
 * user chose one, see [AppearanceSettings]). The brand palette is used as
 * is; wallpaper-based dynamic color is not applied.
 */
@Composable
fun TerpoirTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    CompositionLocalProvider(LocalDark provides darkTheme) {
        MaterialTheme(
            colorScheme = if (darkTheme) TerroirDark else TerroirLight,
            typography = TerroirTypography,
            shapes = TerroirShapes,
            content = content,
        )
    }
}

/**
 * Card colours that stand off the page in both themes. Light cards are
 * paler than the cream page; dark ones are lighter than the page, with a
 * hairline border, so they don't merge into it.
 */
object CardTones {
    /** A card that should stand out: sessions, the day summary. */
    val raised: Color
        @Composable get() = if (LocalDark.current) MaterialTheme.colorScheme.surfaceContainerHigh else MaterialTheme.colorScheme.surfaceContainerLowest

    /** A quieter card: notes. */
    val quiet: Color
        @Composable get() = if (LocalDark.current) MaterialTheme.colorScheme.surfaceContainer else MaterialTheme.colorScheme.surfaceContainerLow

    /** The border dark cards get; none in light. */
    val border: BorderStroke?
        @Composable get() = if (LocalDark.current) BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant) else null
}
