// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.terpoir.core.Journal
import app.terpoir.core.LocalTimeData
import app.terpoir.core.captureReady
import app.terpoir.core.createKeys
import app.terpoir.core.sealCapture
import app.terpoir.journal.JournalRepository
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Write-only capture against the real core on a device: seal while locked, merge on unlock, review. */
@RunWith(AndroidJUnit4::class)
class LockedCaptureTest {
    private val dir = InstrumentationRegistry.getInstrumentation().targetContext.cacheDir
        .resolve("locked-capture-test").apply { deleteRecursively(); mkdirs() }
    private val inbox = dir.resolve("inbox").path
    private val keys = createKeys()

    private fun open() = Journal.open(dir.resolve("journal.db").path, dir.resolve("files").path, keys.masterKey.copyOf(), keys.wrappedDatabaseKey)

    @After
    fun cleanUp() {
        keys.masterKey.fill(0)
        dir.deleteRecursively()
    }

    @Test
    fun aNoteWrittenWhileLockedIsReviewedAfterUnlock() = runBlocking {
        assertFalse(captureReady(inbox))
        open().use { journal ->
            journal.mergeCaptures(inbox, 1u)
            journal.lock()
        }
        assertTrue(captureReady(inbox))

        sealCapture(inbox, "tastes like pine", LocalTimeData(1_760_000_000_000, 60))

        open().use { journal ->
            assertEquals(1u, journal.mergeCaptures(inbox, 2u))
            val repo = JournalRepository(journal)
            val note = repo.standaloneSnippets().single()
            assertEquals("tastes like pine", note.data.text)
            assertTrue(note.data.addedWhileLocked)
            assertEquals(listOf(note.id), journal.lockedCaptures())

            repo.keepCapture(note.id, note.data)
            assertFalse(repo.standaloneSnippets().single().data.addedWhileLocked)
            assertTrue(journal.lockedCaptures().isEmpty())
            journal.lock()
        }
    }
}
