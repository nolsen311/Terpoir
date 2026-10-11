// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ui.theme

import androidx.compose.material3.Typography
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import app.terpoir.R

/*
 * Fonts are bundled (res/font, provenance in android/fonts/README.md), never
 * downloaded: downloadable Google Fonts go through Play services, which the
 * FOSS flavor does not have and which would contact Google.
 *
 * Both are variable fonts; each weight sets its axis explicitly (Fraunces's
 * default instance is Black).
 */

private fun fraunces(weight: FontWeight) = Font(
    R.font.fraunces,
    weight = weight,
    variationSettings = FontVariation.Settings(FontVariation.weight(weight.weight)),
)

private fun workSans(weight: FontWeight) = Font(
    R.font.work_sans,
    weight = weight,
    variationSettings = FontVariation.Settings(FontVariation.weight(weight.weight)),
)

/** Display, headline and title: Fraunces, a soft serif. */
val Fraunces = FontFamily(fraunces(FontWeight.Medium), fraunces(FontWeight.SemiBold))

/** Body and labels: Work Sans. */
val WorkSans = FontFamily(workSans(FontWeight.Normal), workSans(FontWeight.Medium), workSans(FontWeight.SemiBold))

private val defaults = Typography()

/** The Terroir type scale: Material 3 sizes, Fraunces above Work Sans. */
val TerroirTypography = Typography(
    displayLarge = defaults.displayLarge.serif(FontWeight.SemiBold),
    displayMedium = defaults.displayMedium.serif(FontWeight.SemiBold),
    displaySmall = defaults.displaySmall.serif(FontWeight.SemiBold),
    headlineLarge = defaults.headlineLarge.serif(FontWeight.SemiBold),
    headlineMedium = defaults.headlineMedium.serif(FontWeight.SemiBold),
    headlineSmall = defaults.headlineSmall.serif(FontWeight.SemiBold),
    titleLarge = defaults.titleLarge.serif(FontWeight.Medium),
    titleMedium = defaults.titleMedium.sans(FontWeight.SemiBold),
    titleSmall = defaults.titleSmall.sans(FontWeight.SemiBold),
    bodyLarge = defaults.bodyLarge.sans(FontWeight.Normal),
    bodyMedium = defaults.bodyMedium.sans(FontWeight.Normal),
    bodySmall = defaults.bodySmall.sans(FontWeight.Normal),
    labelLarge = defaults.labelLarge.sans(FontWeight.Medium),
    labelMedium = defaults.labelMedium.sans(FontWeight.Medium),
    labelSmall = defaults.labelSmall.sans(FontWeight.Medium),
)

private fun TextStyle.serif(weight: FontWeight) =
    copy(fontFamily = Fraunces, fontWeight = weight, letterSpacing = (-0.25).sp)

private fun TextStyle.sans(weight: FontWeight) = copy(fontFamily = WorkSans, fontWeight = weight)
