// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.ColorMatrix
import android.graphics.ColorMatrixColorFilter
import android.graphics.Paint
import android.graphics.Rect
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

/** Longest side, in pixels, of the image a recognizer reads. */
private const val MAX_READ_SIDE = 4000

/** Longest side, in pixels, of the copy shown while the user reviews a receipt. */
private const val MAX_VIEW_SIDE = 1600

/** Roughly how many samples per side [levels] takes. */
private const val SAMPLES_PER_SIDE = 400

/** The narrowest brightness range worth stretching; flatter images are left alone. */
private const val MIN_RANGE = 32

/** An image to read and the factor from its pixels back to the photo's. */
internal class ReadableImage(val image: Bitmap, val toPhoto: Float)

/**
 * A copy of [photo] prepared for text recognition: no larger than
 * [MAX_READ_SIDE], gray, and with its contrast stretched so the paper is
 * near white and the ink near black. Camera photos of receipts are often
 * dim, tinted or low in contrast.
 */
internal fun forReading(photo: Bitmap): ReadableImage {
    val scale = min(1f, MAX_READ_SIDE / max(photo.width, photo.height).toFloat())
    val width = (photo.width * scale).roundToInt().coerceAtLeast(1)
    val height = (photo.height * scale).roundToInt().coerceAtLeast(1)
    val (low, high) = levels(photo)
    val stretch = 255f / (high - low)
    val offset = -low * stretch
    // Luminance (BT.601) into each colour channel, then the stretch.
    val r = 0.299f * stretch
    val g = 0.587f * stretch
    val b = 0.114f * stretch
    val matrix = ColorMatrix(
        floatArrayOf(
            r, g, b, 0f, offset,
            r, g, b, 0f, offset,
            r, g, b, 0f, offset,
            0f, 0f, 0f, 1f, 0f,
        ),
    )
    val out = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
    val paint = Paint(Paint.FILTER_BITMAP_FLAG).apply { colorFilter = ColorMatrixColorFilter(matrix) }
    Canvas(out).drawBitmap(photo, null, Rect(0, 0, width, height), paint)
    return ReadableImage(out, toPhoto = photo.width / width.toFloat())
}

/** A small copy of [photo] to show on screen, no larger than [MAX_VIEW_SIDE]. */
internal fun forViewing(photo: Bitmap): Bitmap {
    val scale = min(1f, MAX_VIEW_SIDE / max(photo.width, photo.height).toFloat())
    val width = (photo.width * scale).roundToInt().coerceAtLeast(1)
    val height = (photo.height * scale).roundToInt().coerceAtLeast(1)
    val scaled = Bitmap.createScaledBitmap(photo, width, height, true)
    // Always a copy: the caller recycles the photo.
    return if (scaled === photo) photo.copy(Bitmap.Config.ARGB_8888, false) else scaled
}

/**
 * The darkest and brightest luminance worth keeping: the 1st and 99th
 * percentiles of a grid of samples, so a few specks or glints do not set
 * the range. `0 to 255` when the range is too flat to stretch.
 */
private fun levels(photo: Bitmap): Pair<Int, Int> {
    val step = max(1, max(photo.width, photo.height) / SAMPLES_PER_SIDE)
    val histogram = IntArray(256)
    var count = 0
    val row = IntArray(photo.width)
    for (y in 0 until photo.height step step) {
        photo.getPixels(row, 0, photo.width, 0, y, photo.width, 1)
        for (x in 0 until photo.width step step) {
            val c = row[x]
            val luminance = (299 * (c shr 16 and 0xff) + 587 * (c shr 8 and 0xff) + 114 * (c and 0xff)) / 1000
            histogram[luminance]++
            count++
        }
    }
    fun percentile(p: Int): Int {
        val target = count.toLong() * p / 100
        var seen = 0L
        for (level in 0..255) {
            seen += histogram[level]
            if (seen > target) return level
        }
        return 255
    }
    val low = percentile(1)
    val high = percentile(99)
    return if (high - low < MIN_RANGE) 0 to 255 else low to high
}
