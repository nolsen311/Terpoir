// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ui.theme

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

/*
 * The Terroir palette (docs/design-decisions.md, "Visual design"): moss
 * green, sage and amber resin on linen by day; the same hues lifted for a
 * forest-floor dark by night. Every text color meets 4.5:1 on the
 * backgrounds it sits on (ThemeContrastTest).
 */

/** Terroir, light. */
val TerroirLight: ColorScheme = lightColorScheme(
    primary = Color(0xFF2E5E3E),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFCDE8D2),
    onPrimaryContainer = Color(0xFF0F2E19),
    inversePrimary = Color(0xFF9DD3AB),
    secondary = Color(0xFF5B5E47),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFE4E0CF),
    onSecondaryContainer = Color(0xFF2F2B1A),
    tertiary = Color(0xFF7A500F),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFFFDDB0),
    onTertiaryContainer = Color(0xFF2C1800),
    background = Color(0xFFF3EFE7),
    onBackground = Color(0xFF1F2A22),
    surface = Color(0xFFF3EFE7),
    onSurface = Color(0xFF1F2A22),
    surfaceVariant = Color(0xFFE1DCCF),
    onSurfaceVariant = Color(0xFF4D5A50),
    surfaceTint = Color(0xFF2E5E3E),
    inverseSurface = Color(0xFF313830),
    inverseOnSurface = Color(0xFFEEF1EA),
    error = Color(0xFFA91717),
    onError = Color(0xFFFFFFFF),
    errorContainer = Color(0xFFFFDAD6),
    onErrorContainer = Color(0xFF410002),
    outline = Color(0xFF7A867C),
    outlineVariant = Color(0xFFC9C2B4),
    scrim = Color(0xFF000000),
    surfaceBright = Color(0xFFF8F4EC),
    surfaceDim = Color(0xFFDCD6C8),
    surfaceContainerLowest = Color(0xFFFFFFFF),
    surfaceContainerLow = Color(0xFFEEEADF),
    surfaceContainer = Color(0xFFE8E2D6),
    surfaceContainerHigh = Color(0xFFE2DCCF),
    surfaceContainerHighest = Color(0xFFDCD6C8),
)

/** Terroir, dark. */
val TerroirDark: ColorScheme = darkColorScheme(
    primary = Color(0xFF9DD3AB),
    onPrimary = Color(0xFF00391C),
    primaryContainer = Color(0xFF15462A),
    onPrimaryContainer = Color(0xFFB9EFC6),
    inversePrimary = Color(0xFF2E5E3E),
    secondary = Color(0xFFC8C6B0),
    onSecondary = Color(0xFF31311F),
    secondaryContainer = Color(0xFF3A3829),
    onSecondaryContainer = Color(0xFFE6E1CB),
    tertiary = Color(0xFFF2BE6E),
    onTertiary = Color(0xFF452B00),
    tertiaryContainer = Color(0xFF63420A),
    onTertiaryContainer = Color(0xFFFFDDB0),
    background = Color(0xFF141A15),
    onBackground = Color(0xFFE2E6DE),
    surface = Color(0xFF141A15),
    onSurface = Color(0xFFE2E6DE),
    surfaceVariant = Color(0xFF3F4840),
    onSurfaceVariant = Color(0xFFBEC8BD),
    surfaceTint = Color(0xFF9DD3AB),
    inverseSurface = Color(0xFFE2E6DE),
    inverseOnSurface = Color(0xFF2E352F),
    error = Color(0xFFFFB4AB),
    onError = Color(0xFF690005),
    errorContainer = Color(0xFF93000A),
    onErrorContainer = Color(0xFFFFDAD6),
    outline = Color(0xFF889287),
    outlineVariant = Color(0xFF3F4840),
    scrim = Color(0xFF000000),
    surfaceBright = Color(0xFF3A413A),
    surfaceDim = Color(0xFF141A15),
    surfaceContainerLowest = Color(0xFF0F140F),
    surfaceContainerLow = Color(0xFF1A201A),
    surfaceContainer = Color(0xFF1E251F),
    surfaceContainerHigh = Color(0xFF283029),
    surfaceContainerHighest = Color(0xFF333B33),
)
