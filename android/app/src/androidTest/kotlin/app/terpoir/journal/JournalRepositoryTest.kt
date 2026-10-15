// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import android.net.Uri
import app.terpoir.core.BrandData
import app.terpoir.core.CoreException
import app.terpoir.core.Journal
import app.terpoir.core.LocalTimeData
import app.terpoir.core.ProductData
import app.terpoir.core.ProductLineData
import app.terpoir.core.PurchaseData
import app.terpoir.core.PurchaseLineData
import app.terpoir.core.RecordType
import app.terpoir.core.SessionData
import app.terpoir.core.SnippetData
import app.terpoir.core.SpotData
import app.terpoir.core.TagData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData
import app.terpoir.core.createKeys
import app.terpoir.core.defaultMethodForForm
import app.terpoir.core.doseUnitsForMethod
import app.terpoir.core.keysForRestore
import app.terpoir.core.readExportArchive
import app.terpoir.core.trackableDataPoints
import app.terpoir.core.vocabulary
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import org.junit.runner.RunWith

/** The journal UI's data layer against the real core on a device. */
@RunWith(AndroidJUnit4::class)
class JournalRepositoryTest {
    private val dir = InstrumentationRegistry.getInstrumentation().targetContext.cacheDir
        .resolve("repository-test").apply { deleteRecursively(); mkdirs() }
    private val keys = createKeys()
    private val journal = Journal.open(dir.resolve("journal.db").path, dir.resolve("files").path, keys.masterKey, keys.wrappedDatabaseKey)
        .also { keys.masterKey.fill(0) }
    private val repo = JournalRepository(journal)

    @After
    fun cleanUp() {
        journal.lock()
        journal.close()
        dir.deleteRecursively()
    }

    @Test
    fun anExportCarriesTheRecoveryWrappedKeyAndImportsBack() = runBlocking {
        repo.saveBrand(null, BrandData("Acme"))
        val archive = repo.exportArchive(keys.masterWrappedByRecovery)
        assertTrue(readExportArchive(archive).masterWrappedByRecovery.contentEquals(keys.masterWrappedByRecovery))
        // Importing an export of the same journal merges cleanly.
        repo.importArchive(archive)
        assertEquals(listOf("Acme"), repo.brands().map { it.data.name })
    }

    @Test
    fun anExportSavedToAFileRestoresOnANewDeviceUnchanged() = runBlocking {
        val brand = repo.saveBrand(null, BrandData("Acme"))
        val spot = repo.saveSpot(
            null,
            SpotData(
                name = "Corner shop", purposes = emptyList(), latE7 = 377_000_000, lonE7 = -1_224_000_000,
                accuracyM = 25u, radiusM = 150u, purchasePrompts = true,
            ),
        )
        val product = repo.saveProduct(
            null,
            ProductData("Blue Dream", "flower", brand, emptyList(), 22_400, "percent", null, null, emptyList(), null, null, emptyList(), listOf(spot)),
        )
        repo.savePurchase(
            null,
            PurchaseData(
                `when` = LocalTimeData(900, 0),
                spot = spot,
                lines = listOf(PurchaseLineData(lineId = null, product = product, batch = "B-7", priceMinorUnits = 2_000, priceCurrency = "USD")),
            ),
        )
        val session = repo.saveSession(
            null,
            SessionData(
                LocalTimeData(1_000, 0),
                listOf(ProductLineData(null, product, "smoked", 2000, "hits", null, null, null)),
                null, null, null, null,
                listOf(TagIntensity(TagRefData.Builtin("relaxed"), 4)),
                null, null, null, null, 5u, null, null, emptyList(), emptyList(),
            ),
        )
        repo.saveSnippet(null, SnippetData(session, LocalTimeData(1_100, 0), "piney, émigré 🙂", emptyList()))
        val gone = repo.saveBrand(null, BrandData("Gone"))
        assertTrue(repo.delete(gone))

        // Saved and read back through the ContentResolver, as the data card does.
        val file = dir.resolve("export.terpoir")
        val resolver = InstrumentationRegistry.getInstrumentation().targetContext.contentResolver
        val bytes = repo.exportArchive(keys.masterWrappedByRecovery)
        checkNotNull(resolver.openOutputStream(Uri.fromFile(file), "wt")).use { it.write(bytes) }
        val read = checkNotNull(resolver.openInputStream(Uri.fromFile(file))).use { it.readBytes() }
        assertTrue(read.contentEquals(bytes))

        // What restoring does on a new phone, after the Keystore step.
        val archive = readExportArchive(read)
        val restoredKeys = keysForRestore(keys.recoveryKey, archive.masterWrappedByRecovery)
        val newDir = dir.resolve("new-phone").apply { mkdirs() }
        val restored = Journal.open(newDir.resolve("journal.db").path, newDir.resolve("files").path, restoredKeys.masterKey, restoredKeys.wrappedDatabaseKey)
        restoredKeys.masterKey.fill(0)
        try {
            for (blob in archive.blobs) assertEquals(0uL, restored.importSyncBlob(blob, 5_000u).deferred)
            // Every field of every record, compared through the plaintext export.
            val expected = journal.exportPlaintext(0u).map { it.name to it.bytes.decodeToString() }
            val actual = restored.exportPlaintext(0u).map { it.name to it.bytes.decodeToString() }
            assertEquals(expected, actual)
            for (type in RecordType.entries) assertEquals(journal.list(type), restored.list(type))
            assertEquals(null, restored.getBrand(gone))
        } finally {
            restored.lock()
            restored.close()
        }
    }

