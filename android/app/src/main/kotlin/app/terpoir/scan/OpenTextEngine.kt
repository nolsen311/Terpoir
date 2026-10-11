// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.content.Context
import android.graphics.Bitmap
import com.googlecode.tesseract.android.TessBaseAPI
import java.io.File

/**
 * The text recognition engine on both builds: Tesseract
 * with the bundled English model (tessdata_fast). See android/ocr/README.md.
 */
object OpenTextEngine {
    private const val LANGUAGE = "eng"
    private const val MODEL_ASSET = "tessdata/$LANGUAGE.traineddata"

    /** Opens Tesseract, or `null` if its model could not be prepared. */
    fun open(context: Context): TextRecognizer? {
        val root = modelRoot(context) ?: return null
        val api = TessBaseAPI()
        if (!api.init(root.absolutePath, LANGUAGE, TessBaseAPI.OEM_LSTM_ONLY)) {
            api.recycle()
            return null
        }
        api.pageSegMode = TessBaseAPI.PageSegMode.PSM_AUTO
        return object : TextRecognizer {
            override suspend fun recognize(image: Bitmap): String {
                api.setImage(image)
                return try {
                    api.utF8Text.orEmpty()
                } finally {
                    api.clear()
                }
            }

            override fun close() = api.recycle()
        }
    }

    /**
     * Tesseract reads its model from a file, so the bundled asset is copied
     * once to the no-backup directory (it is not user data).
     */
    private fun modelRoot(context: Context): File? {
        val root = File(context.noBackupFilesDir, "tesseract")
        val model = File(root, MODEL_ASSET)
        val size = context.assets.openFd(MODEL_ASSET).use { it.length }
        if (model.length() != size) {
            model.parentFile?.mkdirs()
            val partial = File(model.path + ".partial")
            context.assets.open(MODEL_ASSET).use { input -> partial.outputStream().use { input.copyTo(it) } }
            if (!partial.renameTo(model)) return null
        }
        return root
    }
}
