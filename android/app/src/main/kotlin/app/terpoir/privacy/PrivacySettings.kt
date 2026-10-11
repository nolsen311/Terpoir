// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.content.Context

/**
 * Privacy toggles that must be readable before unlock. Not journal
 * content, so they live outside the encrypted store; app data is excluded
 * from backups. Every default is the most private option.
 */
class PrivacySettings(context: Context) {
    private val prefs = context.getSharedPreferences("privacy", Context.MODE_PRIVATE)

    /**
     * Screen protection (FLAG_SECURE): blocks screenshots, screen recording
     * and casting, and blanks the app in Recents. On by default.
     */
    var screenProtection: Boolean
        get() = prefs.getBoolean(KEY_SCREEN_PROTECTION, true)
        set(value) = prefs.edit().putBoolean(KEY_SCREEN_PROTECTION, value).apply()

    /**
     * Opening a scanned lab-results (COA) link in the browser. Off by
     * default: the lab's site learns the user's IP address and which batch
     * they bought.
     */
    var coaLinks: Boolean
        get() = prefs.getBoolean(KEY_COA_LINKS, false)
        set(value) = prefs.edit().putBoolean(KEY_COA_LINKS, value).apply()

    private companion object {
        const val KEY_SCREEN_PROTECTION = "screen_protection"
        const val KEY_COA_LINKS = "coa_links"
    }
}
