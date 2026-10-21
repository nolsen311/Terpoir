// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.terpoir.Distribution
import app.terpoir.map.MapProvider

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
     * "Look up on the web": opening a saved site in the browser with a
     * strain's name filled in. Off by default: the site learns the name and
     * the user's IP address. The saved sites live in the journal.
     */
    var webLookups: Boolean
        get() = prefs.getBoolean(KEY_WEB_LOOKUPS, false)
        set(value) = prefs.edit().putBoolean(KEY_WEB_LOOKUPS, value).apply()

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
    // Off in a build without background location, whatever was stored.
    var notifyNearSpots: Boolean
        get() = Distribution.BACKGROUND_LOCATION && prefs.getBoolean(KEY_NOTIFY_NEAR, false)
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
     * The online map for picking a spot's location. Off by default: the
     * provider sees the areas viewed and the IP address.
     */
    var onlineMap: Boolean
        get() = prefs.getBoolean(KEY_ONLINE_MAP, false)
        set(value) = prefs.edit().putBoolean(KEY_ONLINE_MAP, value).apply()

    /** Whose map the online map shows. OpenFreeMap unless the user picks another. */
    var mapProvider: MapProvider
        get() = MapProvider.fromId(prefs.getString(KEY_MAP_PROVIDER, null))
        set(value) = prefs.edit().putString(KEY_MAP_PROVIDER, value.id).apply()

    /**
     * Whether the user has saved their own MapTiler key. The key itself is
     * in the journal's encrypted local metadata; this only lets the map be
     * offered while the journal is locked, and YOLO Mode choose MapTiler.
     */
    var mapTilerKeySaved: Boolean
        get() = prefs.getBoolean(KEY_MAPTILER_KEY_SAVED, false)
        set(value) = prefs.edit().putBoolean(KEY_MAPTILER_KEY_SAVED, value).apply()

    /**
     * Targeted symptoms: the session editor shows a section for rating
     * whether a session helped with a symptom. Off by default: symptoms
     * are health information.
     */
    var targetedSymptoms: Boolean
        get() = prefs.getBoolean(KEY_SYMPTOMS, false)
        set(value) = prefs.edit().putBoolean(KEY_SYMPTOMS, value).apply()

    /**
     * Insights: patterns in the user's own ratings, computed on the phone
     * when opened. Off by default: nothing is computed until the user asks.
     */
    var insights: Boolean
        get() = prefs.getBoolean(KEY_INSIGHTS, false)
        set(value) = prefs.edit().putBoolean(KEY_INSIGHTS, value).apply()

    /**
     * Purchase prompts: leaving an opted-in Re-up Spot after a stop shows
     * "Quick log?". Off by default: like "Notify me", it needs location
     * "all the time", and the OS location service holds those spots'
     * coordinates.
     */
    var purchasePrompts: Boolean
        get() = Distribution.BACKGROUND_LOCATION && prefs.getBoolean(KEY_PURCHASE_PROMPTS, false)
        set(value) = prefs.edit().putBoolean(KEY_PURCHASE_PROMPTS, value).apply()

    /**
     * Folder sync: encrypted sync files in a folder the user picks. Off by
     * default: whatever syncs that folder (a cloud drive app) stores them.
     */
    var folderSync: Boolean
        get() = folderSyncState
        set(value) {
            prefs.edit().putBoolean(KEY_FOLDER_SYNC, value).apply()
            folderSyncState = value
        }

    // Observable, so the folder card on the Sync and keys page appears as
    // soon as the toggle turns on (it is read there, not in the toggle).
    private var folderSyncState by mutableStateOf(prefs.getBoolean(KEY_FOLDER_SYNC, false))

    /**
     * The "New note" app shortcut (long-press the icon). On by default: it
     * holds no content and reveals no more than the icon does
     * (docs/design-decisions.md, "Quick capture").
     */
    var newNoteShortcut: Boolean
        get() = prefs.getBoolean(KEY_NEW_NOTE_SHORTCUT, true)
        set(value) = prefs.edit().putBoolean(KEY_NEW_NOTE_SHORTCUT, value).apply()

    /** The "New note" Quick Settings tile. Off by default. */
    var quickSettingsTile: Boolean
        get() = prefs.getBoolean(KEY_QS_TILE, false)
        set(value) = prefs.edit().putBoolean(KEY_QS_TILE, value).apply()

    /** The "New note" home-screen widget. Off by default. */
    var homeWidget: Boolean
        get() = prefs.getBoolean(KEY_HOME_WIDGET, false)
        set(value) = prefs.edit().putBoolean(KEY_HOME_WIDGET, value).apply()

    /** Whether the voice first-use card has been shown and answered. */
    var voiceIntroSeen: Boolean
        get() = prefs.getBoolean(KEY_VOICE_INTRO_SEEN, false)
        set(value) = prefs.edit().putBoolean(KEY_VOICE_INTRO_SEEN, value).apply()

    private companion object {
        const val KEY_SCREEN_PROTECTION = "screen_protection"
        const val KEY_COA_LINKS = "coa_links"
        const val KEY_WEB_LOOKUPS = "web_lookups"
        const val KEY_VOICE_FIRST = "voice_first"
        const val KEY_SHOW_NEARBY = "show_nearby_spots"
        const val KEY_VOICE_INTRO_SEEN = "voice_intro_seen"
        const val KEY_GOOGLE_TEXT = "google_text_recognition"
        const val KEY_NOTIFY_NEAR = "notify_near_spots"
        const val KEY_ALERT_NAMES = "alert_names"
        const val KEY_ALERT_DETAILS = "alert_details"
        const val KEY_CHECK_INS = "follow_up_check_ins"
        const val KEY_ONLINE_MAP = "online_map"
        const val KEY_MAP_PROVIDER = "map_provider"
        const val KEY_MAPTILER_KEY_SAVED = "maptiler_key_saved"
        const val KEY_SYMPTOMS = "targeted_symptoms"
        const val KEY_INSIGHTS = "insights"
        const val KEY_PURCHASE_PROMPTS = "purchase_prompts"
        const val KEY_FOLDER_SYNC = "folder_sync"
        const val KEY_NEW_NOTE_SHORTCUT = "new_note_shortcut"
        const val KEY_QS_TILE = "quick_settings_tile"
        const val KEY_HOME_WIDGET = "home_widget"
    }
}
