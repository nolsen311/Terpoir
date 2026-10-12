// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

/**
 * When the master key is wiped after leaving the app. [millis] is `null` for
 * [WITH_DEVICE], which has no timer of its own.
 */
enum class LockTimeout(val millis: Long?) {
    /**
     * The default (decided 2026-10-12): no in-app timer. The journal locks
     * when the device locks or the screen turns off, and when the process
     * ends, like the user's other apps.
     */
    WITH_DEVICE(null),

    /** As soon as the app leaves the foreground. */
    IMMEDIATELY(0),

    /** One minute. */
    ONE_MINUTE(60_000),

    /** Five minutes. */
    FIVE_MINUTES(300_000),

    /** Fifteen minutes. */
    FIFTEEN_MINUTES(900_000),
    ;

    companion object {
        /** The default timeout (docs/design-decisions.md, "Memory hygiene and app unlock"). */
        val DEFAULT = WITH_DEVICE

        /** Parses a stored name, falling back to [DEFAULT]. */
        fun fromName(name: String?): LockTimeout = entries.firstOrNull { it.name == name } ?: DEFAULT
    }
}

/**
 * When to lock. Pure logic, so it is unit-tested; [AutoLock] applies it.
 *
 * - Leaving the app hides journal content at once.
 * - With an opt-in timeout, the master key is wiped when it passes. A
 *   timer does this while the process runs; because Android may freeze or
 *   kill a backgrounded app before the timer fires, the elapsed time is
 *   also checked on return.
 * - With [LockTimeout.WITH_DEVICE] (the default) there is no timer.
 * - Locking the device locks the journal immediately, whatever the
 *   setting.
 */
class AutoLockPolicy(private val timeout: LockTimeout) {
    /** Whether to lock right away when the app goes to the background. */
    fun lockOnBackground(): Boolean = timeout == LockTimeout.IMMEDIATELY

    /** Delay before the background timer locks the journal, or `null` for no timer. */
    fun backgroundDelayMillis(): Long? = timeout.millis

    /**
     * Whether to lock when the app returns to the foreground.
     *
     * @param backgroundedAt elapsed-realtime when the app left the
     *   foreground, or `null` if it never did.
     * @param now current elapsed-realtime.
     * @param deviceLocked whether the device was locked meanwhile or is
     *   locked now.
     */
    fun lockOnReturn(backgroundedAt: Long?, now: Long, deviceLocked: Boolean): Boolean {
        if (deviceLocked) return true
        val limit = timeout.millis ?: return false
        if (backgroundedAt == null) return false
        // A clock that went backwards (it should not: elapsed-realtime is
        // monotonic) is treated as expired, the safe side.
        val away = now - backgroundedAt
        return away < 0 || away >= limit
    }
}
