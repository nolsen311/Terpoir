// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap
import android.graphics.Rect

/** How the text on a photo is laid out, so an engine can read it best. */
enum class TextLayout {
    /** Blocks of text anywhere, such as a product label. */
    BLOCKS,

    /** One column of printed rows, such as a receipt. */
    COLUMN,
}

/** A recognized line of text and its box on the image, in pixels. */
class TextLine(val text: String, val box: Rect)

/**
 * On-device text recognition. Implementations never touch the network and
 * never keep the image; the caller recycles it.
 */
interface TextRecognizer {
    /** The lines of text in [image], laid out as [layout]. */
    suspend fun recognize(image: Bitmap, layout: TextLayout): List<TextLine>

    /** Frees native resources. */
    fun close() = Unit
}

