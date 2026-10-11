// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AlertCacheTest {
    private val dir: File = Files.createTempDirectory("alerts").toFile()
    private val file = File(dir, "alerts/spots.tsv")
    private val spot = CachedSpot("Spot\t3\nnorth", sessions = 5, ratingTenths = 42, lastProduct = "Blue Dream")

    @Test
    fun discreetKeepsNoCache() {
        val cache = AlertCache(file)
        cache.write(AlertTier.RICH, mapOf("s0" to spot))
        assertTrue(file.exists())
        cache.write(AlertTier.DISCREET, mapOf("s0" to spot))
        assertFalse(file.exists())
        assertEquals(emptyMap<String, CachedSpot>(), cache.read())
    }

    @Test
    fun namedKeepsOnlyTheName() {
        val cache = AlertCache(file)
        cache.write(AlertTier.NAMED, mapOf("s0" to spot))
        assertEquals(mapOf("s0" to CachedSpot("Spot 3 north")), cache.read())
        // Only the name is on disk.
        assertFalse("Blue Dream" in file.readText())
    }

    @Test
    fun richKeepsDetailsAndNarrows() {
        val cache = AlertCache(file)
        cache.write(AlertTier.RICH, mapOf("s0" to spot, "s1" to CachedSpot("Spot 4", sessions = 0)))
        assertEquals(
            mapOf(
                "s0" to CachedSpot("Spot 3 north", 5, 42, "Blue Dream"),
                "s1" to CachedSpot("Spot 4", 0, null, null),
            ),
            cache.read(),
        )
        cache.narrowTo(AlertTier.NAMED)
        assertEquals(mapOf("s0" to CachedSpot("Spot 3 north"), "s1" to CachedSpot("Spot 4")), cache.read())
        cache.narrowTo(AlertTier.DISCREET)
        assertFalse(file.exists())
    }

    @Test
    fun corruptLinesAreSkipped() {
        file.parentFile?.mkdirs()
        file.writeText("garbage\ns2\tSpot 9\t\t\t\n")
        assertEquals(mapOf("s2" to CachedSpot("Spot 9")), AlertCache(file).read())
    }
}
