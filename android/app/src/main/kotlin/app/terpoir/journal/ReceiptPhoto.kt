// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.graphics.Bitmap
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChanged
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.min
import app.terpoir.R
import app.terpoir.core.TextRowData
import kotlin.math.abs
import kotlin.math.min
import kotlin.math.roundToInt

/** Where the fitted photo sits in the viewport, in pixels, and its scale from photo pixels. */
private class Fitted(val left: Float, val top: Float, val width: Float, val height: Float, val scale: Float)

private fun fit(image: Bitmap, viewport: IntSize): Fitted {
    val scale = min(viewport.width / image.width.toFloat(), viewport.height / image.height.toFloat())
    val w = image.width * scale
    val h = image.height * scale
    return Fitted((viewport.width - w) / 2, (viewport.height - h) / 2, w, h, scale)
}

/**
 * The receipt photo with a box on each row that was read: filled for rows
 * in an item card, in another colour for the shop, outlined for the rest.
 * Tapping anywhere on a row's line picks it ([onTap]).
 *
 * It zooms and pans (maintainer, 2026-10-15), so a row on a long receipt is
 * easy to tap: pinch, and drag once zoomed in, work as usual, and the
 * visible +, − and Fit buttons do the same, so no gesture is hidden. The
 * boxes zoom with the photo, so a tap lands on the row under the finger.
 * Unzoomed, a drag scrolls the page. The rows are also listed
 * ([ReceiptRowList]) for screen readers.
 *
 * @param atTop start zoomed to the photo's width, showing its top (where
 *   the shop's name is printed).
 */
