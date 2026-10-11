// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ocr

import android.graphics.Bitmap
import app.terpoir.ocr.tesseract.TesseractReader
import app.terpoir.scan.TextEngineModule
import app.terpoir.scan.TextRecognizer
import java.io.File

/** The module's entry point: Tesseract, behind the scanner's interface. */
class TesseractModule : TextEngineModule {
    override fun open(dataDir: File): TextRecognizer? {
        val reader = TesseractReader.open(dataDir) ?: return null
        return object : TextRecognizer {
            override suspend fun recognize(image: Bitmap): String = reader.read(image)

            override fun close() = reader.close()
        }
    }
}
