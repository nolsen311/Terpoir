// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.BrandData
import app.terpoir.core.ExportFile
import app.terpoir.core.InsightData
import app.terpoir.core.InsightFeedback
import app.terpoir.core.Journal
import app.terpoir.core.NearbySpot
import app.terpoir.core.ProductData
import app.terpoir.core.ProductMatch
import app.terpoir.core.PurchaseData
import app.terpoir.core.RecordType
import app.terpoir.core.SessionData
import app.terpoir.core.ShopInsightData
import app.terpoir.core.SnippetData
import app.terpoir.core.SpotData
import app.terpoir.core.StrainData
import app.terpoir.core.TagData
import app.terpoir.core.TagSuggestion
import app.terpoir.core.TimelineInsightData
import app.terpoir.core.readExportArchive
import app.terpoir.session.onJournal

/** A record and its ID. */
data class Entry<T>(val id: String, val data: T)

/** An export or sync blob needs a newer version of the app; nothing was changed. */
class NewerAppNeeded : Exception()

/**
 * The open journal for the UI. Every call goes to the Rust core, which owns
 * the data, encryption and storage; calls run off the main thread because
 * they touch the encrypted database.
 */
class JournalRepository(private val journal: Journal, private val clock: () -> Long = System::currentTimeMillis) {
    /** Sessions, newest first. */
    suspend fun sessions(): List<Entry<SessionData>> = io {
        load(RecordType.SESSION, journal::getSession).sortedByDescending { it.data.`when`.unixMs }
    }

    /** Each product's display label (brand, then name; see [productLabel]), by ID. */
    suspend fun productLabels(): Map<String, String> {
        // A product shows without its brand when brands are hidden ("What I track").
        val brands = if (tracking().shows(DataPoint.BRAND)) brands().associate { it.id to it.data.name } else emptyMap()
        // A product with no name of its own is known by its strains.
        val strains = if (tracking().shows(DataPoint.STRAIN)) strains().associate { it.id to it.data.name } else emptyMap()
        return products().associate { p ->
            p.id to productLabel(p.data.name, p.data.brand?.let(brands::get), p.data.strains.mapNotNull(strains::get))
        }
    }

    /** Products, by name. */
    suspend fun products(): List<Entry<ProductData>> = io {
        load(RecordType.PRODUCT, journal::getProduct).sortedBy { it.data.name.lowercase() }
    }

    /** Strains, by name. */
    suspend fun strains(): List<Entry<StrainData>> = io {
        load(RecordType.STRAIN, journal::getStrain).sortedBy { it.data.name.lowercase() }
    }

    /** Brands, by name. */
    suspend fun brands(): List<Entry<BrandData>> = io {
        load(RecordType.BRAND, journal::getBrand).sortedBy { it.data.name.lowercase() }
    }

    /** Standalone stream snippets (not attached to a session), newest first. */
    suspend fun standaloneSnippets(): List<Entry<SnippetData>> = io {
        load(RecordType.SNIPPET, journal::getSnippet)
            .filter { it.data.session == null }
            .sortedByDescending { it.data.at.unixMs }
    }

    suspend fun snippet(id: String): SnippetData? = io { journal.getSnippet(id) }

    /** A session's stream snippets, oldest first (the order they were written). */
    suspend fun sessionSnippets(session: String): List<Entry<SnippetData>> = io {
        load(RecordType.SNIPPET, journal::getSnippet)
            .filter { it.data.session == session }
            .sortedBy { it.data.at.unixMs }
    }

    /** Keyword tag suggestions for [text], matched on the device by the core. */
    suspend fun suggestTags(text: String): List<TagSuggestion> = io { journal.suggestTags(text) }

    suspend fun saveSnippet(id: String?, data: SnippetData): String = io { journal.saveSnippet(id, data, now()) }

    /** "What I track": the data points the user has hidden. */
    suspend fun tracking(): Tracking = io {
        Tracking(journal.hiddenDataPoints().toSet(), StrainDetail.of(journal.strainDetail()))
    }

    /** Sets the journal's strain detail step; lower steps hide fields, never delete them. The choice syncs. */
    suspend fun setStrainDetail(step: StrainDetail) = io { journal.setStrainDetail(step.slug, now()) }

    /** Shows or hides a data point; hiding keeps its values. The choice syncs. */
    suspend fun setTracked(point: DataPoint, tracked: Boolean) = io { journal.setDataPointTracked(point.slug, tracked, now()) }

    /** Deletes a data point's values from every record. Returns how many records changed. */
    suspend fun deleteDataPoint(point: DataPoint): Int = io { journal.deleteDataPoint(point.slug, now()).toInt() }

    /** Keeps a note added while the journal was locked: it stays, no longer marked for review. */
    suspend fun keepCapture(id: String, data: SnippetData): String = saveSnippet(id, data.copy(addedWhileLocked = false))

    /** Spots, by name. */
    suspend fun spots(): List<Entry<SpotData>> = io {
        load(RecordType.SPOT, journal::getSpot).sortedBy { it.data.name.lowercase() }
    }

    suspend fun spot(id: String): SpotData? = io { journal.getSpot(id) }

    /** Saved spots near a fix, closest first, matched by the core. */
    suspend fun spotsNear(latE7: Int, lonE7: Int, accuracyM: Int): List<NearbySpot> =
        io { journal.spotsNear(latE7, lonE7, accuracyM.toUInt()) }

