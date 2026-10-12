// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import androidx.core.app.NotificationCompat
import app.terpoir.MainActivity
import app.terpoir.R
import app.terpoir.privacy.PrivacySettings
import java.time.LocalDate

/**
 * Purchase prompts (docs/design-decisions.md, "Purchase prompts"): leaving
 * an opted-in Re-up Spot after a real stop shows "Quick log?", which opens
 * receipt scan. Discreet at every tier: it never names the spot.
 *
 * No visit log. While the phone is inside a prompt fence, only its opaque
 * ID and when it was entered (the monotonic clock, not the time of day) are
 * kept, and they are dropped on leaving. To keep to one prompt per spot per
 * day, the IDs prompted today are kept until the day changes. Dismissed
 * prompts leave nothing behind.
 */
class PurchasePrompts(private val context: Context, private val settings: PrivacySettings) {
    private val state = context.getSharedPreferences("purchase_prompts", Context.MODE_PRIVATE)

    /** The phone arrived in a prompt fence: after a stop when [stopped], else just entering. */
    fun onArrived(id: String, stopped: Boolean) {
        if (!settings.purchasePrompts) return
        state.edit().putLong(KEY_AT + id, if (stopped) STOPPED else SystemClock.elapsedRealtime()).apply()
    }

    /** The phone left a prompt fence: prompts if it had stopped there, once a day per spot. */
    fun onLeft(id: String) {
        val at = state.getLong(KEY_AT + id, NONE)
        state.edit().remove(KEY_AT + id).apply()
        if (!settings.purchasePrompts || at == NONE) return
        val stopped = at == STOPPED || SystemClock.elapsedRealtime() - at >= STOP_MS
        if (!stopped) return
        val today = LocalDate.now().toEpochDay()
        val prompted = if (state.getLong(KEY_DAY, -1) == today) state.getStringSet(KEY_PROMPTED, emptySet()).orEmpty() else emptySet()
        if (id in prompted) return
        state.edit().putLong(KEY_DAY, today).putStringSet(KEY_PROMPTED, prompted + id).apply()
        notifyQuickLog()
    }

    /** The toggle's off-action, with the fences' removal: drops all pending state and any prompt showing. */
    fun forget() {
        state.edit().clear().apply()
        context.getSystemService(NotificationManager::class.java)?.cancel(NOTIFICATION_ID)
    }

    private fun notifyQuickLog() {
        val manager = context.getSystemService(NotificationManager::class.java) ?: return
        if (!manager.areNotificationsEnabled()) return
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, context.getString(R.string.prompt_channel), NotificationManager.IMPORTANCE_DEFAULT)
                .apply { description = context.getString(R.string.prompt_channel_description) },
        )
        val open = PendingIntent.getActivity(
            context,
            NOTIFICATION_ID,
            Intent(context, MainActivity::class.java)
                .setAction(ACTION_QUICK_LOG)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_journal)
            .setContentTitle(context.getString(R.string.prompt_title))
            .setContentText(context.getString(R.string.prompt_text))
            .setContentIntent(open)
            .setAutoCancel(true)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .build()
        manager.notify(NOTIFICATION_ID, notification)
    }

    companion object {
        /** The action of a prompt's tap: receipt scan, after unlock. */
        const val ACTION_QUICK_LOG = "app.terpoir.QUICK_LOG"

        /** Prompt fence IDs start with this ("p0", "p1", ...). */
        const val FENCE_PREFIX = "p"

        /** Prompt fences at most; "Notify me" gets the rest of the OS limit. */
        const val MAX_FENCES = 20

        /** How long inside counts as a stop where the OS cannot tell (proximity alerts). */
        const val STOP_MS = 3 * 60 * 1000L

        private const val CHANNEL = "purchase_prompts"
        private const val NOTIFICATION_ID = 3
        private const val KEY_AT = "at_"
        private const val KEY_DAY = "day"
        private const val KEY_PROMPTED = "prompted"
        private const val NONE = Long.MIN_VALUE
        private const val STOPPED = -1L
    }
}
