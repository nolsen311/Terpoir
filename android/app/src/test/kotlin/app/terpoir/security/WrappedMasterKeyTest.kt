// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.security

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WrappedMasterKeyTest {
    @Test
    fun roundTrip() {
        val iv = ByteArray(12) { it.toByte() }
        val ciphertext = ByteArray(48) { (it * 3).toByte() }
        val parsed = WrappedMasterKey.fromBytes(WrappedMasterKey(iv, ciphertext).toBytes())!!
        assertArrayEquals(iv, parsed.iv)
        assertArrayEquals(ciphertext, parsed.ciphertext)
    }

    @Test
    fun rejectsMalformed() {
        assertNull(WrappedMasterKey.fromBytes(byteArrayOf()))
        assertNull(WrappedMasterKey.fromBytes(byteArrayOf(2, 12, 0, 0)))
        assertNull(WrappedMasterKey.fromBytes(byteArrayOf(1, 0, 5)))
        assertNull(WrappedMasterKey.fromBytes(byteArrayOf(1, 12) + ByteArray(12)))
    }
}
