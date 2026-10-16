// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService

/**
 * The "New note" Quick Settings tile (opt-in; see [QuickCapture]). Shows no
 * state and no content: tapping it unlocks the device if needed and opens a
 * new note.
 */
class NewNoteTileService : CaptureTileService(QuickCapture.Target.NOTE)

/** The "New entry" tile: the same, opening a new entry after unlock. */
class NewEntryTileService : CaptureTileService(QuickCapture.Target.ENTRY)

/** A tile with no state that opens [target]. */
abstract class CaptureTileService(private val target: QuickCapture.Target) : TileService() {
    override fun onStartListening() {
        qsTile?.apply {
            state = Tile.STATE_INACTIVE
            updateTile()
        }
    }

    override fun onClick() {
        if (isLocked) unlockAndRun(::open) else open()
    }

    private fun open() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startActivityAndCollapse(QuickCapture.pendingIntent(this, target))
        } else {
            @Suppress("DEPRECATION", "StartActivityAndCollapseDeprecated")
            startActivityAndCollapse(QuickCapture.intent(this, target))
        }
    }
}
