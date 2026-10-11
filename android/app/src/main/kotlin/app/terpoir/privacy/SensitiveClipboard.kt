// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Handler
import android.os.Looper
import android.os.PersistableBundle

/**
 * Copies secrets marked sensitive (Android 13+ hides them in the clipboard
 * preview and keyboard suggestions) and clears them after [CLEAR_AFTER_MS].
 */
object SensitiveClipboard {
    /** How long a copied secret stays on the clipboard. */
    const val CLEAR_AFTER_MS = 60_000L

    private const val LABEL = "Terpoir"
    private val handler = Handler(Looper.getMainLooper())

    /** Copies [text]; it is cleared after [CLEAR_AFTER_MS] unless replaced first. */
    fun copy(context: Context, text: String) {
        val clipboard = context.getSystemService(ClipboardManager::class.java)
        val clip = ClipData.newPlainText(LABEL, text)
        clip.description.extras = PersistableBundle().apply {
            // ClipDescription.EXTRA_IS_SENSITIVE (Android 13+); ignored before.
            putBoolean("android.content.extra.IS_SENSITIVE", true)
        }
        clipboard.setPrimaryClip(clip)
        handler.removeCallbacksAndMessages(null)
        handler.postDelayed({ clear(clipboard) }, CLEAR_AFTER_MS)
    }

    private fun clear(clipboard: ClipboardManager) {
        // The description is unreadable from the background (null): clear
        // anyway rather than leave a secret behind. In the foreground, leave
        // whatever the user copied since.
        val description = clipboard.primaryClipDescription
        if (description == null || description.label == LABEL) clipboard.clearPrimaryClip()
    }
}
