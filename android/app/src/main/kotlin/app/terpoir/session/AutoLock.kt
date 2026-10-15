// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import android.app.KeyguardManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner

/**
 * Applies [AutoLockPolicy] to the whole app process: hides content when the
 * app leaves the foreground (not for a system picker the user opened, see
 * [HandOff]), wipes the master key after an opt-in timeout,
 * and locks immediately when the screen turns off or the device is locked
 * (the only trigger with the default, [LockTimeout.WITH_DEVICE]).
 */
class AutoLock(
    private val context: Context,
    private val session: JournalSession,
    private val settings: LockSettings,
) : DefaultLifecycleObserver {
    private val handler = Handler(Looper.getMainLooper())
    private val lockNow = Runnable { session.lock() }
    private var backgroundedAt: Long? = null
    private var screenWentOff = false

    private val screenOff = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action == Intent.ACTION_SCREEN_OFF) {
                screenWentOff = true
                session.hideContent()
                session.lock()
            }
        }
    }

    /** Starts watching. Call once, from the Application. */
    fun install() {
        ProcessLifecycleOwner.get().lifecycle.addObserver(this)
        val filter = IntentFilter(Intent.ACTION_SCREEN_OFF)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.registerReceiver(screenOff, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            context.registerReceiver(screenOff, filter)
        }
    }

    override fun onStop(owner: LifecycleOwner) {
        // A picker the user opened is part of what they're doing here: the
        // screen that opened it must stay, or its answer is lost.
        val handedOff = HandOff.active
        if (!handedOff) session.hideContent()
        backgroundedAt = SystemClock.elapsedRealtime()
        val policy = AutoLockPolicy(settings.timeout)
        if (policy.lockOnBackground(handedOff)) {
            session.lock()
        } else {
            policy.backgroundDelayMillis(handedOff)?.let { handler.postDelayed(lockNow, it) }
        }
    }

    override fun onStart(owner: LifecycleOwner) {
        handler.removeCallbacks(lockNow)
        val keyguard = context.getSystemService(KeyguardManager::class.java)
        val lock = AutoLockPolicy(settings.timeout).lockOnReturn(
            backgroundedAt = backgroundedAt,
            now = SystemClock.elapsedRealtime(),
            deviceLocked = screenWentOff || keyguard.isDeviceLocked,
            handedOff = HandOff.active,
        )
        if (lock) session.lock()
        backgroundedAt = null
        screenWentOff = false
        session.showContent()
    }
}