    /** Purchases that include [product], newest first. */
    suspend fun purchasesOf(product: String): List<Entry<PurchaseData>> = io {
        load(RecordType.PURCHASE, journal::getPurchase)
            .filter { p -> p.data.lines.any { it.product == product } }
            .sortedByDescending { it.data.`when`.unixMs }
    }

    /** Products that may be the one being entered (scanned code first, then same name and brand). */
    suspend fun findProducts(name: String?, brand: String?, codes: List<String>): List<ProductMatch> =
        io { journal.findProducts(name, brand, codes) }

    /** The saved spot a receipt's shop name most likely refers to. */
    suspend fun matchSpot(shop: String): String? = io { journal.matchSpot(shop) }

    /** Custom tags of one category (for example `effect`), by name. */
    suspend fun tags(category: String): List<Entry<TagData>> = io {
        load(RecordType.TAG, journal::getTag)
            .filter { it.data.category == category }
            .sortedBy { it.data.name.lowercase() }
    }

    suspend fun session(id: String): SessionData? = io { journal.getSession(id) }

    suspend fun product(id: String): ProductData? = io { journal.getProduct(id) }

    /**
     * Each product's usual method (from its type and subtype; a patch is
     * transdermal), as a new session line's default, by ID. Products with
     * none are left out.
     */
    suspend fun usualMethods(products: List<Entry<ProductData>>): Map<String, String> = io {
        products.mapNotNull { p -> journal.usualMethod(p.data.form, p.data.subtype)?.let { p.id to it } }.toMap()
    }

    suspend fun strain(id: String): StrainData? = io { journal.getStrain(id) }

    suspend fun brand(id: String): BrandData? = io { journal.getBrand(id) }

    /** Saves a session; `id` null creates one. Returns its ID. */
    suspend fun saveSession(id: String?, data: SessionData): String = io { journal.saveSession(id, data, now()) }

    suspend fun saveProduct(id: String?, data: ProductData): String = io { journal.saveProduct(id, data, now()) }

    suspend fun saveStrain(id: String?, data: StrainData): String = io { journal.saveStrain(id, data, now()) }

    suspend fun saveBrand(id: String?, data: BrandData): String = io { journal.saveBrand(id, data, now()) }

    suspend fun saveSpot(id: String?, data: SpotData): String = io { journal.saveSpot(id, data, now()) }

    suspend fun savePurchase(id: String?, data: PurchaseData): String = io { journal.savePurchase(id, data, now()) }

    suspend fun saveTag(id: String?, data: TagData): String = io { journal.saveTag(id, data, now()) }

    /** The user's own MapTiler key for the online map, if saved (this device only, never synced). */
    suspend fun mapTilerKey(): String? = io { journal.maptilerKey() }

    /** Saves the user's MapTiler key, or deletes it with null. */
    suspend fun setMapTilerKey(key: String?) = io { journal.setMaptilerKey(key) }

    /** Deletes a record (crypto-erasure in the core). */
    suspend fun delete(id: String): Boolean = io { journal.deleteRecord(id) }

    /** The whole journal as an encrypted export (only the recovery key opens it). */
    suspend fun exportArchive(masterWrappedByRecovery: ByteArray): ByteArray = io { journal.exportArchive(masterWrappedByRecovery) }

    /** Insights over the rated sessions; see [InsightsScreen]. Call only while insights are on. */
    suspend fun insights(): List<InsightData> = io { journal.insights() }

    /** Shop insights, per Re-up Spot; same opt-in as [insights]. */
    suspend fun shopInsights(): List<ShopInsightData> = io { journal.shopInsights() }

    /** "How it usually goes": typical timelines by method, product and strain. */
    suspend fun timelineInsights(): List<TimelineInsightData> = io { journal.timelineInsights() }

    /** Records the user's feedback on an insight (or a category, for muting). */
    suspend fun setInsightFeedback(key: String, feedback: InsightFeedback) = io { journal.setInsightFeedback(key, feedback, now()) }

    /** The whole journal in plaintext (JSON and CSV files); see [DataCard]. */
    suspend fun exportPlaintext(): List<ExportFile> = io { journal.exportPlaintext(now()) }

    /**
     * Merges an export of this same journal (for example from before a
     * reset) into it; returns how many records were added or updated.
     */
    suspend fun importArchive(bytes: ByteArray): Long = io {
        val archive = readExportArchive(bytes)
        // Checked before anything changes, so a file from a newer app is
        // refused whole rather than imported partly.
        if (journal.checkExportArchive(archive).newerFormatBlobs > 0uL) throw NewerAppNeeded()
        archive.blobs.sumOf { journal.importSyncBlob(it, now()).records.toLong() }
    }

    /** How many records a newer app version wrote, kept until this app can read them. */
    suspend fun newerRecordCount(): Long = io { journal.pendingRecordCount().toLong() }

    private fun <T> load(type: RecordType, get: (String) -> T?): List<Entry<T>> =
        journal.list(type).mapNotNull { id -> get(id)?.let { Entry(id, it) } }

    private fun now(): ULong = clock().toULong()

    // A screen's load can start just after the journal locks; it is
    // cancelled then, not a crash.
    private suspend fun <T> io(block: () -> T): T = onJournal(block)
}
