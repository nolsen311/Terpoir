// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.BrandData
import app.terpoir.core.Journal
import app.terpoir.core.ProductData
import app.terpoir.core.ProductMatch
import app.terpoir.core.PurchaseData
import app.terpoir.core.RecordType
import app.terpoir.core.SessionData
import app.terpoir.core.SnippetData
import app.terpoir.core.SpotData
import app.terpoir.core.StrainData
import app.terpoir.core.TagSuggestion
import app.terpoir.core.TagData
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** A record and its ID. */
data class Entry<T>(val id: String, val data: T)

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

    /** Spots, by name. */
    suspend fun spots(): List<Entry<SpotData>> = io {
        load(RecordType.SPOT, journal::getSpot).sortedBy { it.data.name.lowercase() }
    }

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

    /** Deletes a record (crypto-erasure in the core). */
    suspend fun delete(id: String): Boolean = io { journal.deleteRecord(id) }

    private fun <T> load(type: RecordType, get: (String) -> T?): List<Entry<T>> =
        journal.list(type).mapNotNull { id -> get(id)?.let { Entry(id, it) } }

    private fun now(): ULong = clock().toULong()

    private suspend fun <T> io(block: () -> T): T = withContext(Dispatchers.IO) { block() }
}
