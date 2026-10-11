// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.content.Context
import android.graphics.Bitmap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What one photo yielded. The photo itself is gone by the time this exists. */
class PhotoText(val text: String?, val code: DecodedCode?)

/**
 * Reads photos for rapid product entry: text through the open engine
 * (Tesseract) and any barcode or QR code through ZXing. Everything runs on
 * the device.
 */
class Scanner(private val context: Context) {
    /**
     * Reads [photo] and recycles it, whatever happens: the image is never
     * kept (docs/design-decisions.md, "Rapid product entry").
     */
    suspend fun read(photo: Bitmap): PhotoText = withContext(Dispatchers.Default) {
        try {
            val code = CodeDecoder().decode(photo)
            val recognizer = OpenTextEngine.open(context)
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
