// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DistributionTest {
    @Test
    fun capabilitiesMatchTheDistribution() {
        when (Distribution.NAME) {
            "foss" -> assertTrue(Distribution.capabilities.isEmpty())
            "play" -> assertEquals(Capability.entries.toSet(), Distribution.capabilities)
            else -> throw AssertionError("unknown distribution")
        }
    }
}
