// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import app.terpoir.R
import app.terpoir.core.TagRefData
import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

/*
 * Display names for the core's vocabulary slugs. The lists themselves come
 * from the core (`vocabulary()`); only their translations live here. A slug
 * this version does not know (from a newer app) is shown as is.
 */

private val LABELS: Map<String, Int> = mapOf(
    // Effects
    "relaxed" to R.string.effect_relaxed,
    "euphoric" to R.string.effect_euphoric,
    "focused" to R.string.effect_focused,
    "creative" to R.string.effect_creative,
    "sleepy" to R.string.effect_sleepy,
    "hungry" to R.string.effect_hungry,
    "anxious" to R.string.effect_anxious,
    "paranoid" to R.string.effect_paranoid,
    "dry_mouth" to R.string.effect_dry_mouth,
    "couch_lock" to R.string.effect_couch_lock,
    // Methods
    "smoked" to R.string.method_smoked,
    "vaped_flower" to R.string.method_vaped_flower,
    "vape_cart" to R.string.method_vape_cart,
    "dabbed" to R.string.method_dabbed,
    "edible" to R.string.method_edible,
    "tincture" to R.string.method_tincture,
    "beverage" to R.string.method_beverage,
    "topical" to R.string.method_topical,
    // Dose units
    "hits" to R.string.unit_hits,
    "bowls" to R.string.unit_bowls,
    "puffs" to R.string.unit_puffs,
    "dabs" to R.string.unit_dabs,
    "mg" to R.string.unit_mg,
    "ml" to R.string.unit_ml,
    // Forms (vape_cart, edible, tincture, beverage and topical share the
    // method names above)
    "flower" to R.string.form_flower,
    "pre_roll" to R.string.form_pre_roll,
    "concentrate" to R.string.form_concentrate,
    // Label units
    "percent" to R.string.label_percent,
    "mg_per_unit" to R.string.label_mg_per_unit,
    "mg_total" to R.string.label_mg_total,
    // Strain types
    "indica" to R.string.strain_indica,
    "sativa" to R.string.strain_sativa,
    "hybrid" to R.string.strain_hybrid,
    "dominant" to R.string.level_dominant,
    "secondary" to R.string.level_secondary,
    "trace" to R.string.level_trace,
    "cbn" to R.string.cannabinoid_cbn,
    "cbg" to R.string.cannabinoid_cbg,
    "thca" to R.string.cannabinoid_thca,
    "cbdv" to R.string.cannabinoid_cbdv,
    // Intentions
    "sleep" to R.string.intention_sleep,
    "relax" to R.string.intention_relax,
    "create" to R.string.intention_create,
    "relief" to R.string.intention_relief,
    // Terpenes
    "myrcene" to R.string.terpene_myrcene,
    "limonene" to R.string.terpene_limonene,
    "caryophyllene" to R.string.terpene_caryophyllene,
    "linalool" to R.string.terpene_linalool,
    "alpha_pinene" to R.string.terpene_alpha_pinene,
    "beta_pinene" to R.string.terpene_beta_pinene,
    "humulene" to R.string.terpene_humulene,
    "terpinolene" to R.string.terpene_terpinolene,
    "ocimene" to R.string.terpene_ocimene,
    "bisabolol" to R.string.terpene_bisabolol,
    "nerolidol" to R.string.terpene_nerolidol,
    "terpineol" to R.string.terpene_terpineol,
    // Flavors
    "citrus" to R.string.flavor_citrus,
    "berry" to R.string.flavor_berry,
    "sweet" to R.string.flavor_sweet,
    "earthy" to R.string.flavor_earthy,
    "pine" to R.string.flavor_pine,
    "diesel" to R.string.flavor_diesel,
    "skunk" to R.string.flavor_skunk,
    "spicy" to R.string.flavor_spicy,
    "herbal" to R.string.flavor_herbal,
    "floral" to R.string.flavor_floral,
    "woody" to R.string.flavor_woody,
    "minty" to R.string.flavor_minty,
    // Moods
    "happy" to R.string.mood_happy,
    "calm" to R.string.mood_calm,
    "content" to R.string.mood_content,
    "energetic" to R.string.mood_energetic,
    "stressed" to R.string.mood_stressed,
    "irritable" to R.string.mood_irritable,
    "sad" to R.string.mood_sad,
    "tired" to R.string.mood_tired,
    // Targeted symptoms; "sleep" shares the intention's label.
    "pain" to R.string.symptom_pain,
    "anxiety" to R.string.symptom_anxiety,
    "appetite" to R.string.symptom_appetite,
    "nausea" to R.string.symptom_nausea,
    "focus" to R.string.symptom_focus,
    // Spot purposes
    "blaze" to R.string.purpose_blaze,
    "munch" to R.string.purpose_munch,
    "re_up" to R.string.purpose_re_up,
    "chill" to R.string.purpose_chill,
    // Scanned code kinds
    "barcode" to R.string.code_barcode,
    "qr" to R.string.code_qr,
    "package_tag" to R.string.code_package_tag,
    // Social
    "solo" to R.string.social_solo,
    "with_others" to R.string.social_with_others,
)

/** The display name of a vocabulary slug. */
@Composable
fun label(slug: String): String = LABELS[slug]?.let { stringResource(it) } ?: slug.replace('_', ' ')

/** The display name of an effect: built-in by slug, custom by its tag's name. */
@Composable
fun tagLabel(tag: TagRefData, customNames: Map<String, String>): String = when (tag) {
    is TagRefData.Builtin -> label(tag.slug)
    is TagRefData.Custom -> customNames[tag.id] ?: stringResource(R.string.effect_unknown_custom)
}

/** "Sat, Oct 11, 2026, 8:40 PM" in the time zone the session was logged in. */
fun formatWhen(unixMs: Long, utcOffsetMin: Short): String =
    DateTimeFormatter.ofLocalizedDateTime(FormatStyle.MEDIUM, FormatStyle.SHORT)
        .format(Instant.ofEpochMilli(unixMs).atOffset(ZoneOffset.ofTotalSeconds(utcOffsetMin * 60)))

/**
 * How a product is shown everywhere: its brand, then its own name ("Fire
 * Bros Block Berry Cart"), so the name needn't repeat the brand. A name that
 * already starts with the brand is shown as it is.
 */
fun productLabel(name: String, brand: String?): String {
    val b = brand?.trim().orEmpty()
    val n = name.trim()
    return when {
        b.isEmpty() -> n
        n.isEmpty() -> b
        n.startsWith(b, ignoreCase = true) -> n
        else -> "$b $n"
    }
}
