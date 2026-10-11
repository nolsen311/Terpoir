// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap
import com.google.zxing.BarcodeFormat
import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.MultiFormatReader
import com.google.zxing.PlanarYUVLuminanceSource
import com.google.zxing.ReaderException
import com.google.zxing.common.HybridBinarizer

/** A decoded code before the core classifies it. */
class DecodedCode(val text: String, val twoDimensional: Boolean)

/**
 * Barcode and QR decoding with ZXing, entirely on the device. Not thread
 * safe: use one decoder per thread.
 */
class CodeDecoder {
    private val reader = MultiFormatReader().apply {
        setHints(
            mapOf(
                DecodeHintType.POSSIBLE_FORMATS to FORMATS.keys.toList(),
                DecodeHintType.TRY_HARDER to true,
            ),
        )
    }

    /**
     * Decodes a greyscale image ([width] by [height], one byte per pixel,
     * rows [rowStride] bytes apart). Tries the image as is, then turned a
     * quarter, since linear barcodes are read along rows.
     */
    fun decode(luminance: ByteArray, width: Int, height: Int, rowStride: Int = width): DecodedCode? {
        val packed = if (rowStride == width) luminance else pack(luminance, width, height, rowStride)
        return attempt(packed, width, height) ?: attempt(rotate(packed, width, height), height, width)
    }

    /** Decodes a photo. */
    fun decode(bitmap: Bitmap): DecodedCode? {
        val pixels = IntArray(bitmap.width * bitmap.height)
        bitmap.getPixels(pixels, 0, bitmap.width, 0, 0, bitmap.width, bitmap.height)
        val luminance = ByteArray(pixels.size) { i ->
            val p = pixels[i]
            // Rec. 601 luma, integer approximation.
            ((((p shr 16) and 0xFF) * 77 + ((p shr 8) and 0xFF) * 150 + (p and 0xFF) * 29) shr 8).toByte()
        }
        return decode(luminance, bitmap.width, bitmap.height)
    }

    private fun attempt(data: ByteArray, width: Int, height: Int): DecodedCode? {
        val source = PlanarYUVLuminanceSource(data, width, height, 0, 0, width, height, false)
        return try {
            val result = reader.decodeWithState(BinaryBitmap(HybridBinarizer(source)))
            DecodedCode(result.text, FORMATS[result.barcodeFormat] ?: false)
        } catch (_: ReaderException) {
            null
        } finally {
            reader.reset()
        }
    }

    private companion object {
        /** Formats on product packaging, and whether each is two-dimensional. */
        val FORMATS = mapOf(
            BarcodeFormat.QR_CODE to true,
            BarcodeFormat.DATA_MATRIX to true,
            BarcodeFormat.PDF_417 to true,
            BarcodeFormat.EAN_13 to false,
            BarcodeFormat.EAN_8 to false,
            BarcodeFormat.UPC_A to false,
            BarcodeFormat.UPC_E to false,
            BarcodeFormat.CODE_128 to false,
            BarcodeFormat.CODE_39 to false,
        )

        fun pack(data: ByteArray, width: Int, height: Int, rowStride: Int): ByteArray {
            val out = ByteArray(width * height)
            for (y in 0 until height) System.arraycopy(data, y * rowStride, out, y * width, width)
            return out
        }

        /** Turns the image a quarter clockwise. */
        fun rotate(data: ByteArray, width: Int, height: Int): ByteArray {
            val out = ByteArray(data.size)
            for (y in 0 until height) {
                for (x in 0 until width) out[x * height + (height - 1 - y)] = data[y * width + x]
            }
            return out
        }
    }
}
