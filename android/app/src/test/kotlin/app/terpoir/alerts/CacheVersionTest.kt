// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CacheVersionTest {
    private val dir: File = Files.createTempDirectory("alerts").toFile()
    private val file = File(dir, "alerts/version")

    @Test
    fun noVersionIsOutdated() {
        assertFalse(CacheVersion(file, 1).isCurrent)
    }

    @Test
    fun markedVersionIsCurrentUntilTheRulesChange() {
        CacheVersion(file, 1).markCurrent()
        assertTrue(CacheVersion(file, 1).isCurrent)
        // An upgrade whose rules changed.
        assertFalse(CacheVersion(file, 2).isCurrent)
        // A downgrade doesn't trust a newer version's cache either.
        CacheVersion(file, 2).markCurrent()
        assertFalse(CacheVersion(file, 1).isCurrent)
    }

    @Test
    fun anUnreadableVersionIsOutdated() {
        file.parentFile?.mkdirs()
        file.writeText("garbage")
        assertFalse(CacheVersion(file, 1).isCurrent)
    }
}
