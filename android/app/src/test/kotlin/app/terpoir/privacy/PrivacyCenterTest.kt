// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

class PrivacyCenterTest {
    private val card = PrivacyCard(1, 2, 3, 4, 5)

    /** A toggle backed by a field, counting how often each action ran. */
    private class Fake(initial: Boolean) {
        var on = initial
        var onRuns = 0
        var offRuns = 0
    }

    private fun toggle(
        id: String,
        fake: Fake,
        privateState: Boolean = false,
        group: PrivacyGroup = PrivacyGroup.LOCATION,
    ) = PrivacyToggle(
        id = id,
        group = group,
        card = card,
        privateState = privateState,
        isOn = { fake.on },
        onAction = { fake.onRuns++; fake.on = true },
        offAction = { fake.offRuns++; fake.on = false },
    )

    @Test
    fun duplicateIdsAreRejected() {
        assertThrows(IllegalArgumentException::class.java) {
            PrivacyCenter(listOf(toggle("a", Fake(false)), toggle("a", Fake(false))))
        }
    }

    @Test
    fun relaxedCountsTogglesAwayFromTheirPrivateState() {
        val leakyOff = Fake(false)
        val leakyOn = Fake(true)
        val protectionOn = Fake(true)
        val protectionOff = Fake(false)
        val center = PrivacyCenter(
            listOf(
                toggle("leaky-off", leakyOff),
                toggle("leaky-on", leakyOn),
                toggle("protection-on", protectionOn, privateState = true),
                toggle("protection-off", protectionOff, privateState = true),
            ),
        )
        assertEquals(2, center.relaxedCount())
    }

    @Test
    fun restoreRunsEveryCleanupAndReachesMaximumPrivacy() = runBlocking {
        val leaky = Fake(true)
        val alreadyOff = Fake(false)
        val protection = Fake(false)
        val center = PrivacyCenter(
            listOf(
                toggle("leaky", leaky),
                toggle("already-off", alreadyOff),
                toggle("protection", protection, privateState = true),
            ),
        )
        center.restoreMaximumPrivacy()
        assertEquals(0, center.relaxedCount())
        assertFalse(leaky.on)
        assertTrue(protection.on)
        // Cleanup runs even for a toggle that already looked private.
        assertEquals(1, alreadyOff.offRuns)
    }

    @Test
    fun restoreReachesEveryToggleEvenWhenOneFails() {
        val failing = PrivacyToggle(
            id = "failing",
            group = PrivacyGroup.SCANNING,
            card = card,
            privateState = false,
            isOn = { false },
            onAction = {},
            offAction = { error("cleanup failed") },
        )
        val after = Fake(true)
        val center = PrivacyCenter(listOf(failing, toggle("after", after)))
        val thrown = assertThrows(IllegalStateException::class.java) { runBlocking { center.restoreMaximumPrivacy() } }
        assertEquals("cleanup failed", thrown.message)
        // The toggle after the failing one was still restored.
        assertFalse(after.on)
        assertEquals(1, after.offRuns)
    }

    @Test
    fun anActionThatDoesNotTakeEffectFails() {
        val broken = PrivacyToggle(
            id = "broken",
            group = PrivacyGroup.PHOTOS,
            card = card,
            privateState = false,
            isOn = { true },
            onAction = {},
            offAction = {},
        )
        assertThrows(IllegalStateException::class.java) { runBlocking { broken.set(false) } }
    }

    @Test
    fun groupsFollowDisplayOrderAndSkipEmptyOnes() {
        val center = PrivacyCenter(
            listOf(
                toggle("security", Fake(true), privateState = true, group = PrivacyGroup.DEVICE_SECURITY),
                toggle("location", Fake(false), group = PrivacyGroup.LOCATION),
            ),
        )
        assertEquals(listOf(PrivacyGroup.LOCATION, PrivacyGroup.DEVICE_SECURITY), center.groups.keys.toList())
    }
}
