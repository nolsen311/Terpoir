// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ocr.tesseract

import android.graphics.Bitmap
import android.graphics.Rect
import com.googlecode.tesseract.android.TessBaseAPI
import com.googlecode.tesseract.android.TessBaseAPI.PageIteratorLevel.RIL_TEXTLINE
import java.io.Closeable
import java.io.File

/**
 * Tesseract with the English model, reading one image at a time. Not
 * thread-safe. See android/ocr/README.md.
 */
class TesseractReader private constructor(private val api: TessBaseAPI) : Closeable {
    /** A recognized line and its box on the image, in pixels. */
    class Line(val text: String, val box: Rect)

    /**
     * The lines of text in [image]; the image is neither kept nor recycled.
     *
     * @param column true for one column of printed rows (a receipt): rows are
     *   read whole instead of as separate blocks, and the image is binarized
     *   locally (Sauvola), which copes with shadows and curled paper.
     */
    fun read(image: Bitmap, column: Boolean = false): List<Line> {
        api.pageSegMode = if (column) TessBaseAPI.PageSegMode.PSM_SINGLE_COLUMN else TessBaseAPI.PageSegMode.PSM_AUTO
        api.setVariable(THRESHOLDING, if (column) SAUVOLA else OTSU)
        api.setImage(image)
        return try {
            // Runs recognition; the iterator walks its result.
            api.utF8Text
            val lines = api.resultIterator ?: return emptyList()
            try {
                lines.begin()
                buildList {
                    do {
                        val text = lines.getUTF8Text(RIL_TEXTLINE)?.trim()
                        val box = lines.getBoundingRect(RIL_TEXTLINE)
                        if (!text.isNullOrEmpty() && box != null) add(Line(text, box))
                    } while (lines.next(RIL_TEXTLINE))
                }
            } finally {
                lines.delete()
            }
        } finally {
            api.clear()
        }
    }

    override fun close() = api.recycle()

    companion object {
        /** The model's language, and the name of its file in `tessdata/`. */
        const val LANGUAGE = "eng"

        /** Where the model lives, relative to the data directory. */
        const val MODEL_PATH = "tessdata/$LANGUAGE.traineddata"

        private const val THRESHOLDING = "thresholding_method"
        private const val OTSU = "0"
        private const val SAUVOLA = "2"

        /**
         * Opens Tesseract on [dataDir], which holds [MODEL_PATH], or returns
         * `null` if it would not start.
         */
        fun open(dataDir: File): TesseractReader? {
            val api = TessBaseAPI()
            if (!api.init(dataDir.absolutePath, LANGUAGE, TessBaseAPI.OEM_LSTM_ONLY)) {
                api.recycle()
                return null
            }
            return TesseractReader(api)
        }
    }
}
