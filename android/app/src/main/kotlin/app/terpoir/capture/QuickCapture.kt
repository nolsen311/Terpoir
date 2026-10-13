// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import android.app.PendingIntent
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.content.pm.ShortcutInfo
import android.content.pm.ShortcutManager
import android.graphics.drawable.Icon
import app.terpoir.MainActivity
import app.terpoir.R
import app.terpoir.privacy.PrivacySettings

/**
 * Quick capture (docs/design-decisions.md, "Quick capture"): one tap from
 * outside the app to a new note in the user's default mode (voice or
 * text). Every entry point opens the app; the journal unlocks as usual,
 * then the note opens. Every surface says only "New note", with a neutral
 * icon, and never shows journal content.
 *
 * - The app shortcut (long-press the icon) is on by default and can be
 *   hidden. It is dynamic, not declared in the manifest, so hiding it
 *   removes it, pinned copies included.
 * - The Quick Settings tile and the home-screen widget are opt-in Privacy
 *   Center toggles. Their components ship disabled; turning one on enables
 *   it, and turning it off disables it again, which takes the tile out of
 *   Quick Settings and the widget out of the widget list.
 */
object QuickCapture {
    /** Opens a new note after unlock. */
    const val ACTION_NEW_NOTE = "app.terpoir.NEW_NOTE"

    private const val SHORTCUT_ID = "new_note"

    /** The intent every entry point launches. */
    fun intent(context: Context): Intent =
        Intent(context, MainActivity::class.java)
            .setAction(ACTION_NEW_NOTE)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)

    /** The same, for the tile and the widget. */
    fun pendingIntent(context: Context): PendingIntent =
        PendingIntent.getActivity(context, 0, intent(context), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)

    /** Shows or hides the "New note" app shortcut. */
    fun setShortcut(context: Context, shown: Boolean) {
        val shortcuts = context.getSystemService(ShortcutManager::class.java) ?: return
        if (shown) {
            // A shortcut hidden earlier stays disabled until re-enabled.
            shortcuts.enableShortcuts(listOf(SHORTCUT_ID))
            val info = ShortcutInfo.Builder(context, SHORTCUT_ID)
                .setShortLabel(context.getString(R.string.capture_new_note))
                .setIcon(Icon.createWithResource(context, R.drawable.ic_add))
                .setIntent(intent(context))
                .build()
            shortcuts.dynamicShortcuts = listOf(info)
        } else {
            shortcuts.removeDynamicShortcuts(listOf(SHORTCUT_ID))
            // Copies the user pinned to the home screen stop working too.
            shortcuts.disableShortcuts(listOf(SHORTCUT_ID), context.getString(R.string.capture_shortcut_hidden))
        }
    }

    /** Enables or disables the tile's or the widget's component. */
    fun setComponent(context: Context, component: Class<*>, enabled: Boolean) {
        context.packageManager.setComponentEnabledSetting(
            ComponentName(context, component),
            if (enabled) PackageManager.COMPONENT_ENABLED_STATE_ENABLED else PackageManager.COMPONENT_ENABLED_STATE_DISABLED,
            PackageManager.DONT_KILL_APP,
        )
    }

    /** Brings every surface in line with the settings (on app start). */
    fun sync(context: Context, settings: PrivacySettings) {
        setShortcut(context, settings.newNoteShortcut)
        setComponent(context, NewNoteTileService::class.java, settings.quickSettingsTile)
        setComponent(context, NewNoteWidget::class.java, settings.homeWidget)
    }
}
