// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import app.terpoir.core.BrandData
import app.terpoir.core.LocalTimeData
import app.terpoir.core.PackageSizeData
import app.terpoir.core.ProductData
import app.terpoir.core.ProductLineData
import app.terpoir.core.PurchaseData
import app.terpoir.core.PurchaseLineData
import app.terpoir.core.SessionData
import app.terpoir.core.SpotData
import app.terpoir.core.StrainData
import app.terpoir.core.SymptomData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData
import app.terpoir.core.TerpeneLevelData
import app.terpoir.journal.JournalRepository
import java.util.TimeZone

/**
 * The journal the store screenshots show: made-up brands and a shop, common
 * strain names, and two weeks of sessions ending [now]: six with the
 * gummies, all rated 5, against seven others, so Insights has a comparison.
 */
suspend fun JournalRepository.seedDemo(now: Long) {
    val offset = (TimeZone.getDefault().getOffset(now) / 60_000).toShort()
    fun daysAgo(days: Int, hour: Int): LocalTimeData {
        val midnight = now - Math.floorMod(now + offset * 60_000L, DAY)
        return LocalTimeData(midnight - days * DAY + hour * HOUR, offset)
    }

    fun tag(slug: String) = TagRefData.Builtin(slug)

    val shop = saveSpot(
        null,
        SpotData(
            name = "Greenleaf on Main", purposes = listOf(tag("buy")), latE7 = null, lonE7 = null,
            accuracyM = null, radiusM = null, purchasePrompts = false,
        ),
    )
    val meadow = saveBrand(null, BrandData("Meadow & Co."))
    val nightjar = saveBrand(null, BrandData("Nightjar Kitchen"))
    val coastal = saveBrand(null, BrandData("Coastal Extracts"))

    suspend fun strain(name: String, type: String, lineage: String, vararg terpenes: String) = saveStrain(
        null,
        StrainData(
            name = name, lineage = lineage, terpenes = terpenes.map(::tag), strainType = type,
            terpeneLevels = terpenes.mapIndexed { i, t -> TerpeneLevelData(tag(t), if (i == 0) "dominant" else "secondary") },
            parents = emptyList(), cannabinoids = emptyList(),
        ),
    )
    val blueDream = strain("Blue Dream", "hybrid", "Blueberry x Haze", "myrcene", "pinene")
    val gdp = strain("Granddaddy Purple", "indica", "Purple Urkle x Big Bud", "myrcene", "caryophyllene")
    val sourDiesel = strain("Sour Diesel", "sativa", "Chemdawg x Super Skunk", "caryophyllene", "limonene")

    val flower = saveProduct(
        null,
        ProductData(
            name = "Blue Dream", form = "flower", brand = meadow, strains = listOf(blueDream),
            thcMilli = 22_400, thcUnit = "percent", cbdMilli = null, cbdUnit = null,
            terpenes = listOf(TagIntensity(tag("myrcene"), 1_150), TagIntensity(tag("pinene"), 420)),
            batch = null, packageDateDay = null, identifiers = emptyList(), shops = listOf(shop),
            defaultSize = PackageSizeData(milli = 3_500, unit = "g", pack = null, per = null),
        ),
    )
    val gummies = saveProduct(
        null,
        ProductData(
            name = "Granddaddy Purple", form = "edible", brand = nightjar, strains = listOf(gdp),
            thcMilli = 5_000, thcUnit = "mg_per_unit", cbdMilli = 1_000, cbdUnit = "mg_per_unit",
            terpenes = emptyList(), batch = null, packageDateDay = null, identifiers = emptyList(),
            shops = listOf(shop), subtype = tag("gummy"),
            defaultSize = PackageSizeData(milli = 5_000, unit = "mg", pack = 20u, per = "each"),
        ),
    )
    val cart = saveProduct(
        null,
        ProductData(
            name = "Sour Diesel", form = "vape_cart", brand = coastal, strains = listOf(sourDiesel),
            thcMilli = 84_000, thcUnit = "percent", cbdMilli = null, cbdUnit = null,
            terpenes = listOf(TagIntensity(tag("limonene"), 2_300)), batch = null, packageDateDay = null,
            identifiers = emptyList(), shops = listOf(shop), subtype = tag("cartridge"),
        ),
    )
    val tincture = saveProduct(
        null,
        ProductData(
            name = "Calm 1:1 Drops", form = "tincture", brand = meadow, strains = emptyList(),
            thcMilli = 300_000, thcUnit = "mg_total", cbdMilli = 300_000, cbdUnit = "mg_total",
            terpenes = emptyList(), batch = null, packageDateDay = null, identifiers = emptyList(),
            shops = listOf(shop),
        ),
    )

    savePurchase(
        null,
        PurchaseData(
            daysAgo(15, 17), shop,
            listOf(
                PurchaseLineData(null, flower, null, 4_500, "USD"),
                PurchaseLineData(null, gummies, null, 2_200, "USD"),
                PurchaseLineData(null, cart, null, 3_800, "USD"),
            ),
        ),
    )

    suspend fun session(
        `when`: LocalTimeData,
        product: String,
        method: String,
        dose: Long?,
        unit: String?,
        intention: String,
        activity: String,
        effects: Map<String, Long>,
        rating: Int,
        moodBefore: Int,
        moodAfter: Int,
        onset: Int,
        duration: Int,
        symptom: Pair<String, String>? = null,
    ) = saveSession(
        null,
        SessionData(
            `when` = `when`,
            lines = listOf(ProductLineData(null, product, method, dose, unit, null, null, null)),
            activity = activity, intention = intention, moodBefore = moodBefore.toUByte(),
            social = "solo", effects = effects.map { (slug, value) -> TagIntensity(tag(slug), value) },
            onsetMin = onset.toUInt(), peakMin = null, durationMin = duration.toUInt(),
            moodAfter = moodAfter.toUByte(), rating = rating.toUByte(), wouldRepeat = rating >= 4,
            spot = null, symptoms = listOfNotNull(symptom?.let { (s, r) -> SymptomData(tag(s), r) }),
            attachments = emptyList(),
        ),
    )

    val sleep = mapOf("sleepy" to 4L, "relaxed" to 3L)
    session(daysAgo(14, 22), gummies, "edible", 5_000, "mg", "sleep", "reading in bed", sleep, 5, 2, 4, 50, 420, "sleep" to "helped")
    session(daysAgo(12, 13), cart, "vape_cart", 2_000, "puffs", "create", "sketching", mapOf("focused" to 3L, "creative" to 4L), 4, 3, 4, 5, 120)
    session(daysAgo(11, 22), gummies, "edible", 5_000, "mg", "sleep", "wind-down", sleep, 5, 2, 4, 55, 400, "sleep" to "helped")
    session(daysAgo(9, 23), gummies, "edible", 5_000, "mg", "sleep", "reading in bed", sleep, 5, 2, 4, 50, 420, "sleep" to "helped")
    session(daysAgo(10, 20), flower, "vaped_flower", 2_000, "hits", "relax", "movie night", mapOf("relaxed" to 4L, "euphoric" to 2L, "hungry" to 2L), 4, 3, 4, 10, 150)
    session(daysAgo(8, 14), cart, "vape_cart", 2_000, "puffs", "create", "writing", mapOf("focused" to 4L, "creative" to 3L), 4, 3, 4, 5, 110)
    session(daysAgo(7, 21), tincture, "tincture", 500, "ml", "relief", "after a long run", mapOf("relaxed" to 3L), 4, 2, 4, 30, 240, "pain" to "helped")
    session(daysAgo(6, 22), gummies, "edible", 5_000, "mg", "sleep", "reading in bed", sleep, 5, 3, 5, 45, 420, "sleep" to "helped")
    session(daysAgo(5, 19), flower, "vaped_flower", 3_000, "hits", "relax", "cooking", mapOf("relaxed" to 3L, "dry_mouth" to 2L, "anxious" to 1L), 3, 3, 3, 10, 140)
    session(daysAgo(4, 11), cart, "vape_cart", 3_000, "puffs", "create", "guitar practice", mapOf("creative" to 4L, "euphoric" to 3L, "focused" to 3L), 4, 3, 5, 5, 120)
    session(daysAgo(3, 23), gummies, "edible", 5_000, "mg", "sleep", "wind-down", sleep, 5, 3, 4, 45, 400, "sleep" to "helped")
    session(daysAgo(2, 22), gummies, "edible", 5_000, "mg", "sleep", "wind-down", mapOf("sleepy" to 5L, "relaxed" to 3L), 5, 2, 4, 50, 450, "sleep" to "helped")
    session(daysAgo(1, 20), flower, "vaped_flower", 2_000, "hits", "relax", "board games", mapOf("relaxed" to 4L, "euphoric" to 3L), 3, 3, 4, 10, 150)
    session(daysAgo(0, 9), tincture, "tincture", 250, "ml", "relief", "before work", mapOf("relaxed" to 2L, "focused" to 2L), 4, 3, 4, 25, 200, "anxiety" to "helped")
}

private const val HOUR = 3_600_000L
private const val DAY = 24 * HOUR
