// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.terpoir.core.BrandData
import app.terpoir.core.CoreException
import app.terpoir.core.Journal
import app.terpoir.core.createKeys
import app.terpoir.core.init as initCore
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith

/** The Rust core loads and works on a real Android runtime. */
@RunWith(AndroidJUnit4::class)
class CoreTest {
    @Test
    fun journalRoundTripOnDevice() {
        initCore()
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val dir = context.noBackupFilesDir.resolve("core-test").apply { deleteRecursively(); mkdirs() }
        val keys = createKeys()
        val journal = Journal.open(
            dir.resolve("journal.db").path,
            dir.resolve("files").path,
            keys.masterKey,
            keys.wrappedDatabaseKey,
        )
        val id = journal.saveBrand(null, BrandData("Acme"), 1uL)
        assertEquals(BrandData("Acme"), journal.getBrand(id))
        journal.lock()
        assertThrows(CoreException.Locked::class.java) { journal.getBrand(id) }
        keys.masterKey.fill(0)
        dir.deleteRecursively()
    }
}
