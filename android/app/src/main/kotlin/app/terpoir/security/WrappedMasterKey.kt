// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.security

/**
 * The master key as stored on disk: encrypted by the Keystore wrapping key.
 *
 * Format: `version (1) || IV length (1) || IV || ciphertext with GCM tag`.
 * Not secret on its own: it opens only through the hardware-backed key,
 * after the user authenticates.
 */
class WrappedMasterKey(val iv: ByteArray, val ciphertext: ByteArray) {
    /** Serialized form. */
    fun toBytes(): ByteArray {
        require(iv.size in 1..255)
        return byteArrayOf(VERSION, iv.size.toByte()) + iv + ciphertext
    }

    companion object {
        private const val VERSION: Byte = 1

        /** Parses [toBytes] output, or returns `null` if it is malformed. */
        fun fromBytes(bytes: ByteArray): WrappedMasterKey? {
            if (bytes.size < 3 || bytes[0] != VERSION) return null
            val ivLength = bytes[1].toInt() and 0xff
            if (ivLength == 0 || bytes.size <= 2 + ivLength) return null
            return WrappedMasterKey(
                iv = bytes.copyOfRange(2, 2 + ivLength),
                ciphertext = bytes.copyOfRange(2 + ivLength, bytes.size),
            )
        }
    }
}
