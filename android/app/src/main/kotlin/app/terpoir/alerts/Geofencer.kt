// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.location.LocationManager

/**
 * A circle the OS location service watches. [id] is opaque ("s0", "s1",
 * ...): never a spot's record ID or name.
 */
data class Fence(val id: String, val latE7: Int, val lonE7: Int, val radiusM: Int)

/**
 * Registers "Notify me" geofences with the OS. The OS location service
 * holds the coordinates in plaintext (a disclosed tradeoff); nothing else
 * about a spot is given to it.
 */
interface Geofencer {
    /** Replaces every registered fence with [fences]; false if the OS refused. */
    suspend fun register(fences: List<Fence>): Boolean

    /** Removes every fence this app registered. */
    suspend fun clear()

    /** The fence IDs an alert intent reports entering, or `null` if it reports none. */
    fun entered(intent: Intent): List<String>?
}

/**
 * Geofencing through the platform LocationManager's proximity alerts: the
 * foss build's only engine and the Play build's fallback without Play
 * services. It reports entering a fence (no dwell time).
 */
class AospGeofencer(private val context: Context) : Geofencer {
    private val manager = context.getSystemService(LocationManager::class.java)
    private val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    // Callers check ACCESS_FINE_LOCATION and ACCESS_BACKGROUND_LOCATION
    // first (SpotAlerts); a revoked permission is caught below.
    @SuppressLint("MissingPermission")
    override suspend fun register(fences: List<Fence>): Boolean {
        clear()
        val manager = manager ?: return false
        return try {
            fences.forEachIndexed { index, fence ->
                manager.addProximityAlert(
                    fence.latE7 / 1e7,
                    fence.lonE7 / 1e7,
                    fence.radiusM.toFloat(),
                    NEVER_EXPIRE,
                    pendingIntent(index, fence.id),
                )
            }
            // Only how many, so clear() can rebuild the same intents.
            prefs.edit().putInt(KEY_COUNT, fences.size).apply()
            true
        } catch (_: SecurityException) {
            prefs.edit().putInt(KEY_COUNT, fences.size).apply()
            clear()
            false
        }
    }

    override suspend fun clear() {
        val count = prefs.getInt(KEY_COUNT, 0)
        for (index in 0 until count) {
            val intent = pendingIntent(index, null)
            runCatching { manager?.removeProximityAlert(intent) }
            intent.cancel()
        }
        prefs.edit().remove(KEY_COUNT).apply()
    }

    override fun entered(intent: Intent): List<String>? {
        if (intent.action != ACTION) return null
        if (!intent.getBooleanExtra(LocationManager.KEY_PROXIMITY_ENTERING, false)) return null
        return listOfNotNull(intent.getStringExtra(EXTRA_FENCE))
    }

    /**
     * One intent per fence (the request code tells them apart). Mutable,
     * because the system adds whether the phone entered or left.
     */
    private fun pendingIntent(index: Int, id: String?): PendingIntent {
        val intent = Intent(context, GeofenceReceiver::class.java).setAction(ACTION)
        if (id != null) intent.putExtra(EXTRA_FENCE, id)
        return PendingIntent.getBroadcast(
            context,
            index,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
        )
    }

    private companion object {
        const val PREFS = "alerts_aosp"
        const val KEY_COUNT = "count"
        const val ACTION = "app.terpoir.alerts.PROXIMITY"
        const val EXTRA_FENCE = "fence"
        const val NEVER_EXPIRE = -1L
    }
}
