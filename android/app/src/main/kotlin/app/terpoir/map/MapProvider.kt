// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import app.terpoir.R

/**
 * Whose map the online map shows (docs/design-decisions.md, "Map picker"):
 * one provider at a time, never combined. The "Online map" toggle says
 * whether there is a map at all.
 */
enum class MapProvider(
    /** Stored in the settings; never renamed. */
    val id: String,
    /** The only hosts the gateway lets out while this provider is chosen. */
    val hosts: Set<String>,
    /** The credit drawn over the map, if the provider doesn't draw its own. */
    val attribution: Int?,
) {
    /**
     * OpenStreetMap vector tiles from OpenFreeMap's public instance. No key,
     * registration or cookies. Its style, tiles, fonts, icons and shaded
     * relief all come from the one host.
     */
    OPENFREEMAP("openfreemap", setOf("tiles.openfreemap.org"), R.string.map_attribution_openfreemap),

    /** MapTiler's street map, only with the user's own API key. */
    MAPTILER("maptiler", setOf("api.maptiler.com"), R.string.map_attribution_maptiler),

    /**
     * Google Maps through Play services, Play builds with a key only. Its
     * SDK does its own networking, outside the gateway, which therefore
     * lets nothing out for MapLibre while Google is chosen. Google draws
     * its own logo and credits.
     */
    GOOGLE("google", emptySet(), null),
    ;

    /**
     * The style MapLibre loads; [mapTilerKey] is needed for [MAPTILER] only.
     * Null for [GOOGLE], which MapLibre never draws.
     */
    fun styleUrl(mapTilerKey: String?): String? = when (this) {
        OPENFREEMAP -> "https://tiles.openfreemap.org/styles/liberty"
        MAPTILER -> mapTilerKey?.let { "https://api.maptiler.com/maps/streets-v2/style.json?key=$it" }
        GOOGLE -> null
    }

    companion object {
        /** The default, in every build. */
        val DEFAULT = OPENFREEMAP

        /** The provider stored as [id]; the default for anything unknown. */
        fun fromId(id: String?): MapProvider = entries.firstOrNull { it.id == id } ?: DEFAULT
    }
}
