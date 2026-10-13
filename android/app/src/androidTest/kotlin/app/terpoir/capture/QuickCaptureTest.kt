// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import android.content.ComponentName
import android.content.pm.PackageManager
import android.content.pm.ShortcutManager
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.terpoir.TerpoirApplication
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Quick-capture surfaces follow their settings on a real device. */
@RunWith(AndroidJUnit4::class)
class QuickCaptureTest {
    private val app = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext
        as TerpoirApplication

    @After
    fun restore() = runBlocking {
        app.privacyCenter.restoreMaximumPrivacy()
        app.privacySettings.newNoteShortcut = true
        QuickCapture.setShortcut(app, true)
    }

    private fun enabled(component: Class<*>): Boolean =
        app.packageManager.getComponentEnabledSetting(ComponentName(app, component)) ==
            PackageManager.COMPONENT_ENABLED_STATE_ENABLED

    @Test
    fun tileAndWidgetFollowTheirToggles() = runBlocking {
        val tile = app.privacyCenter.toggles.first { it.id == "quick_settings_tile" }
        val widget = app.privacyCenter.toggles.first { it.id == "home_widget" }
        tile.set(true)
        widget.set(true)
        assertTrue(enabled(NewNoteTileService::class.java))
        assertTrue(enabled(NewNoteWidget::class.java))
        tile.set(false)
        widget.set(false)
        assertFalse(enabled(NewNoteTileService::class.java))
        assertFalse(enabled(NewNoteWidget::class.java))
    }

    @Test
    fun shortcutCanBeHiddenAndShown() {
        val shortcuts = app.getSystemService(ShortcutManager::class.java)
        QuickCapture.setShortcut(app, true)
        assertEquals(listOf("new_note"), shortcuts.dynamicShortcuts.map { it.id })
        QuickCapture.setShortcut(app, false)
        assertTrue(shortcuts.dynamicShortcuts.isEmpty())
        QuickCapture.setShortcut(app, true)
        assertEquals(listOf("new_note"), shortcuts.dynamicShortcuts.map { it.id })
    }
}
