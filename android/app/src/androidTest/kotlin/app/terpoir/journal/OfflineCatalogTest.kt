// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The bundled strain catalog and its licence notice ship in the app and are searchable. */
@RunWith(AndroidJUnit4::class)
class OfflineCatalogTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext

    @Test
    fun bundledCatalogIsSearchable() = runBlocking {
        val found = OfflineCatalog.search(context, "AK-47", limit = 1)
        assertEquals("AK-47", found.single().name)
        assertEquals("Serious Seeds", found.single().breeder)
        assertTrue(OfflineCatalog.search(context, "blue dr").isNotEmpty())
    }

    @Test
    fun licenceNoticeShips() {
        val notice = context.assets.open("licenses/Kushy-MIT.txt").bufferedReader().use { it.readText() }
        assertTrue(notice.contains("Copyright (c) 2016-present, Kushy."))
    }
}
