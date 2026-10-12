// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import android.Manifest
import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import app.terpoir.MainActivity
import app.terpoir.R
import app.terpoir.TerpoirApplication
import app.terpoir.privacy.PrivacySettings

/**
 * Follow-up check-ins (docs/design-decisions.md, "Follow-up check-ins"): a
 * local reminder some time after a session, "Quick check-in?" at every
 * notification tier. Tapping it opens a one-screen update of that session
 * after unlock.
 *
 * - One pending check-in at a time; a newer one replaces it.
 * - The OS alarm carries nothing: it reveals only when it fires. The
 *   session's ID waits in the app's private, no-backup preferences (an
 *   opaque random ID, meaningless without the journal) and is removed once
 *   the check-in is opened or cancelled.
 * - Inexact alarms only (no exact-alarm permission); a reboot drops a
 *   pending check-in.
 */
class CheckIns(private val context: Context, private val settings: PrivacySettings) {
    private val prefs = context.getSharedPreferences("check_ins", Context.MODE_PRIVATE)
    private val alarms = context.getSystemService(AlarmManager::class.java)

    /** Schedules a check-in on [sessionId] at [atMs], replacing any pending one. */
    fun schedule(sessionId: String, atMs: Long) {
        if (!settings.followUpCheckIns || atMs <= System.currentTimeMillis()) return
        prefs.edit().putString(KEY_SESSION, sessionId).apply()
        alarms?.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, alarmIntent())
    }

    /** The session whose check-in the user tapped, if any. */
    fun pending(): String? = prefs.getString(KEY_SESSION, null)

    /** The check-in was opened: forget it. */
    fun done() {
        prefs.edit().clear().apply()
    }

    /** Cancels the pending check-in and clears a showing reminder (the toggle's off-action). */
    fun cancel() {
        alarms?.cancel(alarmIntent())
        prefs.edit().clear().apply()
        context.getSystemService(NotificationManager::class.java)?.cancel(NOTIFICATION_ID)
    }

    /** The alarm fired: shows the reminder. */
    fun onAlarm() {
        if (!settings.followUpCheckIns || pending() == null) return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, context.getString(R.string.check_in_channel), NotificationManager.IMPORTANCE_DEFAULT)
                .apply { description = context.getString(R.string.check_in_channel_description) },
        )
        val open = PendingIntent.getActivity(
            context,
            0,
            Intent(context, MainActivity::class.java)
                .setAction(ACTION_CHECK_IN)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        manager.notify(
            NOTIFICATION_ID,
            NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.ic_journal)
                .setContentTitle(context.getString(R.string.check_in_title))
                .setContentText(context.getString(R.string.check_in_text))
                .setContentIntent(open)
                .setAutoCancel(true)
                .setCategory(NotificationCompat.CATEGORY_REMINDER)
                .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
                .build(),
        )
    }

    private fun alarmIntent(): PendingIntent = PendingIntent.getBroadcast(
        context,
        0,
        Intent(context, CheckInReceiver::class.java),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    companion object {
        /** The action of the reminder's tap. */
        const val ACTION_CHECK_IN = "app.terpoir.CHECK_IN"

        /**
         * The default delay for a session's methods, in minutes, or `null`
         * for no check-in by default: edibles and drinks take longest to
         * come on, tinctures less; inhaled and topical need none.
         */
        fun defaultMinutes(methods: Collection<String>): Int? = when {
            methods.any { it == "edible" || it == "beverage" } -> 90
            "tincture" in methods -> 45
            else -> null
        }

        private const val CHANNEL = "check_ins"
        private const val NOTIFICATION_ID = 2
        private const val KEY_SESSION = "session"
    }
}

/** Receives the check-in alarm (not exported: only this app's own alarm). */
class CheckInReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        (context.applicationContext as TerpoirApplication).checkIns.onAlarm()
    }
}
