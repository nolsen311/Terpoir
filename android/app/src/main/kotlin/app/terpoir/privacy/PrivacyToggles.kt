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
    ),
)
