// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.terpoir.TerpoirApplication
import app.terpoir.map.MapProvider
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Every toggle the app registers works both ways on a real device. */
@RunWith(AndroidJUnit4::class)
class PrivacyTogglesTest {
    private val app = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
        as TerpoirApplication

    @After
    fun restore() = runBlocking { app.privacyCenter.restoreMaximumPrivacy() }

    @Test
    fun screenProtectionIsRegistered() {
        assertTrue(app.privacyCenter.toggles.any { it.id == "screen_protection" })
    }

    @Test
    fun everyPresetAppliesAndMatches() = runBlocking {
        val center = app.privacyCenter
        center.restoreMaximumPrivacy()
        assertEquals("most_private", center.currentPreset()?.id)
        for (preset in center.presets) {
            center.apply(preset)
            assertEquals(preset.id, center.currentPreset()?.id)
            assertTrue(center.changes(preset).isEmpty())
        }
        // Leaving YOLO Mode runs the off-action of everything it turned on.
        center.apply(center.presets.first { it.id == "most_private" })
        assertEquals(0, center.relaxedCount())
    }

    @Test
    fun everyToggleRelaxesAndRestores() = runBlocking {
        val center = app.privacyCenter
        center.restoreMaximumPrivacy()
        assertEquals(0, center.relaxedCount())
        for (toggle in center.toggles) {
            toggle.set(!toggle.privateState)
            assertEquals(1, center.relaxedCount())
            toggle.set(toggle.privateState)
            assertEquals(0, center.relaxedCount())
        }
    }

    @Test
    fun theOnlineMapChoosesItsProvider() = runBlocking {
        val settings = app.privacySettings
        val map = app.privacyCenter.toggles.first { it.id == ONLINE_MAP }
        val hadKey = settings.mapTilerKeySaved
        try {
            // No key: OpenFreeMap.
            settings.mapTilerKeySaved = false
            map.set(true)
            assertEquals(MapProvider.OPENFREEMAP, settings.mapProvider)
            map.set(false)
            // With the user's own key, turning the map on (as YOLO Mode
            // does) chooses MapTiler; off returns to OpenFreeMap.
            settings.mapTilerKeySaved = true
            map.set(true)
            assertEquals(MapProvider.MAPTILER, settings.mapProvider)
            map.set(false)
            assertEquals(MapProvider.OPENFREEMAP, settings.mapProvider)
        } finally {
            settings.mapTilerKeySaved = hadKey
        }
    }
}
