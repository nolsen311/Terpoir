// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.Typeface
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The foss build's bundled Tesseract loads its libraries and model, and reads printed text. */
@RunWith(AndroidJUnit4::class)
class BundledTextEngineTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    @Test
    fun readsPrintedText() = runBlocking {
        val engine = BundledTextEngine(context)
        assertEquals(EngineStatus.Ready, engine.status())
        val image = Bitmap.createBitmap(1400, 220, Bitmap.Config.ARGB_8888)
        val paint = Paint().apply {
            color = Color.BLACK
            textSize = 72f
            isAntiAlias = true
            typeface = Typeface.create(Typeface.SANS_SERIF, Typeface.BOLD)
        }
        Canvas(image).apply {
            drawColor(Color.WHITE)
            // A receipt row: the name, a wide gap, the price.
            drawText("BLUE DREAM 3.5G", 40f, 140f, paint)
            drawText("35.00", 1100f, 140f, paint)
        }
        val recognizer = engine.open()
        assertNotNull("Tesseract did not start", recognizer)
        try {
            for (layout in TextLayout.entries) {
                val lines = recognizer!!.recognize(image, layout)
                val text = lines.joinToString("\n") { it.text }
                assertTrue("$layout recognized: $text", "BLUE DREAM" in text.uppercase() && "35.00" in text)
                assertTrue("$layout boxes", lines.all { !it.box.isEmpty && Rect(0, 0, image.width, image.height).contains(it.box) })
            }
        } finally {
            recognizer?.close()
            image.recycle()
        }
    }
}
