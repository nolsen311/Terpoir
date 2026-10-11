// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ocr.tesseract

import android.graphics.Bitmap
import com.googlecode.tesseract.android.TessBaseAPI
import java.io.Closeable
import java.io.File

/**
 * Tesseract with the English model, reading one image at a time. Not
 * thread-safe. See android/ocr/README.md.
 */
class TesseractReader private constructor(private val api: TessBaseAPI) : Closeable {
    /** The text in [image]; the image is neither kept nor recycled. */
    fun read(image: Bitmap): String {
        api.setImage(image)
        return try {
            api.utF8Text.orEmpty()
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
            api.pageSegMode = TessBaseAPI.PageSegMode.PSM_AUTO
            return TesseractReader(api)
        }
    }
}
