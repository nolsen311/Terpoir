// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import android.annotation.SuppressLint
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import app.terpoir.scan.await
import com.google.android.gms.common.ConnectionResult
import com.google.android.gms.common.GoogleApiAvailability
import com.google.android.gms.location.Geofence
import com.google.android.gms.location.GeofencingEvent
import com.google.android.gms.location.GeofencingRequest
import com.google.android.gms.location.LocationServices

/**
 * Geofencing through Google Play services (the Play build), which waits for
 * a short stop ([LOITERING_MS]) so driving past a spot doesn't alert.
 * Falls back to [AospGeofencer] on phones without Play services.
 */
class PlayGeofencer(private val context: Context) : Geofencer {
    private val fallback = AospGeofencer(context)
    private val client = LocationServices.getGeofencingClient(context)

    private fun playServices() =
        GoogleApiAvailability.getInstance().isGooglePlayServicesAvailable(context) == ConnectionResult.SUCCESS

    // Callers check ACCESS_FINE_LOCATION and ACCESS_BACKGROUND_LOCATION
    // first (SpotAlerts); a refusal is caught below.
    @SuppressLint("MissingPermission")
    override suspend fun register(fences: List<Fence>): Boolean {
        clear()
        if (!playServices()) return fallback.register(fences)
        if (fences.isEmpty()) return true
        val request = GeofencingRequest.Builder()
            .setInitialTrigger(GeofencingRequest.INITIAL_TRIGGER_DWELL)
            .addGeofences(
                fences.map { fence ->
                    Geofence.Builder()
                        .setRequestId(fence.id)
                        .setCircularRegion(fence.latE7 / 1e7, fence.lonE7 / 1e7, fence.radiusM.toFloat())
                        .setExpirationDuration(Geofence.NEVER_EXPIRE)
                        .setTransitionTypes(Geofence.GEOFENCE_TRANSITION_DWELL)
                        .setLoiteringDelay(LOITERING_MS)
                        .build()
                },
            )
            .build()
        return try {
            client.addGeofences(request, pendingIntent()).await()
            true
        } catch (_: Exception) {
            false
        }
    }

    override suspend fun clear() {
        if (playServices()) runCatching { client.removeGeofences(pendingIntent()).await() }
        fallback.clear()
    }

    override fun entered(intent: Intent): List<String>? {
        if (intent.action != ACTION) return fallback.entered(intent)
        val event = GeofencingEvent.fromIntent(intent) ?: return null
        if (event.hasError() || event.geofenceTransition != Geofence.GEOFENCE_TRANSITION_DWELL) return null
        return event.triggeringGeofences?.map { it.requestId }
    }

    /** Mutable, because Play services adds the event to it. */
    private fun pendingIntent(): PendingIntent = PendingIntent.getBroadcast(
        context,
        0,
        Intent(context, GeofenceReceiver::class.java).setAction(ACTION),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE,
    )

    private companion object {
        const val ACTION = "app.terpoir.alerts.GEOFENCE"

        /** How long the phone must stay inside before the alert. */
        const val LOITERING_MS = 3 * 60 * 1000
    }
}
