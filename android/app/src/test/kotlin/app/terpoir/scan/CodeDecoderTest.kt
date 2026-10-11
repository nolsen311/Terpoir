// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import com.google.zxing.BarcodeFormat
import com.google.zxing.MultiFormatWriter
import com.google.zxing.common.BitMatrix
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CodeDecoderTest {
    /** A white-bordered greyscale image of [matrix]. */
    private fun luminance(matrix: BitMatrix): Triple<ByteArray, Int, Int> {
        val (w, h) = matrix.width to matrix.height
        val bytes = ByteArray(w * h) { i -> if (matrix[i % w, i / w]) 0 else -1 }
        return Triple(bytes, w, h)
    }

    private fun encode(text: String, format: BarcodeFormat, w: Int, h: Int) =
        luminance(MultiFormatWriter().encode(text, format, w, h, mapOf(com.google.zxing.EncodeHintType.MARGIN to 10)))

    @Test
    fun decodesQrCodesAsTwoDimensional() {
        val (bytes, w, h) = encode("https://lab.example/coa/BD-1042", BarcodeFormat.QR_CODE, 300, 300)
        val code = CodeDecoder().decode(bytes, w, h)!!
        assertEquals("https://lab.example/coa/BD-1042", code.text)
        assertEquals(true, code.twoDimensional)
    }

    @Test
    fun decodesBarcodesEitherWayRound() {
        val (bytes, w, h) = encode("0123456789012", BarcodeFormat.EAN_13, 400, 120)
        assertEquals("0123456789012", CodeDecoder().decode(bytes, w, h)!!.text)
        // The same barcode turned a quarter, as when the phone is held sideways.
        val turned = ByteArray(bytes.size)
        for (y in 0 until h) for (x in 0 until w) turned[x * h + (h - 1 - y)] = bytes[y * w + x]
        val code = CodeDecoder().decode(turned, h, w)!!
        assertEquals("0123456789012", code.text)
        assertEquals(false, code.twoDimensional)
    }

    @Test
    fun readsPaddedRows() {
        val (bytes, w, h) = encode("1A4060300002EE1000012345", BarcodeFormat.CODE_128, 500, 100)
        val stride = w + 16
        val padded = ByteArray(stride * h)
        for (y in 0 until h) System.arraycopy(bytes, y * w, padded, y * stride, w)
        assertEquals("1A4060300002EE1000012345", CodeDecoder().decode(padded, w, h, stride)!!.text)
    }

    @Test
    fun blankImagesDecodeToNothing() {
        assertNull(CodeDecoder().decode(ByteArray(100 * 100) { -1 }, 100, 100))
    }
}
