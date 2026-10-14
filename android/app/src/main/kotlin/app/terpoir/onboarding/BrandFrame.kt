// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import android.content.Context
import android.provider.Settings
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.lerp
import androidx.compose.ui.unit.sp
import app.terpoir.R
import app.terpoir.ui.theme.WorkSans
import app.terpoir.ui.theme.Wordmark
import kotlinx.coroutines.delay

/** The launcher icon's background, behind the brand in every theme. */
private val BrandGreen = Color(0xFF2E4A3B)
private val Paper = Color(0xFFE8E2D0)
private val PaperSoft = Color(0xFFC9C2AE)

/** Whether the intro has played in this process; it plays once per launch. */
private var introPlayed = false

private const val HOLD_MILLIS = 900L
private const val SLIDE_MILLIS = 700
private val HeaderHeight = 88.dp

/** The header's lockup is this fraction of the intro's. */
private const val HEADER_SCALE = 0.5f

/**
 * Onboarding's frame: the brand (the mark, "Terpoir" and the tagline, as in
 * the store's feature graphic) on the icon's green. The first time it
 * appears after a launch, it fills the screen, then slides up into a header
 * band while the page below fades in. Later pages show the header straight
 * away, and with animations turned off in system settings there is no
 * intro.
 */
@Composable
fun BrandFrame(content: @Composable () -> Unit) {
    val context = LocalContext.current
    val progress = remember { Animatable(if (introPlayed || !animationsOn(context)) 1f else 0f) }
    LaunchedEffect(Unit) {
        introPlayed = true
        if (progress.value < 1f) {
            delay(HOLD_MILLIS)
            progress.animateTo(1f, tween(SLIDE_MILLIS, easing = FastOutSlowInEasing))
        }
    }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val p = progress.value
        val full = maxHeight
        Column(Modifier.fillMaxSize()) {
            Box(
                Modifier.fillMaxWidth().height(lerp(full, HeaderHeight, p)).background(BrandGreen),
                contentAlignment = Alignment.Center,
            ) {
                Lockup(scale = 1f - (1f - HEADER_SCALE) * p, taglineAlpha = (1f - 2f * p).coerceIn(0f, 1f))
            }
            // The page fades in over the slide's second half.
            Box(Modifier.weight(1f).fillMaxWidth().alpha((2f * p - 1f).coerceIn(0f, 1f))) { content() }
        }
    }
}

/**
 * The mark sitting on the wordmark's baseline with its bubbles rising above,
 * and the tagline below (faded out in the header).
 */
@Composable
private fun Lockup(scale: Float, taglineAlpha: Float) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.graphicsLayer {
            scaleX = scale
            scaleY = scale
        },
    ) {
        Row {
            // Sized so the pages (24.5 of the mark's 48 units) match the
            // wordmark's capitals (about 0.7 of its size), as in the
            // feature graphic.
            Image(
                painterResource(R.drawable.logo_mark),
                contentDescription = null,
                modifier = Modifier.size(width = 69.dp, height = 72.dp).alignBy { it.measuredHeight },
            )
            Spacer(Modifier.width(14.dp))
            Text(
                stringResource(R.string.app_name),
                fontFamily = Wordmark,
                // A logo: fixed in dp, so a large font setting can't push
                // it off the screen. The tagline does scale.
                fontSize = with(LocalDensity.current) { 52.dp.toSp() },
                color = Paper,
                modifier = Modifier.alignByBaseline().semantics { heading() },
            )
        }
        if (taglineAlpha > 0f) {
            Text(
                stringResource(R.string.brand_tagline),
                fontFamily = WorkSans,
                fontSize = 18.sp,
                color = PaperSoft,
                modifier = Modifier.padding(top = 16.dp).alpha(taglineAlpha),
            )
        }
    }
}

/** False when the user turned animations off (animator duration scale 0). */
private fun animationsOn(context: Context): Boolean =
    Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) != 0f
