// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PrivacyPresetTest {
    /** A toggle backed by a variable, counting its actions. */
    private class Fake(id: String, privateState: Boolean) {
        var on = privateState
        var actions = 0
        val toggle = PrivacyToggle(
            id = id,
            group = PrivacyGroup.LOCATION,
            card = PrivacyCard(0, 0, 0, 0, 0),
            privateState = privateState,
            isOn = { on },
            onAction = { on = true; actions++ },
            offAction = { on = false; actions++ },
        )
    }

    private val shield = Fake("shield", privateState = true)
    private val banner = Fake("banner", privateState = false)
    private val beacon = Fake("beacon", privateState = false)
    private val mostPrivate = PrivacyPreset("most_private", 0, 0, emptySet())
    private val yolo = PrivacyPreset("yolo", 0, 0, setOf("banner", "not_in_this_build"))
    private val center = PrivacyCenter(listOf(shield.toggle, banner.toggle, beacon.toggle), listOf(mostPrivate, yolo))

    @Test
    fun theDefaultIsMostPrivate() {
        assertEquals("most_private", center.currentPreset()?.id)
    }

    @Test
    fun aPresetSetsEveryToggleAndRunsEveryAction() = runBlocking {
        beacon.on = true
        assertEquals(listOf(banner.toggle to true, beacon.toggle to false), center.changes(yolo))
        center.apply(yolo)
        assertEquals(Triple(true, true, false), Triple(shield.on, banner.on, beacon.on))
        // Toggles already in place still run their action (idempotent cleanup).
        assertEquals(listOf(1, 1, 1), listOf(shield.actions, banner.actions, beacon.actions))
        assertEquals("yolo", center.currentPreset()?.id)
    }

    @Test
    fun anyOtherMixIsCustom() {
        shield.on = false
        assertNull(center.currentPreset())
    }

    @Test
    fun extrasRunAfterTheTogglesAndDontChangeTheMatch() = kotlinx.coroutines.runBlocking {
        var ran = 0
        val withExtra = PrivacyPreset("extra", 0, 0, setOf("banner"), listOf(PresetExtra(0) { ran++ }))
        val c = PrivacyCenter(listOf(shield.toggle, banner.toggle, beacon.toggle), listOf(mostPrivate, withExtra))
        c.apply(withExtra)
        assertEquals(1, ran)
        assertEquals("extra", c.currentPreset()?.id)
    }
}
