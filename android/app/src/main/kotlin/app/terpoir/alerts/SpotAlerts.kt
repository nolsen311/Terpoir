// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import app.terpoir.MainActivity
import app.terpoir.R
import app.terpoir.core.Journal
import app.terpoir.privacy.PrivacySettings
import java.io.File
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/** A permission "Notify me" needs and does not have yet, in the order to ask. */
enum class AlertNeed { PRECISE_LOCATION, BACKGROUND_LOCATION, NOTIFICATIONS }

/**
 * "Notify me": geofences around saved spots, and the alert when the phone
 * stops at one (docs/design-decisions.md, "Location and saved spots" and
 * "Notifications").
 *
 * - Fences are registered only while the journal is unlocked (they need the
 *   spots' coordinates), again on every unlock and after edits, so after a
 *   reboot alerts resume when the journal is next opened.
 * - An alert shows what the tier allows, from the notification cache; with
 *   no cache entry it stays discreet. It never says more on the lock screen
 *   than "Did you hit a spot?".
 * - At most one alert per [COOLDOWN_MS], whichever spot, so alerts don't
 *   become a visit log. Only the end of the quiet period is stored, not
 *   which spot; dismissed alerts leave nothing behind.
 */
class SpotAlerts(
    private val context: Context,
    private val settings: PrivacySettings,
    val geofencer: Geofencer,
) {
    private val cache = AlertCache(File(context.noBackupFilesDir, "alerts/spots.tsv"))
    private val state = context.getSharedPreferences("alerts", Context.MODE_PRIVATE)
    private val lock = Mutex()

    /** The tier the alert settings select. */
    val tier: AlertTier
        get() = when {
            settings.alertDetails -> AlertTier.RICH
            settings.alertNames -> AlertTier.NAMED
            else -> AlertTier.DISCREET
        }

    /** What is still missing for "Notify me" to work, in the order to ask. */
    fun needs(): List<AlertNeed> = buildList {
        if (!granted(Manifest.permission.ACCESS_FINE_LOCATION)) add(AlertNeed.PRECISE_LOCATION)
        if (!granted(Manifest.permission.ACCESS_BACKGROUND_LOCATION)) add(AlertNeed.BACKGROUND_LOCATION)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU && !granted(Manifest.permission.POST_NOTIFICATIONS)) {
            add(AlertNeed.NOTIFICATIONS)
        }
    }

    /**
     * Brings the fences and the cache in line with the settings and the
     * journal. Without the journal (locked) it changes nothing but can still
     * remove: with "Notify me" off, everything is cleared.
     */
    suspend fun refresh(journal: Journal?) = lock.withLock {
        if (!settings.notifyNearSpots) {
            clearLocked()
            return@withLock
        }
        if (journal == null) return@withLock
        val needs = needs()
        if (AlertNeed.PRECISE_LOCATION in needs || AlertNeed.BACKGROUND_LOCATION in needs) {
            geofencer.clear()
            cache.delete()
            return@withLock
        }
        val spots = withContext(Dispatchers.IO) { journal.spotAlerts() }
            .sortedByDescending { it.sessions }
            .take(MAX_FENCES)
        val fences = spots.mapIndexed { index, spot -> Fence("s$index", spot.latE7, spot.lonE7, spot.radiusM.toInt()) }
        val tier = tier
        withContext(Dispatchers.IO) {
            cache.write(
                tier,
                fences.zip(spots).associate { (fence, spot) ->
                    fence.id to CachedSpot(
                        name = spot.name,
                        sessions = spot.sessions.toInt(),
                        ratingTenths = spot.averageRatingTenths?.toInt(),
                        lastProduct = spot.lastProduct,
                    )
                },
            )
        }
        if (fences.isEmpty()) geofencer.clear() else geofencer.register(fences)
    }

    /** Removes every fence, the cache, the quiet period and any alert showing. */
    suspend fun clear() = lock.withLock { clearLocked() }

    /** After the tier was lowered: drops from the cache what it no longer shows. */
    suspend fun narrowCache() = lock.withLock {
        withContext(Dispatchers.IO) { cache.narrowTo(tier) }
    }

    private suspend fun clearLocked() {
        geofencer.clear()
        withContext(Dispatchers.IO) { cache.delete() }
        state.edit().clear().apply()
        context.getSystemService(NotificationManager::class.java)?.cancel(NOTIFICATION_ID)
    }

    /** The phone stopped at the fences [fenceIds]: shows one alert, unless quiet. */
    fun onEntered(fenceIds: List<String>) {
        if (!settings.notifyNearSpots || fenceIds.isEmpty()) return
        val now = System.currentTimeMillis()
        if (now < state.getLong(KEY_QUIET_UNTIL, 0)) return
        if (AlertNeed.NOTIFICATIONS in needs()) return
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, context.getString(R.string.alert_channel), NotificationManager.IMPORTANCE_DEFAULT)
                .apply { description = context.getString(R.string.alert_channel_description) },
        )
        val spot = if (tier == AlertTier.DISCREET) null else cache.read().let { cached -> fenceIds.firstNotNullOfOrNull(cached::get) }
        manager.notify(NOTIFICATION_ID, notification(spot))
        state.edit().putLong(KEY_QUIET_UNTIL, now + COOLDOWN_MS).apply()
    }

    private fun notification(spot: CachedSpot?): android.app.Notification {
        val open = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java)
                .setAction(ACTION_NEAR_SPOT)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val discreet = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_journal)
            .setContentTitle(context.getString(R.string.alert_discreet))
            .setContentText(context.getString(R.string.alert_discreet_text))
            .setContentIntent(open)
            .setAutoCancel(true)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
        if (spot == null || spot.name.isEmpty()) {
            return discreet.setVisibility(NotificationCompat.VISIBILITY_PUBLIC).build()
        }
        // Named and Rich: private on the lock screen, which shows the
        // discreet version instead.
        val detail = if (tier == AlertTier.RICH) detail(spot) else null
        return NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_journal)
            .setContentTitle(context.getString(R.string.alert_named, spot.name))
            .setContentText(detail ?: context.getString(R.string.alert_discreet_text))
            .setContentIntent(open)
            .setAutoCancel(true)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setPublicVersion(discreet.build())
            .build()
    }

    /** The Rich tier's line: rating, sessions and last product, as available. */
    private fun detail(spot: CachedSpot): String? = listOfNotNull(
        spot.ratingTenths?.let { context.getString(R.string.alert_rating, String.format(Locale.getDefault(), "%.1f", it / 10.0)) },
        spot.sessions?.takeIf { it > 0 }?.let { context.resources.getQuantityString(R.plurals.alert_sessions, it, it) },
        spot.lastProduct?.let { context.getString(R.string.alert_last_product, it) },
    ).joinToString(" · ").ifEmpty { null }

    private fun granted(permission: String) =
        ContextCompat.checkSelfPermission(context, permission) == PackageManager.PERMISSION_GRANTED

    companion object {
        /** The action of an alert's tap: the journal checks which spot after unlock. */
        const val ACTION_NEAR_SPOT = "app.terpoir.NEAR_SPOT"

        /** Geofences per app are limited (100 with Play services); the most-used spots win. */
        const val MAX_FENCES = 90

        /** One alert at most per this many milliseconds, whichever spot. */
        const val COOLDOWN_MS = 4 * 60 * 60 * 1000L

        private const val CHANNEL = "spot_alerts"
        private const val NOTIFICATION_ID = 1
        private const val KEY_QUIET_UNTIL = "quiet_until"
    }
}