    @Test
    fun anotherJournalsExportIsRefusedWithoutChanges() = runBlocking {
        repo.saveBrand(null, BrandData("Acme"))
        val otherDir = dir.resolve("other").apply { mkdirs() }
        val otherKeys = createKeys()
        val other = Journal.open(otherDir.resolve("journal.db").path, otherDir.resolve("files").path, otherKeys.masterKey, otherKeys.wrappedDatabaseKey)
        otherKeys.masterKey.fill(0)
        val foreign = try {
            JournalRepository(other).run {
                saveBrand(null, BrandData("Theirs"))
                exportArchive(otherKeys.masterWrappedByRecovery)
            }
        } finally {
            other.lock()
            other.close()
        }
        try {
            repo.importArchive(foreign)
            fail("another journal's export was imported")
        } catch (_: CoreException) {
        }
        assertEquals(listOf("Acme"), repo.brands().map { it.data.name })
    }

    @Test
    fun vocabularyComesFromTheCore() {
        val vocab = vocabulary()
        assertEquals("relaxed", vocab.coreEffects.first())
        assertEquals(listOf("hits", "bowls"), doseUnitsForMethod("smoked"))
        assertEquals("dabbed", defaultMethodForForm("concentrate"))
        for (method in vocab.methods) assertTrue(vocab.knowsUnits(doseUnitsForMethod(method)))
    }

    @Test
    fun sessionsListNewestFirstWithTheirProducts() = runBlocking {
        val brand = repo.saveBrand(null, BrandData("Acme"))
        val product = repo.saveProduct(
            null,
            ProductData("Blue Dream", "flower", brand, emptyList(), 22_400, "percent", null, null, emptyList(), null, null, emptyList(), emptyList()),
        )
        val custom = repo.saveTag(null, TagData("Chatty", EFFECT_CATEGORY))
        repo.saveTag(null, TagData("Pine", "flavor"))
        fun session(ms: Long) = SessionData(
            LocalTimeData(ms, 0),
            listOf(ProductLineData(null, product, "smoked", 2000, "hits", null, null, null)),
            null, null, null, null,
            listOf(TagIntensity(TagRefData.Builtin("relaxed"), 4), TagIntensity(TagRefData.Custom(custom), 2)),
            null, null, null, null, 5u, null, null, emptyList(), emptyList(),
        )
        repo.saveSession(null, session(1_000))
        val newer = repo.saveSession(null, session(2_000))

        val sessions = repo.sessions()
        assertEquals(listOf(newer), sessions.take(1).map { it.id })
        assertEquals(2, sessions.size)
        assertEquals(product, sessions.first().data.lines.single().product)
        assertEquals(listOf("Chatty"), repo.tags(EFFECT_CATEGORY).map { it.data.name })
        assertEquals(listOf("Blue Dream"), repo.products().map { it.data.name })

        assertTrue(repo.delete(newer))
        assertEquals(1, repo.sessions().size)
        // Deleting a session leaves its product.
        assertEquals(1, repo.products().size)
    }