@Composable
internal fun ReceiptPhotoView(
    image: Bitmap,
    rows: List<TextRowData>,
    itemRows: Set<Int>,
    shopRow: Int?,
    description: String,
    atTop: Boolean,
    onTap: (Int) -> Unit,
) {
    val bitmap = remember(image) { image.asImageBitmap() }
    val itemColor = MaterialTheme.colorScheme.primary
    val shopColor = MaterialTheme.colorScheme.tertiary
    val otherColor = MaterialTheme.colorScheme.outline
    var viewport by remember { mutableStateOf(IntSize.Zero) }
    var zoom by remember(image) { mutableStateOf(PhotoZoom()) }
    val currentZoom by rememberUpdatedState(zoom)
    val tap by rememberUpdatedState(onTap)
    val density = LocalDensity.current

    LaunchedEffect(viewport, atTop) {
        if (atTop && viewport != IntSize.Zero) {
            val f = fit(image, viewport)
            zoom = PhotoZoom.top(f.left, f.top, f.width, viewport.width.toFloat(), viewport.height.toFloat())
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        BoxWithConstraints(modifier = Modifier.fillMaxWidth()) {
            // As tall as the photo at full width, but never taller than most
            // of a phone screen: a long receipt fits, then zooms.
            val natural = maxWidth * (image.height / image.width.toFloat())
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(min(natural, MAX_PHOTO_HEIGHT.dp))
                    .clipToBounds()
                    .onSizeChanged { viewport = it }
                    .semantics { contentDescription = description }
                    .pointerInput(rows, image) {
                        detectTapGestures { offset ->
                            val f = fit(image, IntSize(size.width, size.height))
                            val z = currentZoom
                            val (_, cy) = z.toContent(offset.x, offset.y)
                            val y = (cy - f.top) / f.scale
                            // A little slack above and below, for small print.
                            val slack = with(density) { ROW_SLACK_DP.dp.toPx() } / (f.scale * z.scale)
                            rows.indices
                                .filter { y >= rows[it].top - slack && y <= rows[it].bottom + slack }
                                .minByOrNull { abs((rows[it].top + rows[it].bottom) / 2f - y) }
                                ?.let(tap)
                        }
                    }
                    .pointerInput(image) {
                        // Pinch to zoom; once zoomed in, drag to pan. Unzoomed,
                        // a one-finger drag is left to the page's scrolling.
                        awaitEachGesture {
                            awaitFirstDown(requireUnconsumed = false)
                            var moved = 0f
                            var transforming = false
                            do {
                                val event = awaitPointerEvent()
                                val fingers = event.changes.count { it.pressed }
                                val pan = event.calculatePan()
                                moved += pan.getDistance()
                                if (!transforming && (fingers >= 2 || (currentZoom.scale > 1f && moved > viewConfiguration.touchSlop))) {
                                    transforming = true
                                }
                                if (transforming) {
                                    val w = size.width.toFloat()
                                    val h = size.height.toFloat()
                                    val centroid = event.calculateCentroid(useCurrent = true)
                                    var next = currentZoom
                                    val factor = event.calculateZoom()
                                    if (factor != 1f && centroid != Offset.Unspecified) next = next.zoomed(factor, centroid.x, centroid.y, w, h)
                                    next = next.panned(pan.x, pan.y, w, h)
                                    zoom = next
                                    event.changes.forEach { if (it.positionChanged()) it.consume() }
                                }
                            } while (event.changes.any { it.pressed })
                        }
                    },
            ) {
                Canvas(modifier = Modifier.fillMaxSize()) {
                    val f = fit(image, IntSize(size.width.roundToInt(), size.height.roundToInt()))
                    withTransform({
                        translate(zoom.x, zoom.y)
                        scale(zoom.scale, zoom.scale, pivot = Offset.Zero)
                    }) {
                        drawImage(
                            bitmap,
                            dstOffset = IntOffset(f.left.roundToInt(), f.top.roundToInt()),
                            dstSize = IntSize(f.width.roundToInt(), f.height.roundToInt()),
                        )
                        val stroke = Stroke(width = 2.dp.toPx() / zoom.scale)
                        rows.forEachIndexed { index, row ->
                            val topLeft = Offset(f.left + row.left * f.scale, f.top + row.top * f.scale)
                            val box = Size((row.right - row.left) * f.scale, (row.bottom - row.top) * f.scale)
                            val color = when (index) {
                                shopRow -> shopColor
                                in itemRows -> itemColor
                                else -> otherColor
                            }
                            if (index == shopRow || index in itemRows) drawRect(color.copy(alpha = 0.25f), topLeft, box)
                            drawRect(color, topLeft, box, style = stroke)
                        }
                    }
                }
            }
        }
        // The same zoom as visible buttons.
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            fun step(factor: Float) {
                val w = viewport.width.toFloat()
                val h = viewport.height.toFloat()
                zoom = zoom.zoomed(factor, w / 2, h / 2, w, h)
            }
            val zoomOut = stringResource(R.string.receipt_zoom_out)
            val zoomIn = stringResource(R.string.receipt_zoom_in)
            OutlinedButton(onClick = { step(1 / PhotoZoom.STEP) }, enabled = zoom.scale > 1f, modifier = Modifier.semantics { contentDescription = zoomOut }) {
                Text("−")
            }
            OutlinedButton(onClick = { step(PhotoZoom.STEP) }, enabled = zoom.scale < PhotoZoom.MAX_ZOOM, modifier = Modifier.semantics { contentDescription = zoomIn }) {
                Text("+")
            }
            TextButton(onClick = { zoom = PhotoZoom() }, enabled = zoom != PhotoZoom()) { Text(stringResource(R.string.receipt_zoom_fit)) }
        }
    }
}

/**
 * The photo's rows as a list of buttons: the alternative to tapping the
 * photo, for screen readers and small print. [label] says what a row is
 * already used for, if anything; [enabled] whether it can be picked.
 */
@Composable
internal fun ReceiptRowList(
    rows: List<TextRowData>,
    indices: List<Int>,
    label: (Int) -> String?,
    enabled: (Int) -> Boolean,
    onPick: (Int) -> Unit,
) {
    Column {
        for (index in indices) {
            val text = rows[index].text
            val used = label(index)
            TextButton(onClick = { onPick(index) }, enabled = enabled(index), modifier = Modifier.fillMaxWidth()) {
                Text(
                    if (used == null) text else "$text ($used)",
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}

/** Slack above and below a row that still picks it, in dp. */
private const val ROW_SLACK_DP = 6

/** The photo's tallest, in dp. */
private const val MAX_PHOTO_HEIGHT = 480
