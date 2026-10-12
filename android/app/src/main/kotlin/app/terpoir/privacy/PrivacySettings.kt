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

    /**
     * "Notify me": geofences around saved spots, with an alert when the
     * phone stops at one. Off by default: it needs location "all the time",
     * and the OS location service holds the spots' coordinates.
     */
    var notifyNearSpots: Boolean
        get() = prefs.getBoolean(KEY_NOTIFY_NEAR, false)
        set(value) = prefs.edit().putBoolean(KEY_NOTIFY_NEAR, value).apply()

    /**
     * Alerts name the spot (Named tier). Off by default: names are then kept
     * in a small notification cache outside the encrypted journal.
     */
    var alertNames: Boolean
        get() = prefs.getBoolean(KEY_ALERT_NAMES, false)
        set(value) = prefs.edit().putBoolean(KEY_ALERT_NAMES, value).apply()

    /**
     * Alerts also show ratings, session count and last product (Rich tier).
     * Off by default, for the same reason, with more in the cache.
     */
    var alertDetails: Boolean
        get() = prefs.getBoolean(KEY_ALERT_DETAILS, false)
        set(value) = prefs.edit().putBoolean(KEY_ALERT_DETAILS, value).apply()

    /**
     * Follow-up check-ins: the session editor offers a reminder some time
     * after a session. Off by default: the OS scheduler holds the time.
     */
    var followUpCheckIns: Boolean
        get() = prefs.getBoolean(KEY_CHECK_INS, false)
        set(value) = prefs.edit().putBoolean(KEY_CHECK_INS, value).apply()

    /**
     * The online map for picking a spot's location (builds with a MapTiler
     * key only). Off by default: MapTiler sees the areas viewed and the IP
     * address.
     */
    var onlineMap: Boolean
        get() = prefs.getBoolean(KEY_ONLINE_MAP, false)
        set(value) = prefs.edit().putBoolean(KEY_ONLINE_MAP, value).apply()

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
        const val KEY_NOTIFY_NEAR = "notify_near_spots"
        const val KEY_ALERT_NAMES = "alert_names"
        const val KEY_ALERT_DETAILS = "alert_details"
        const val KEY_CHECK_INS = "follow_up_check_ins"
        const val KEY_ONLINE_MAP = "online_map"
    }
}
