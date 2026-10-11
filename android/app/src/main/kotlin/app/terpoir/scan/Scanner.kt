// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What one photo yielded. The photo itself is gone by the time this exists. */
class PhotoText(val text: String?, val code: DecodedCode?)

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
     * Reads [photo] and recycles it, whatever happens: the image is never
     * kept (docs/design-decisions.md, "Rapid product entry").
     */
    suspend fun read(photo: Bitmap): PhotoText = withContext(Dispatchers.Default) {
        try {
            val code = CodeDecoder().decode(photo)
            val recognizer = textEngine().open()
            val text = try {
                recognizer?.recognize(photo)
            } finally {
                recognizer?.close()
            }
            PhotoText(text, code)
        } finally {
            photo.recycle()
        }
    }
}
