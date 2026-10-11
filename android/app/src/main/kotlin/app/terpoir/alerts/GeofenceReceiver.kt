// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import app.terpoir.TerpoirApplication

/**
 * Receives "Notify me" geofence events from the OS location service (not
 * exported: only this app's own pending intents reach it).
 */
class GeofenceReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val app = context.applicationContext as TerpoirApplication
        val fences = app.spotAlerts.geofencer.entered(intent) ?: return
        app.spotAlerts.onEntered(fences)
    }
}
