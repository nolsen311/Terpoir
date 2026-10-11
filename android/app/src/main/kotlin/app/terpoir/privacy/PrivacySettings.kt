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

    /**
     * The new-note button starts in voice mode (on-device recognition
     * only). On by default: keyboards see everything typed, while on-device
     * voice is seen by no one else.
     */
    var voiceFirst: Boolean
        get() = prefs.getBoolean(KEY_VOICE_FIRST, true)
        set(value) = prefs.edit().putBoolean(KEY_VOICE_FIRST, value).apply()

    /**
     * "Show me": when the journal opens, check whether you're near a saved
     * spot, with a location fix taken only then. Nothing is registered with
     * the OS. Off by default.
     */
    var showNearbySpots: Boolean
        get() = prefs.getBoolean(KEY_SHOW_NEARBY, false)
        set(value) = prefs.edit().putBoolean(KEY_SHOW_NEARBY, value).apply()

    /**
     * Google text recognition: scans are read by ML Kit through Google Play
     * services instead of the open engine (Play build only). Off by default:
     * a closed Google component, and Play services may send usage metrics.
     */
    var googleTextRecognition: Boolean
        get() = prefs.getBoolean(KEY_GOOGLE_TEXT, false)
        set(value) = prefs.edit().putBoolean(KEY_GOOGLE_TEXT, value).apply()

    /** Whether the voice first-use card has been shown and answered. */
    var voiceIntroSeen: Boolean
        get() = prefs.getBoolean(KEY_VOICE_INTRO_SEEN, false)
        set(value) = prefs.edit().putBoolean(KEY_VOICE_INTRO_SEEN, value).apply()

    private companion object {
        const val KEY_SCREEN_PROTECTION = "screen_protection"
        const val KEY_COA_LINKS = "coa_links"
        const val KEY_VOICE_FIRST = "voice_first"
        const val KEY_SHOW_NEARBY = "show_nearby_spots"
        const val KEY_VOICE_INTRO_SEEN = "voice_intro_seen"
        const val KEY_GOOGLE_TEXT = "google_text_recognition"
    }
}
