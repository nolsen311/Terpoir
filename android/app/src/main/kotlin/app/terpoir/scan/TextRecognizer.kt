// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.graphics.Bitmap

/**
 * On-device text recognition. Implementations never touch the network and
 * never keep the image; the caller recycles it.
 */
interface TextRecognizer {
    /** The text in [image], one printed line per text line. */
    suspend fun recognize(image: Bitmap): String

    /** Frees native resources. */
    fun close() = Unit
}

