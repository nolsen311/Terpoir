// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap
import app.terpoir.core.TextPieceData
import app.terpoir.core.TextRowData
import app.terpoir.core.groupTextRows
import kotlin.math.roundToInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What one photo yielded. The photo itself is gone by the time this exists. */
class PhotoText(val text: String?, val code: DecodedCode?)

/**
 * What one receipt photo yielded, for the review screen.
 *
 * @property image a small copy of the photo, held in memory only while the
 *   user reviews it, so they can pick lines on it. Whoever holds this
 *   recycles it.
 * @property rows the printed rows, top to bottom, with boxes in [image]'s
 *   pixels; null if no engine could read.
 */
class ReceiptPhoto(val image: Bitmap, val rows: List<TextRowData>?)

/**
 * Reads photos for rapid product entry: text through the current
 * [TextEngine] and any barcode or QR code through ZXing. Everything runs on
 * the device.
 *
 * @property textEngine the engine to read text with; asked on every photo,
 *   since the user can switch engines in the Privacy Center.
 */
class Scanner(val textEngine: () -> TextEngine) {
    /**
     * Reads a label [photo] and recycles it, whatever happens: the image is
     * never kept (docs/design-decisions.md, "Rapid product entry").
     */
    suspend fun read(photo: Bitmap): PhotoText = withContext(Dispatchers.Default) {
        try {
            val code = CodeDecoder().decode(photo)
            val rows = rows(photo, TextLayout.BLOCKS)
            PhotoText(rows?.joinToString("\n") { it.text }, code)
        } finally {
            photo.recycle()
        }
    }

    /**
     * Reads a receipt [photo] and recycles it, whatever happens. Only a small
     * copy outlives this call, for the review screen to show and recycle.
     */
    suspend fun readReceipt(photo: Bitmap): ReceiptPhoto = withContext(Dispatchers.Default) {
        try {
            val view = forViewing(photo)
            val toView = view.width / photo.width.toFloat()
            val rows = rows(photo, TextLayout.COLUMN)?.map {
                fun scaled(value: Int) = (value * toView).roundToInt()
                TextRowData(it.text, scaled(it.left), scaled(it.top), scaled(it.right), scaled(it.bottom))
            }
            ReceiptPhoto(view, rows)
        } finally {
            photo.recycle()
        }
    }

    /** The printed rows in [photo], boxes in its pixels, or null if no engine could read. */
    private suspend fun rows(photo: Bitmap, layout: TextLayout): List<TextRowData>? {
        val readable = forReading(photo)
        try {
            val recognizer = textEngine().open() ?: return null
            val lines = try {
                recognizer.recognize(readable.image, layout)
            } finally {
                recognizer.close()
            }
            // Engines report blocks; the core puts pieces of one printed row back together.
            fun scaled(value: Int) = (value * readable.toPhoto).roundToInt()
            return groupTextRows(
                lines.map { TextPieceData(it.text, scaled(it.box.left), scaled(it.box.top), scaled(it.box.right), scaled(it.box.bottom)) },
            )
        } finally {
            readable.image.recycle()
        }
    }
}
