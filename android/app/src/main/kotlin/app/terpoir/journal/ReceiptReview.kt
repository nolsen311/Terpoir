// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.ItemBreakdownData
import app.terpoir.core.PackageSizeData
import app.terpoir.core.ProductData
import app.terpoir.core.ReceiptShopMatch
import app.terpoir.core.ScanSuggestion
import app.terpoir.core.TagRefData
import java.math.BigDecimal
import java.util.Currency

// The receipt review's small decisions, kept free of Compose so they can be
// unit tested. Reading, matching and suggesting all happen in the core;
// these only arrange what it returns for the screen.

/**
 * The size and pack count a receipt line prints, as a purchase line keeps
 * them (the units are the same), or null when it prints neither. What a
 * pack's amount covers is left undecided here: see [withPer].
 */
internal fun ItemBreakdownData.packageSize(): PackageSizeData? {
    val milli = sizeMilli?.takeIf { it > 0 && sizeUnit != null }
    val pack = pack?.takeIf { it > 0u }
    if (milli == null && pack == null) return null
    return PackageSizeData(milli, sizeUnit.takeIf { milli != null }, pack)
}

/** "3.5 g", "100 mg": a size amount as printed, without deciding what it covers. */
internal fun sizeAmountText(milli: Long, unit: String): String = "${formatMilli(milli)} $unit"

/** What a pack's amount covers: each piece's, or the whole pack's (the core's slugs). */
internal const val PER_EACH = "each"
internal const val PER_TOTAL = "total"

/** The units a card's size can be changed to. */
internal val SIZE_UNITS = listOf("g", "mg", "ml")

/**
 * [size] with what its amount covers decided (maintainer, 2026-10-15): the
 * user's [chosen] reading, else the core's [guess] (`package_amount_per`:
 * each piece's, unless that is implausibly large for the product). Only a
 * size with both an amount and a pack has one.
 */
internal fun withPer(size: PackageSizeData?, chosen: String?, guess: (PackageSizeData) -> String?): PackageSizeData? {
    if (size == null) return null
    val both = size.milli != null && size.pack != null
    return size.copy(per = if (both) chosen ?: guess(size.copy(per = null)) else null)
}

/**
 * A size typed on a card: an amount in [unit] and a pack count, either
 * optional. Null when nothing is typed or it isn't a size (an amount with
 * more than three decimals or of zero, a pack that isn't a whole number of
 * at least one); [typedSizeValid] tells those apart.
 */
internal fun typedSize(amount: String, unit: String, pack: String): PackageSizeData? {
    if (!typedSizeValid(amount, pack)) return null
    val milli = (parseMilli(amount) as? Parsed.Milli)?.value
    val count = pack.trim().toUIntOrNull()
    if (milli == null && count == null) return null
    return PackageSizeData(milli, unit.takeIf { milli != null }, count)
}

/** Whether a typed size is empty or a size [typedSize] reads. */
internal fun typedSizeValid(amount: String, pack: String): Boolean {
    val milli = parseMilli(amount)
    val amountOk = milli == Parsed.Empty || (milli is Parsed.Milli && milli.value > 0)
    val packOk = pack.isBlank() || (pack.trim().toUIntOrNull()?.let { it >= 1u } == true)
    return amountOk && packOk
}

/**
 * How many a line bought: a count the parser read before the name, else one
 * the breakdown found in the name (`BD X2`), else 1.
 */
internal fun lineQuantity(itemQuantity: UInt, breakdown: ItemBreakdownData): UInt = when {
    itemQuantity > 1u -> itemQuantity
    (breakdown.quantity ?: 0u) > 0u -> breakdown.quantity!!
    else -> 1u
}

/** A price in minor units as the price field shows it ("35.00"), in [currency]'s minor unit. */
internal fun priceText(minorUnits: Long, currency: String): String {
    val digits = runCatching { Currency.getInstance(currency).defaultFractionDigits }.getOrNull()?.coerceIn(0, 3) ?: 2
    return BigDecimal.valueOf(minorUnits, digits).toPlainString()
}

/**
 * The product a line links to without asking: only a product that learned
 * this exact line, and only when no other product learned it too (the core
 * marks both `close` then, to be offered as choices).
 */
internal fun autoLinked(suggestions: List<ScanSuggestion>): String? =
    suggestions.firstOrNull()?.takeIf { it.learned && !it.close && it.recordType == "product" }?.id

/**
 * Two products that learned this exact line (only after a sync), to offer
 * merging them: the first two, best first; null unless there are two.
 */
internal fun learnedPair(suggestions: List<ScanSuggestion>): Pair<String, String>? {
    val learned = suggestions.filter { it.learned && it.recordType == "product" }.map { it.id }.distinct()
    return if (learned.size >= 2) learned[0] to learned[1] else null
}

/** A merge the user confirmed in the review, done at Save: [other] goes into [keep]. */
internal data class PendingMerge(val keep: String, val other: String)

/** The product [id] ends up as once [merges] are done, in order (a merged-away product's keeper, followed along). */
internal fun mergedInto(id: String, merges: List<PendingMerge>): String {
    var current = id
    for (m in merges) if (m.other == current) current = m.keep
    return current
}

