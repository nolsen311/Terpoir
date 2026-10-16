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
 * text), or to a new entry. Every entry point opens the app. A note can be
 * written while the journal is locked; an entry opens after unlock. Every
 * surface says only "New note" or "New entry", with a neutral icon, and
 * never shows journal content.
 *
 * - The app shortcut (long-press the icon) is on by default and can be
 *   hidden. It is dynamic, not declared in the manifest, so hiding it
 *   removes it, pinned copies included.
 * - The Quick Settings tile and the home-screen widget are opt-in Privacy
 *   Center toggles. Their components ship disabled; turning one on enables
 *   it, and turning it off disables it again, which takes the tile out of
 *   Quick Settings and the widget out of the widget list. One toggle
 *   covers both tiles ("New note" and "New entry"); the widget has both
 *   buttons.
 */
object QuickCapture {
    /** Opens a new note after unlock. */
    const val ACTION_NEW_NOTE = "app.terpoir.NEW_NOTE"

    /** Opens a new entry (a session) after unlock. */
    const val ACTION_NEW_ENTRY = "app.terpoir.NEW_ENTRY"

    /** What an entry point opens: its intent action, shortcut ID and label. */
    enum class Target(val action: String, val shortcutId: String, val label: Int) {
        NOTE(ACTION_NEW_NOTE, "new_note", R.string.capture_new_note),
        ENTRY(ACTION_NEW_ENTRY, "new_entry", R.string.session_new),
    }

    /** The intent an entry point launches. */
    fun intent(context: Context, target: Target): Intent =
        Intent(context, MainActivity::class.java)
            .setAction(target.action)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)

    /** The same, for the tiles and the widget (one request code per target). */
    fun pendingIntent(context: Context, target: Target): PendingIntent =
        PendingIntent.getActivity(context, target.ordinal, intent(context, target), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)

    /** Shows or hides the "New note" and "New entry" app shortcuts. */
    fun setShortcut(context: Context, shown: Boolean) {
        val shortcuts = context.getSystemService(ShortcutManager::class.java) ?: return
        val ids = Target.entries.map { it.shortcutId }
        if (shown) {
            // A shortcut hidden earlier stays disabled until re-enabled.
            shortcuts.enableShortcuts(ids)
            shortcuts.dynamicShortcuts = Target.entries.map { target ->
                ShortcutInfo.Builder(context, target.shortcutId)
                    .setShortLabel(context.getString(target.label))
                    .setIcon(Icon.createWithResource(context, R.drawable.ic_add))
                    .setIntent(intent(context, target))
                    .setRank(target.ordinal)
                    .build()
            }
        } else {
            shortcuts.removeDynamicShortcuts(ids)
            // Copies the user pinned to the home screen stop working too.
            shortcuts.disableShortcuts(ids, context.getString(R.string.capture_shortcut_hidden))
        }
    }

    /** Enables or disables both Quick Settings tiles. */
    fun setTiles(context: Context, enabled: Boolean) {
        setComponent(context, NewNoteTileService::class.java, enabled)
        setComponent(context, NewEntryTileService::class.java, enabled)
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
        setTiles(context, settings.quickSettingsTile)
        setComponent(context, NewNoteWidget::class.java, settings.homeWidget)
    }
}
