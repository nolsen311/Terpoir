// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.content.Context
import app.terpoir.Capability
import app.terpoir.Distribution
import app.terpoir.R
import app.terpoir.alerts.CheckIns
import app.terpoir.alerts.SpotAlerts
import app.terpoir.capture.NewNoteWidget
import app.terpoir.capture.QuickCapture
import app.terpoir.core.Journal
import app.terpoir.map.MapProvider
import app.terpoir.map.OnlineMap
import app.terpoir.sync.FolderSync
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Builds the Privacy Center with every toggle this build has. */
fun buildPrivacyCenter(
    context: Context,
    settings: PrivacySettings,
    alerts: SpotAlerts,
    checkIns: CheckIns,
    folderSync: FolderSync,
    journal: () -> Journal?,
): PrivacyCenter = PrivacyCenter(
    listOfNotNull(
        // Quick capture entry points (docs/design-decisions.md, "Quick
        // capture"): each enables its component on, and disables it off,
        // which takes the tile out of Quick Settings and the widget out of
        // the widget list. Neither shows journal content.
        PrivacyToggle(
            id = "quick_settings_tile",
            group = PrivacyGroup.OS_INTEGRATION,
            card = PrivacyCard(
                title = R.string.capture_tile,
                whatItDoes = R.string.capture_tile_does,
                whatLeaves = R.string.capture_tile_leaves,
                whoCanSee = R.string.capture_tile_who,
                howToUndo = R.string.capture_tile_undo,
            ),
            privateState = false,
            isOn = { settings.quickSettingsTile },
            onAction = {
                settings.quickSettingsTile = true
                QuickCapture.setTiles(context, true)
            },
            offAction = {
                settings.quickSettingsTile = false
                QuickCapture.setTiles(context, false)
            },
        ),
        PrivacyToggle(
            id = "home_widget",
            group = PrivacyGroup.OS_INTEGRATION,
            card = PrivacyCard(
                title = R.string.capture_widget,
                whatItDoes = R.string.capture_widget_does,
                whatLeaves = R.string.capture_widget_leaves,
                whoCanSee = R.string.capture_widget_who,
                howToUndo = R.string.capture_widget_undo,
            ),
            privateState = false,
            isOn = { settings.homeWidget },
            onAction = {
                settings.homeWidget = true
                QuickCapture.setComponent(context, NewNoteWidget::class.java, true)
            },
            offAction = {
                settings.homeWidget = false
                QuickCapture.setComponent(context, NewNoteWidget::class.java, false)
            },
        ),
        // Applied to the window on the next resume, and at once by the
        // Privacy Center screen. Off only clears FLAG_SECURE.
        PrivacyToggle(
            id = "screen_protection",
            group = PrivacyGroup.DEVICE_SECURITY,
            card = PrivacyCard(
                title = R.string.screen_protection,
                whatItDoes = R.string.screen_protection_does,
                whatLeaves = R.string.screen_protection_leaves,
                whoCanSee = R.string.screen_protection_who,
                howToUndo = R.string.screen_protection_undo,
            ),
            privateState = true,
            isOn = { settings.screenProtection },
            onAction = { settings.screenProtection = true },
            offAction = { settings.screenProtection = false },
        ),
        // Only shows or hides the session editor's section; nothing leaves
        // the journal. Turned off by hand, it asks whether to keep what was
        // recorded (hidden) or delete it.
        PrivacyToggle(
            id = "targeted_symptoms",
            group = PrivacyGroup.JOURNAL,
            card = PrivacyCard(
                title = R.string.symptoms,
                whatItDoes = R.string.symptoms_does,
                whatLeaves = R.string.symptoms_leaves,
                whoCanSee = R.string.symptoms_who,
                howToUndo = R.string.symptoms_undo,
            ),
            privateState = false,
            isOn = { settings.targetedSymptoms },
            onAction = { settings.targetedSymptoms = true },
            offAction = { settings.targetedSymptoms = false },
            offChoice = OffChoice(
                question = R.string.symptoms_off_question,
                hideLabel = R.string.symptoms_off_hide,
                deleteLabel = R.string.symptoms_off_delete,
                delete = {
                    withContext(Dispatchers.IO) { journal()?.deleteSymptoms(System.currentTimeMillis().toULong()) }
                },
            ),
        ),
        // On only shows the journal's Insights entry; insights are computed
        // in the core when opened and never stored. Off forgets what the
        // user dismissed, marked wrong or muted.
        PrivacyToggle(
            id = "insights",
            group = PrivacyGroup.JOURNAL,
            card = PrivacyCard(
                title = R.string.insights_toggle,
                whatItDoes = R.string.insights_does,
                whatLeaves = R.string.insights_leaves,
                whoCanSee = R.string.insights_who,
                howToUndo = R.string.insights_undo,
            ),
            privateState = false,
            isOn = { settings.insights },
            onAction = { settings.insights = true },
            offAction = {
                settings.insights = false
                withContext(Dispatchers.IO) { journal()?.forgetInsightFeedback(System.currentTimeMillis().toULong()) }
            },
        ),
        // A fix is taken only when the journal opens and only compared with
        // spots inside the core; nothing is registered with the OS, so off
        // has nothing to clean up.
        PrivacyToggle(
            id = "show_nearby_spots",
            group = PrivacyGroup.LOCATION,
            card = PrivacyCard(
                title = R.string.show_nearby,
                whatItDoes = R.string.show_nearby_does,
                whatLeaves = R.string.show_nearby_leaves,
                whoCanSee = R.string.show_nearby_who,
                howToUndo = R.string.show_nearby_undo,
            ),
            privateState = false,
            isOn = { settings.showNearbySpots },
            onAction = { settings.showNearbySpots = true },
            offAction = { settings.showNearbySpots = false },
        ),
        // Every build. On lets the spot editor open the map, with MapTiler
        // if the user saved their own key, else OpenFreeMap; YOLO Mode
        // chooses Google Maps instead on Play builds that offer it. The
        // gateway refuses every request while it is off. Off also deletes
        // anything MapLibre stored and returns the provider to OpenFreeMap;
        // a saved MapTiler key stays until the user removes it. Google is
        // otherwise only ever an active choice (onboarding, the Map
        // provider card).
        PrivacyToggle(
            id = ONLINE_MAP,
            group = PrivacyGroup.LOCATION,
            card = PrivacyCard(
                title = R.string.online_map,
                whatItDoes = R.string.online_map_does,
                whatLeaves = R.string.online_map_leaves,
                whoCanSee = R.string.online_map_who,
                howToUndo = R.string.online_map_undo,
            ),
            privateState = false,
            isOn = { settings.onlineMap },
            onAction = {
                settings.mapProvider = if (settings.mapTilerKeySaved) MapProvider.MAPTILER else MapProvider.DEFAULT
                settings.onlineMap = true
            },
            offAction = {
                settings.onlineMap = false
                settings.mapProvider = MapProvider.DEFAULT
                OnlineMap.forget(context)
            },
            presetOnAction = { presetId ->
                settings.mapProvider = when {
                    presetId == YOLO_MODE.id && Distribution.googleMaps(context) != null -> MapProvider.GOOGLE
                    settings.mapTilerKeySaved -> MapProvider.MAPTILER
                    else -> MapProvider.DEFAULT
                }
                settings.onlineMap = true
            },
        ),
        // This toggle and the next three exist only in builds with
        // background location (foss; left out of Play, 2026-10-13).
        // On registers geofences now if the journal is open (else at the
        // next unlock) and once the permissions are granted, which the
        // journal asks for. Off removes every geofence, the notification
        // cache and any alert showing.
        if (Distribution.BACKGROUND_LOCATION) {
            PrivacyToggle(
                id = "notify_near_spots",
                group = PrivacyGroup.LOCATION,
                card = PrivacyCard(
                    title = R.string.notify_near,
                    whatItDoes = R.string.notify_near_does,
                    whatLeaves = R.string.notify_near_leaves,
                    whoCanSee = R.string.notify_near_who,
                    howToUndo = R.string.notify_near_undo,
                ),
                privateState = false,
                isOn = { settings.notifyNearSpots },
                onAction = {
                    settings.notifyNearSpots = true
                    alerts.refresh(journal())
                },
                offAction = {
                    settings.notifyNearSpots = false
                    alerts.clear()
                },
            )
        } else {
            null
        },
        // On lets Re-up Spots opt in (spot editor) and registers their fences
        // with the next refresh, once the journal asks for the permissions.
        // Off removes the prompt fences and any pending state.
        if (Distribution.BACKGROUND_LOCATION) {
            PrivacyToggle(
                id = "purchase_prompts",
                group = PrivacyGroup.LOCATION,
                card = PrivacyCard(
                    title = R.string.purchase_prompts,
                    whatItDoes = R.string.purchase_prompts_does,
                    whatLeaves = R.string.purchase_prompts_leaves,
                    whoCanSee = R.string.purchase_prompts_who,
                    howToUndo = R.string.purchase_prompts_undo,
                ),
                privateState = false,
                isOn = { settings.purchasePrompts },
                onAction = {
                    settings.purchasePrompts = true
                    alerts.refresh(journal())
                },
                offAction = {
                    settings.purchasePrompts = false
                    alerts.refresh(journal())
                },
            )
        } else {
            null
        },
        // The Named tier. On writes spot names to the notification cache
        // (now if the journal is open, else at the next unlock); off drops
        // them, unless the Rich tier still shows them.
        if (Distribution.BACKGROUND_LOCATION) {
            PrivacyToggle(
                id = "alert_names",
                group = PrivacyGroup.NOTIFICATIONS,
                card = PrivacyCard(
                    title = R.string.alert_names,
                    whatItDoes = R.string.alert_names_does,
                    whatLeaves = R.string.alert_names_leaves,
                    whoCanSee = R.string.alert_names_who,
                    howToUndo = R.string.alert_names_undo,
                ),
                privateState = false,
                isOn = { settings.alertNames },
                onAction = {
                    settings.alertNames = true
                    alerts.refresh(journal())
                },
                offAction = {
                    settings.alertNames = false
                    alerts.narrowCache()
                },
            )
        } else {
            null
        },
        // The Rich tier: as above, with session details in the cache.
        if (Distribution.BACKGROUND_LOCATION) {
            PrivacyToggle(
                id = "alert_details",
                group = PrivacyGroup.NOTIFICATIONS,
                card = PrivacyCard(
                    title = R.string.alert_details,
                    whatItDoes = R.string.alert_details_does,
                    whatLeaves = R.string.alert_details_leaves,
                    whoCanSee = R.string.alert_details_who,
                    howToUndo = R.string.alert_details_undo,
                ),
                privateState = false,
                isOn = { settings.alertDetails },
                onAction = {
                    settings.alertDetails = true
                    alerts.refresh(journal())
                },
                offAction = {
                    settings.alertDetails = false
                    alerts.narrowCache()
                },
            )
        } else {
            null
        },
        // On only lets the session editor offer a check-in. Off cancels the
        // pending one and clears a showing reminder.
        PrivacyToggle(
            id = "follow_up_check_ins",
            group = PrivacyGroup.NOTIFICATIONS,
            card = PrivacyCard(
                title = R.string.check_ins,
                whatItDoes = R.string.check_ins_does,
                whatLeaves = R.string.check_ins_leaves,
                whoCanSee = R.string.check_ins_who,
                howToUndo = R.string.check_ins_undo,
            ),
            privateState = false,
            isOn = { settings.followUpCheckIns },
            onAction = { settings.followUpCheckIns = true },
            offAction = {
                settings.followUpCheckIns = false
                checkIns.cancel()
            },
        ),
        // Scanned links are always stored as identifiers; this only allows
        // handing one to the browser. Nothing to clean up when turned off.
        PrivacyToggle(
            id = "coa_links",
            group = PrivacyGroup.CATALOGS,
            card = PrivacyCard(
                title = R.string.coa_links,
                whatItDoes = R.string.coa_links_does,
                whatLeaves = R.string.coa_links_leaves,
                whoCanSee = R.string.coa_links_who,
                howToUndo = R.string.coa_links_undo,
            ),
            privateState = false,
            isOn = { settings.coaLinks },
            onAction = { settings.coaLinks = true },
            offAction = { settings.coaLinks = false },
        ),
        // Play build only. On only switches the scanner's engine; Play
        // services downloads the model when a scan first needs it, after
        // the scanner asks. Off switches back and lets Play services remove
        // the model.
        if (Capability.GOOGLE_TEXT_RECOGNITION in Distribution.capabilities) {
            PrivacyToggle(
                id = "google_text_recognition",
                group = PrivacyGroup.SCANNING,
                card = PrivacyCard(
                    title = R.string.google_text,
                    whatItDoes = R.string.google_text_does,
                    whatLeaves = R.string.google_text_leaves,
                    whoCanSee = R.string.google_text_who,
                    howToUndo = R.string.google_text_undo,
                ),
                privateState = false,
                isOn = { settings.googleTextRecognition },
                onAction = { settings.googleTextRecognition = true },
                offAction = {
                    settings.googleTextRecognition = false
                    Distribution.releaseGoogleTextRecognition(context)
                },
            )
        } else {
            null
        },
        // On only enables the sync card (choose a folder, sync now); syncing
        // runs while the app is open. Off gives up the folder permission and
        // forgets what was synced; files already there stay, encrypted.
        PrivacyToggle(
            id = FOLDER_SYNC,
            group = PrivacyGroup.SYNC_AND_KEYS,
            card = PrivacyCard(
                title = R.string.folder_sync,
                whatItDoes = R.string.folder_sync_does,
                whatLeaves = R.string.folder_sync_leaves,
                whoCanSee = R.string.folder_sync_who,
                howToUndo = R.string.folder_sync_undo,
            ),
            privateState = false,
            isOn = { settings.folderSync },
            onAction = { settings.folderSync = true },
            offAction = {
                settings.folderSync = false
                folderSync.forget()
            },
        ),
        // Which input the new-note button opens. Voice is on-device only,
        // so neither side creates anything to clean up.
        PrivacyToggle(
            id = "voice_first",
            group = PrivacyGroup.KEYBOARD_AND_VOICE,
            card = PrivacyCard(
                title = R.string.voice_first,
                whatItDoes = R.string.voice_first_does,
                whatLeaves = R.string.voice_first_leaves,
                whoCanSee = R.string.voice_first_who,
                howToUndo = R.string.voice_first_undo,
            ),
            privateState = true,
            isOn = { settings.voiceFirst },
            onAction = { settings.voiceFirst = true },
            offAction = { settings.voiceFirst = false },
        ),
    ),
    presets = listOf(MOST_PRIVATE, YOLO_MODE),
)

/** The online map's toggle ID, for the spot editor to find it. */
const val ONLINE_MAP = "online_map"

/** The folder sync toggle's ID. */
const val FOLDER_SYNC = "folder_sync"

/** The default: every protection on, every optional feature off. */
val MOST_PRIVATE = PrivacyPreset("most_private", R.string.preset_most_private, R.string.preset_most_private_does, emptySet())

/**
 * The platform's conveniences, chosen by the maintainer (2026-10-12).
 * Screen protection stays on, and "Notify me" stays its own choice
 * (location "all the time"). The hard lines hold in every preset.
 */
val YOLO_MODE = PrivacyPreset(
    "yolo",
    R.string.preset_yolo,
    R.string.preset_yolo_does,
    setOf(
        "google_text_recognition",
        "coa_links",
        "show_nearby_spots",
        "alert_names",
        "alert_details",
        "follow_up_check_ins",
        // Added by the maintainer, 2026-10-12.
        ONLINE_MAP,
    ),
)
