// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AutoLockPolicyTest {
    @Test
    fun defaultIsOneMinute() {
        assertEquals(LockTimeout.ONE_MINUTE, LockTimeout.DEFAULT)
        assertEquals(LockTimeout.ONE_MINUTE, LockTimeout.fromName(null))
        assertEquals(LockTimeout.ONE_MINUTE, LockTimeout.fromName("garbage"))
        assertEquals(LockTimeout.FIVE_MINUTES, LockTimeout.fromName("FIVE_MINUTES"))
    }

    @Test
    fun immediatelyLocksOnBackground() {
        assertTrue(AutoLockPolicy(LockTimeout.IMMEDIATELY).lockOnBackground())
        assertFalse(AutoLockPolicy(LockTimeout.ONE_MINUTE).lockOnBackground())
        assertEquals(60_000L, AutoLockPolicy(LockTimeout.ONE_MINUTE).backgroundDelayMillis())
    }

    @Test
    fun returnWithinTimeoutStaysUnlocked() {
        val policy = AutoLockPolicy(LockTimeout.ONE_MINUTE)
        assertFalse(policy.lockOnReturn(backgroundedAt = 1_000, now = 60_999, deviceLocked = false))
        assertTrue(policy.lockOnReturn(backgroundedAt = 1_000, now = 61_000, deviceLocked = false))
    }

    @Test
    fun deviceLockAlwaysLocks() {
        val policy = AutoLockPolicy(LockTimeout.FIFTEEN_MINUTES)
        assertTrue(policy.lockOnReturn(backgroundedAt = 1_000, now = 1_001, deviceLocked = true))
        assertTrue(policy.lockOnReturn(backgroundedAt = null, now = 1_001, deviceLocked = true))
    }

    @Test
    fun neverBackgroundedStaysUnlocked() {
        assertFalse(AutoLockPolicy(LockTimeout.ONE_MINUTE).lockOnReturn(null, 5, false))
    }

    @Test
    fun clockGoingBackwardsLocks() {
        assertTrue(AutoLockPolicy(LockTimeout.ONE_MINUTE).lockOnReturn(10_000, 5_000, false))
    }
}
