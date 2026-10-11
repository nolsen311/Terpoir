// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.content.Context
import android.graphics.Bitmap
import app.terpoir.R
import app.terpoir.ocr.tesseract.TesseractReader
import java.io.File
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flowOf

/**
 * The foss build's engine: Tesseract and its English model, bundled in the
 * app (F-Droid has no on-demand delivery, and the app never downloads code).
 * Always ready.
 */
class BundledTextEngine(private val context: Context) : TextEngine {
    // Never shown: the engine is bundled.
    override val downloadPrompt: Int = R.string.ocr_download_tesseract

    override suspend fun status(): EngineStatus = EngineStatus.Ready

    override fun install(): Flow<EngineStatus> = flowOf(EngineStatus.Ready)

    override suspend fun open(): TextRecognizer? {
        val reader = modelRoot()?.let(TesseractReader::open) ?: return null
        return object : TextRecognizer {
            override suspend fun recognize(image: Bitmap): String = reader.read(image)

            override fun close() = reader.close()
        }
    }

    /**
     * Tesseract reads its model from a file, so the bundled asset is copied
     * once to the no-backup directory (it is not user data).
     */
    private fun modelRoot(): File? {
        val asset = TesseractReader.MODEL_PATH
        val root = File(context.noBackupFilesDir, "tesseract")
        val model = File(root, asset)
        val size = context.assets.openFd(asset).use { it.length }
        if (model.length() != size) {
            model.parentFile?.mkdirs()
            val partial = File(model.path + ".partial")
            context.assets.open(asset).use { input -> partial.outputStream().use { input.copyTo(it) } }
            if (!partial.renameTo(model)) return null
        }
        return root
    }
}
