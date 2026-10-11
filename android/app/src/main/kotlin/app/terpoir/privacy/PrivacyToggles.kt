// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import app.terpoir.R

/** Builds the Privacy Center with every toggle the app has. */
fun buildPrivacyCenter(settings: PrivacySettings): PrivacyCenter = PrivacyCenter(
    listOf(
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
)
