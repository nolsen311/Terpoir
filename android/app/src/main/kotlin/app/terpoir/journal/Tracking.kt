// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.annotation.StringRes
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.compositionLocalOf
import app.terpoir.R

/**
 * A data point the user can hide in "What I track" (docs/design-decisions.md).
 * The slugs are the core's (`trackable_data_points`). When, product, method,
 * effects and rating are always tracked, so they are not here.
 */
enum class DataPoint(val slug: String, @StringRes val title: Int, @StringRes val description: Int) {
    BRAND("brand", R.string.track_brand, R.string.track_brand_does),
    STRAIN("strain", R.string.track_strain, R.string.track_strain_does),
    PRICE_AND_SHOP("price_and_shop", R.string.track_price_and_shop, R.string.track_price_and_shop_does),
    PURCHASES("purchases", R.string.track_purchases, R.string.track_purchases_does),
    POTENCY("potency", R.string.track_potency, R.string.track_potency_does),
    TERPENES("terpenes", R.string.track_terpenes, R.string.track_terpenes_does),
    BATCH("batch", R.string.track_batch, R.string.track_batch_does),
    DOSE("dose", R.string.track_dose, R.string.track_dose_does),
    TIMELINE("timeline", R.string.track_timeline, R.string.track_timeline_does),
    MOOD("mood", R.string.track_mood, R.string.track_mood_does),
    INTENTION("intention", R.string.track_intention, R.string.track_intention_does),
    SOCIAL("social", R.string.track_social, R.string.track_social_does),
}

/**
 * Which data points the journal shows. Editors leave hidden fields out but
 * carry their saved values unchanged; scans, insights and share cards skip
 * them. Hiding purchases also hides prices and shops (the core reports
 * both).
 */
/**
 * The journal's strain detail step (docs/design-decisions.md, "Richer
 * strains"); slugs are the core's (`vocabulary().strainDetails`).
 */
enum class StrainDetail(val slug: String, @StringRes val title: Int, @StringRes val description: Int) {
    SIMPLE("simple", R.string.strain_detail_simple, R.string.strain_detail_simple_does),
    TERPENES("terpenes", R.string.strain_detail_terpenes, R.string.strain_detail_terpenes_does),
    DETAILED("detailed", R.string.strain_detail_detailed, R.string.strain_detail_detailed_does),
    ;

    companion object {
        /** The step for a core slug; one from a newer app shows everything. */
        fun of(slug: String): StrainDetail = entries.firstOrNull { it.slug == slug } ?: DETAILED
    }
}

@Immutable
data class Tracking(val hidden: Set<String>, val strainDetail: StrainDetail = StrainDetail.TERPENES) {
    /** Whether [point] is shown. */
    fun shows(point: DataPoint): Boolean = point.slug !in hidden

    companion object {
        /** Everything shown: the default, and before the journal answers. */
        val EVERYTHING = Tracking(emptySet())
    }
}

/** The open journal's tracking choices, for every screen below JournalHome. */
val LocalTracking = compositionLocalOf { Tracking.EVERYTHING }