/** The suggestions a card offers as "?" chips: products, brands and strains, best first. */
internal fun offeredSuggestions(suggestions: List<ScanSuggestion>, limit: Int = 4): List<ScanSuggestion> =
    suggestions.filter { it.recordType in setOf("product", "brand", "strain") }.take(limit)

/** A brand and strain to pre-fill a new product with, and its name. */
internal data class NewProductPrefill(val brand: String?, val strain: String?, val name: String)

/** Match kinds strong enough to pre-fill a new product's brand or strain (the text names it in full). */
private val PREFILL_KINDS = setOf("exact", "words")

/**
 * What "New product" starts with for a line: the best brand and strain the
 * text names in full (an exact or whole-word match, so `BD` alone pre-fills
 * nothing), while those are tracked; the rest of the breakdown fills its
 * type and size. With a strain the product needs no name of its own
 * (names are optional); without one, the line's leftover name is it.
 */
internal fun newProductPrefill(
    breakdown: ItemBreakdownData,
    suggestions: List<ScanSuggestion>,
    brands: Boolean,
    strains: Boolean,
): NewProductPrefill {
    fun best(type: String) = suggestions.firstOrNull { it.recordType == type && it.matchKind in PREFILL_KINDS }?.id
    val brand = if (brands) best("brand") else null
    val strain = if (strains) best("strain") else null
    return NewProductPrefill(brand, strain, if (strain == null) breakdown.name.trim() else "")
}

/**
 * The new product a line creates. Its type, subtype and default size come
 * from the line ([size]: the card's, as the user left it); [fallback] (the printed text) names it when nothing else
 * would, since a product needs a name, a strain, a brand or a type.
 */
internal fun newProductData(
    name: String,
    brand: String?,
    strain: String?,
    breakdown: ItemBreakdownData?,
    fallback: String,
    size: PackageSizeData? = breakdown?.packageSize(),
): ProductData {
    val form = breakdown?.form
    val ownName = name.trim().ifEmpty { if (brand == null && strain == null && form == null) fallback.trim() else "" }
    return ProductData(
        name = ownName,
        form = form,
        brand = brand,
        strains = listOfNotNull(strain),
        thcMilli = null,
        thcUnit = null,
        cbdMilli = null,
        cbdUnit = null,
        terpenes = emptyList(),
        batch = null,
        packageDateDay = null,
        identifiers = emptyList(),
        shops = emptyList(),
        subtype = breakdown?.subtype?.takeIf { form != null }?.let { TagRefData.Builtin(it) },
        defaultSize = size,
    )
}

/**
 * Where the purchase was, as the shop header card shows it.
 */
internal sealed interface ShopState {
    /** "Which shop is this?" */
    data object Unknown : ShopState

    /** Matched by name only: "Green Leaf?" with Yes and Change. */
    data class Guessed(val spot: String) : ShopState

    /** Matched by a phone or licence number the spot is known by: "Green Leaf ✓". */
    data class Recognised(val spot: String) : ShopState

    /** The user said yes to a guess or picked the spot. */
    data class Confirmed(val spot: String) : ShopState

    /** A new Re-up Spot, created on save. */
    data class New(val name: String) : ShopState

    /** The user chose not to record the shop. */
    data object None : ShopState
}

/** The header card's first state, from the core's match. */
internal fun shopState(match: ReceiptShopMatch): ShopState {
    val spot = match.spot ?: return ShopState.Unknown
    return if (match.strong) ShopState.Recognised(spot) else ShopState.Guessed(spot)
}

/** The saved spot a state names, if any (a new shop has none until saved). */
internal fun ShopState.spot(): String? = when (this) {
    is ShopState.Guessed -> spot
    is ShopState.Recognised -> spot
    is ShopState.Confirmed -> spot
    else -> null
}

/**
 * Whether saving remembers the receipt's phone and licence numbers on the
 * shop: only when the user confirmed it, picked it or made it. A guess they
 * never confirmed is saved but teaches nothing.
 */
internal fun ShopState.remembers(): Boolean = this is ShopState.Confirmed || this is ShopState.New

// Item groups: each card's rows on the photo, top to bottom. "Join with
// next line" and "Split here" change them, and the core reads the items
// again from the new grouping (parse_receipt_row_groups).

/**
 * [groups] with the card at [index] joined to the row below its last one:
 * that row's whole card when it has one. Null when there is no row below,
 * or it is [reserved] (the shop's name).
 */
internal fun joinWithNext(groups: List<List<Int>>, index: Int, rowCount: Int, reserved: Int? = null): List<List<Int>>? {
    val group = groups.getOrNull(index)?.takeIf { it.isNotEmpty() } ?: return null
    val next = group.max() + 1
    if (next >= rowCount || next == reserved) return null
    val other = groups.indexOfFirst { next in it }
    val joined = (group + (if (other >= 0) groups[other] else listOf(next))).distinct().sorted()
    return groups.mapIndexedNotNull { i, g ->
        when (i) {
            index -> joined
            other -> null
            else -> g
        }
    }
}

