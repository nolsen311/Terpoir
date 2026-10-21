// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.content.Context
import app.terpoir.core.CatalogStrainData
import app.terpoir.core.StrainCatalog
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/**
 * The offline strain catalog bundled with the app (Kushy's open dataset,
 * factual columns only). The core reads and searches it; nothing is sent
 * anywhere. Loaded once, on first use.
 */
object OfflineCatalog {
    /** The asset the build copies from the core's catalog data. */
    const val ASSET = "catalog/strains-kushy.tsv"

    /** How many suggestions the strain editor shows. */
    const val SUGGESTIONS = 3

    private val lock = Mutex()
    private var catalog: StrainCatalog? = null

    /** Strains whose names match [query], best first; empty if the catalog can't be read. */
    suspend fun search(context: Context, query: String, limit: Int = SUGGESTIONS): List<CatalogStrainData> =
        withContext(Dispatchers.Default) {
            load(context)?.search(query, limit.toUInt()).orEmpty()
        }

    private suspend fun load(context: Context): StrainCatalog? = lock.withLock {
        catalog ?: runCatching {
            val text = context.applicationContext.assets.open(ASSET).bufferedReader().use { it.readText() }
            StrainCatalog.open(text)
        }.getOrNull()?.also { catalog = it }
    }
}
