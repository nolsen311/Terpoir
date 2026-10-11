// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

/** How long after leaving the app the master key is wiped. */
enum class LockTimeout(val millis: Long) {
    /** As soon as the app leaves the foreground. */
    IMMEDIATELY(0),

    /** The default. */
    ONE_MINUTE(60_000),

    /** Five minutes. */
    FIVE_MINUTES(300_000),

    /** Fifteen minutes. */
    FIFTEEN_MINUTES(900_000),
    ;

    companion object {
        /** The default timeout (docs/design-decisions.md, "Memory hygiene and app unlock"). */
        val DEFAULT = ONE_MINUTE

        /** Parses a stored name, falling back to [DEFAULT]. */
        fun fromName(name: String?): LockTimeout = entries.firstOrNull { it.name == name } ?: DEFAULT
    }
}

/**
 * When to lock. Pure logic, so it is unit-tested; [AutoLock] applies it.
 *
 * - Leaving the app hides journal content at once.
 * - The master key is wiped when the timeout passes. A timer does this
 *   while the process runs; because Android may freeze or kill a
 *   backgrounded app before the timer fires, the elapsed time is also
 *   checked on return.
 * - Locking the device locks the journal immediately, whatever the
 *   timeout.
 */
class AutoLockPolicy(private val timeout: LockTimeout) {
    /** Whether to lock right away when the app goes to the background. */
    fun lockOnBackground(): Boolean = timeout == LockTimeout.IMMEDIATELY

    /** Delay before the background timer locks the journal. */
    fun backgroundDelayMillis(): Long = timeout.millis

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
        if (backgroundedAt == null) return false
        // A clock that went backwards (it should not: elapsed-realtime is
        // monotonic) is treated as expired, the safe side.
        val away = now - backgroundedAt
        return away < 0 || away >= timeout.millis
    }
}
