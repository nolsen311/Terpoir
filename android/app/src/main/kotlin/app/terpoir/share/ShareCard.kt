// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.share

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.text.Layout
import android.text.StaticLayout
import android.text.TextPaint
import java.io.ByteArrayOutputStream

/** One line of a share: a label and its value. */
data class ShareLine(val label: String, val value: String)

/**
 * What the user chose to share: a title, the lines, and whether the small
 * "made with Terpoir" mark is included.
 */
data class ShareText(val title: String, val lines: List<ShareLine>, val mark: String?) {
    /** The plain-text version. */
    fun plain(): String = buildString {
        appendLine(title)
        for (line in lines) appendLine("${line.label}: ${line.value}")
        mark?.let {
            appendLine()
            append(it)
        }
    }.trimEnd()
}

/** The card's look: the brand palette, fixed so shares look the same everywhere. */
private const val WIDTH = 1080
private const val PADDING = 72f
private const val BACKGROUND = 0xFFF3EFE7.toInt()
private const val PANEL = 0xFFFFFFFF.toInt()
private const val INK = 0xFF1F2A22.toInt()
private const val MUTED = 0xFF4D5A50.toInt()
private const val ACCENT = 0xFF2E5E3E.toInt()

/** Draws [share] as a PNG card, in memory. */
fun renderCard(share: ShareText): ByteArray {
    val textWidth = (WIDTH - 4 * PADDING).toInt()
    val title = layout(share.title, 64f, ACCENT, bold = true, textWidth)
    val lines = share.lines.map { layout(it.label, 34f, MUTED, bold = false, textWidth) to layout(it.value, 44f, INK, bold = false, textWidth) }
    val mark = share.mark?.let { layout(it, 30f, MUTED, bold = false, textWidth) }
    val gap = 28f
    var height = 2 * PADDING + 2 * PADDING + title.height + gap
    for ((label, value) in lines) height += label.height + value.height + gap
    if (mark != null) height += mark.height + gap
    val bitmap = Bitmap.createBitmap(WIDTH, height.toInt(), Bitmap.Config.ARGB_8888)
    try {
        val canvas = Canvas(bitmap)
        canvas.drawColor(BACKGROUND)
        val panel = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = PANEL }
        canvas.drawRoundRect(RectF(PADDING, PADDING, WIDTH - PADDING, height - PADDING), 36f, 36f, panel)
        var y = 2 * PADDING
        fun draw(text: StaticLayout) {
            canvas.save()
            canvas.translate(2 * PADDING, y)
            text.draw(canvas)
            canvas.restore()
            y += text.height
        }
        draw(title)
        y += gap
        for ((label, value) in lines) {
            draw(label)
            draw(value)
            y += gap
        }
        mark?.let(::draw)
        return ByteArrayOutputStream().use { out ->
            bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)
            out.toByteArray()
        }
    } finally {
        bitmap.recycle()
    }
}

private fun layout(text: String, size: Float, color: Int, bold: Boolean, width: Int): StaticLayout {
    val paint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply {
        textSize = size
        this.color = color
        typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
    }
    return StaticLayout.Builder.obtain(text, 0, text.length, paint, width)
        .setAlignment(Layout.Alignment.ALIGN_NORMAL)
        .build()
}