/** [groups] with the card at [index] split before its [at]th row; null when that isn't inside it. */
internal fun splitGroup(groups: List<List<Int>>, index: Int, at: Int): List<List<Int>>? {
    val group = groups.getOrNull(index)?.sorted() ?: return null
    if (at <= 0 || at >= group.size) return null
    return groups.subList(0, index) + listOf(group.take(at), group.drop(at)) + groups.subList(index + 1, groups.size)
}

/** [groups] with [row] added as a card of its own, in printed order; unchanged if a card has it. */
internal fun addRowGroup(groups: List<List<Int>>, row: Int): List<List<Int>> {
    if (groups.any { row in it }) return groups
    val at = groups.indexOfFirst { it.isNotEmpty() && it.min() > row }
    return if (at < 0) groups + listOf(listOf(row)) else groups.subList(0, at) + listOf(listOf(row)) + groups.subList(at, groups.size)
}

/**
 * The receipt photo's zoom and pan: content is drawn scaled by [scale]
 * then moved by [x], [y] (pixels), inside a viewport of [width] × [height].
 * At scale 1 the photo fits the viewport ("Fit").
 */
internal data class PhotoZoom(val scale: Float = 1f, val x: Float = 0f, val y: Float = 0f) {
    /** Zoomed by [factor] around the viewport point ([cx], [cy]), which stays put. */
    fun zoomed(factor: Float, cx: Float, cy: Float, width: Float, height: Float): PhotoZoom {
        val next = (scale * factor).coerceIn(1f, MAX_ZOOM)
        val ratio = next / scale
        return PhotoZoom(next, cx - (cx - x) * ratio, cy - (cy - y) * ratio).clamped(width, height)
    }

    /** Moved by ([dx], [dy]). */
    fun panned(dx: Float, dy: Float, width: Float, height: Float): PhotoZoom = copy(x = x + dx, y = y + dy).clamped(width, height)

    /** Kept so the viewport never shows past the content's edges. */
    fun clamped(width: Float, height: Float): PhotoZoom = copy(
        x = x.coerceIn((width - width * scale).coerceAtMost(0f), 0f),
        y = y.coerceIn((height - height * scale).coerceAtMost(0f), 0f),
    )

    /** The unzoomed content point under the viewport point ([px], [py]). */
    fun toContent(px: Float, py: Float): Pair<Float, Float> = (px - x) / scale to (py - y) / scale

    companion object {
        /** How far + and pinching can zoom in. */
        const val MAX_ZOOM = 8f

        /** One press of + or −. */
        const val STEP = 1.5f

        /**
         * Zoomed so the fitted photo ([imageLeft], [imageTop], [imageWidth]
         * wide inside the viewport) fills the viewport's width, showing its
         * top: where a receipt prints the shop's name.
         */
        fun top(imageLeft: Float, imageTop: Float, imageWidth: Float, width: Float, height: Float): PhotoZoom {
            if (imageWidth <= 0f) return PhotoZoom()
            val scale = (width / imageWidth).coerceIn(1f, MAX_ZOOM)
            return PhotoZoom(scale, -imageLeft * scale, -imageTop * scale).clamped(width, height)
        }
    }
}

// A long receipt's second photo ("Add the rest of the receipt"): the core
// joins its rows to the first photo's (join_receipt_photos), and the review
// works on the joined rows. Each photo still draws its own rows' boxes, so
// a joined row's index has to be found on the photo it came from.

/**
 * Which of the second photo's own rows are in the joined rows, and where:
 * pairs of (row on the second photo, joined row), for the rows both photos
 * show (counted from the first photo in the joined rows) and the rows below
 * them. The joined rows are the first [fromFirst] rows of the first photo,
 * then the second photo's last `joinedCount - fromFirst` rows; the second
 * photo's rows above the overlap (cut by its top edge) are in neither.
 */
internal fun secondPhotoRows(secondCount: Int, joinedCount: Int, fromFirst: Int, overlap: Int): List<Pair<Int, Int>> {
    val below = joinedCount - fromFirst
    val start = secondCount - below
    if (below < 0 || start < 0 || overlap < 0 || overlap > fromFirst) return emptyList()
    return ((start - overlap).coerceAtLeast(0) until secondCount).map { it to it - start + fromFirst }
}

/**
 * The cards after the second photo is joined, as groups of joined rows in
 * printed order: the user's cards whose rows are all among the first
 * photo's first [fromFirst] rows (those rows are unchanged, so the cards
 * keep their edits, and a card the user removed stays removed), then the
 * items [parsed] from the joined rows that reach into the second photo. An
 * item whose name wraps across the seam replaces the card it continues.
 */
internal fun groupsAfterJoin(user: List<List<Int>>, parsed: List<List<Int>>, fromFirst: Int): List<List<Int>> {
    val added = parsed.filter { g -> g.any { it >= fromFirst } }
    val kept = user.filter { g -> g.isNotEmpty() && g.all { it < fromFirst } && added.none { a -> a.any { it in g } } }
    return (kept + added).sortedBy { it.min() }
}