    @Test
    fun standaloneNotesListNewestFirst() = runBlocking {
        fun note(ms: Long, text: String) = SnippetData(null, LocalTimeData(ms, 0), text, emptyList())
        val older = repo.saveSnippet(null, note(1_000, "first"))
        val newer = repo.saveSnippet(null, note(2_000, "second"))
        assertEquals(listOf(newer, older), repo.standaloneSnippets().map { it.id })

        repo.saveSnippet(older, note(1_000, "first, edited"))
        assertEquals("first, edited", repo.snippet(older)?.text)

        assertTrue(repo.delete(newer))
        assertEquals(listOf(older), repo.standaloneSnippets().map { it.id })
    }

    @Test
    fun suggestionsAndSessionNotes() = runBlocking {
        val suggested = repo.suggestTags("so relaxed, not anxious, tasted piney")
            .map { (it.tag as TagRefData.Builtin).slug }
        assertEquals(listOf("relaxed", "pine"), suggested)

        val session = repo.saveSession(
            null,
            SessionData(
                LocalTimeData(5_000, 0), emptyList(), null, null, null, null, emptyList(),
                null, null, null, null, null, null, null, emptyList(), emptyList(),
            ),
        )
        val note = SnippetData(session, LocalTimeData(6_000, 0), "piney", emptyList())
        val id = repo.saveSnippet(null, note)
        assertEquals(listOf(id), repo.sessionSnippets(session).map { it.id })
        // Session notes are not standalone.
        assertTrue(repo.standaloneSnippets().none { it.id == id })
    }

    private fun app.terpoir.core.Vocabulary.knowsUnits(units: List<String>) = doseUnits.containsAll(units)

    @Test
    fun whatITrackHidesAndDeletesThroughTheCore() = runBlocking {
        val brand = repo.saveBrand(null, BrandData("Acme"))
        val product = repo.saveProduct(
            null,
            ProductData("Gummies", "edible", brand, emptyList(), null, null, null, null, emptyList(), null, null, emptyList(), emptyList()),
        )
        assertEquals("Acme Gummies", repo.productLabels()[product])
        assertTrue(repo.tracking().shows(DataPoint.BRAND))

        // Hidden: the label drops the brand, but the product keeps it.
        repo.setTracked(DataPoint.BRAND, false)
        assertEquals(false, repo.tracking().shows(DataPoint.BRAND))
        assertEquals("Gummies", repo.productLabels()[product])
        assertEquals(brand, repo.product(product)?.brand)

        // Deleted: gone from the product, and the brand record with it.
        assertEquals(2, repo.deleteDataPoint(DataPoint.BRAND))
        assertEquals(null, repo.product(product)?.brand)
        assertTrue(repo.brands().isEmpty())

        // Every data point the app shows is one the core knows.
        assertEquals(trackableDataPoints(), DataPoint.entries.map { it.slug })
    }

    @Test
    fun aLoadAfterLockingCancelsInsteadOfCrashing() = runBlocking {
        // Play crash report (0.4.0): a screen's load reached the journal just
        // after locking had closed it, and the bindings' IllegalStateException
        // crashed the app.
        val other = dir.resolve("locked")
        other.mkdirs()
        val k = createKeys()
        val closed = Journal.open(other.resolve("journal.db").path, other.resolve("files").path, k.masterKey, k.wrappedDatabaseKey)
        k.masterKey.fill(0)
        val lockedRepo = JournalRepository(closed)
        closed.lock()
        try {
            lockedRepo.tracking()
            fail("a locked journal should cancel the load")
        } catch (_: CancellationException) {
        }
        closed.close()
        try {
            lockedRepo.tracking()
            fail("a closed journal should cancel the load")
        } catch (_: CancellationException) {
        }
    }
}
