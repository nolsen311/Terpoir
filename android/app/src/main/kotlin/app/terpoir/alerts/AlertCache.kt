// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import java.io.File

/** How much an alert shows (docs/design-decisions.md, "Notifications"). */
enum class AlertTier {
    /** "Did you hit a spot?": nothing about the spot. The default. */
    DISCREET,

    /** The spot's name. */
    NAMED,

    /** The name, the average rating, the number of sessions and the last product. */
    RICH,
}

/** What one alert may show, as kept in the cache. */
data class CachedSpot(
    val name: String,
    val sessions: Int? = null,
    val ratingTenths: Int? = null,
    val lastProduct: String? = null,
)

/**
 * The notification cache: the only copy of spot names (and, at the Rich
 * tier, session details) outside the encrypted journal, so an alert can
 * show them before the journal is unlocked. It exists only at the Named and
 * Rich tiers, holds only what the tier shows, is kept in the app's
 * no-backup directory, and is deleted when the tier goes back to Discreet
 * or "Notify me" is turned off.
 */
class AlertCache(private val file: File) {
    /** Writes [entries] (fence ID to spot), keeping only what [tier] shows. */
    fun write(tier: AlertTier, entries: Map<String, CachedSpot>) {
        if (tier == AlertTier.DISCREET || entries.isEmpty()) {
            delete()
            return
        }
        val text = entries.entries.joinToString("\n") { (id, spot) -> encode(id, limit(spot, tier)) }
        file.parentFile?.mkdirs()
        val partial = File(file.path + ".partial")
        partial.writeText(text)
        if (!partial.renameTo(file)) partial.delete()
    }

    /** Fence ID to spot; empty when there is no cache. */
    fun read(): Map<String, CachedSpot> {
        if (!file.exists()) return emptyMap()
        return runCatching { file.readLines().mapNotNull(::decode).toMap() }.getOrDefault(emptyMap())
    }

    /** Drops what [tier] no longer shows; works while the journal is locked. */
    fun narrowTo(tier: AlertTier) = write(tier, read())

    fun delete() {
        file.delete()
        File(file.path + ".partial").delete()
    }

    companion object {
        /** Keeps only the fields [tier] shows. */
        fun limit(spot: CachedSpot, tier: AlertTier): CachedSpot = when (tier) {
            AlertTier.DISCREET -> CachedSpot("")
            AlertTier.NAMED -> CachedSpot(spot.name)
            AlertTier.RICH -> spot
        }

        // One line per fence, fields separated by tabs; tabs and line
        // breaks in names become spaces.
        private fun encode(id: String, spot: CachedSpot): String = listOf(
            id,
            spot.name,
            spot.sessions?.toString().orEmpty(),
            spot.ratingTenths?.toString().orEmpty(),
            spot.lastProduct.orEmpty(),
        ).joinToString("\t") { it.replace(Regex("[\t\r\n]"), " ") }

        private fun decode(line: String): Pair<String, CachedSpot>? {
            val fields = line.split('\t')
            if (fields.size != 5 || fields[0].isEmpty()) return null
            return fields[0] to CachedSpot(
                name = fields[1],
                sessions = fields[2].toIntOrNull(),
                ratingTenths = fields[3].toIntOrNull(),
                lastProduct = fields[4].ifEmpty { null },
            )
        }
    }
}
