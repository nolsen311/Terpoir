// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.ItemBreakdownData
import app.terpoir.core.PackageSizeData
import app.terpoir.core.ScanSuggestion

/**
 * One item card's state.
 *
 * @property rows the photo's rows it was read from, top to bottom; empty
 *   when typed in.
 */
internal class ItemDraft(val rows: List<Int>, text: String, price: String, quantity: UInt, breakdown: ItemBreakdownData?) {
    /** The item's text: the printed rows joined, or what was typed. */
    var text by mutableStateOf(text)
    var price by mutableStateOf(price)
    var quantity by mutableStateOf(quantity)
    var breakdown by mutableStateOf(breakdown)

    /** Whether the user changed the size ([sizeEdit]; null for none); otherwise it is as printed. */
    var sizeEdited by mutableStateOf(false)
    var sizeEdit by mutableStateOf<PackageSizeData?>(null)

    /** What a pack's amount covers, as the user chose it (`each` or `total`); null for the core's reading. */
    var per by mutableStateOf<String?>(null)

    /** Whether the size's fields are open. */
    var editingSize by mutableStateOf(false)

    /** The size before deciding what a pack's amount covers: the user's, else as printed. */
    fun baseSize(): PackageSizeData? = if (sizeEdited) sizeEdit else breakdown?.packageSize()

    /** The core's suggestions for [text], best first. */
    var suggestions by mutableStateOf<List<ScanSuggestion>>(emptyList())

    /** The product the line is linked to; null for a new product. */
    var product by mutableStateOf<String?>(null)

    /** Whether [product] was linked automatically, from a learned receipt name (the "matched" chip). */
    var matched by mutableStateOf(false)

    /** The new product's own name, brand and strain (IDs). */
    var newName by mutableStateOf("")
    var newBrand by mutableStateOf<String?>(null)
    var newStrain by mutableStateOf<String?>(null)

    /** Takes fresh suggestions after products changed (a merge): an unlinked line links to a product that alone learned it. */
    fun relinked(list: List<ScanSuggestion>) {
        suggestions = list
        if (product == null) {
            autoLinked(list)?.let {
                product = it
                matched = true
            }
        }
    }

    /** Takes the core's suggestions; the first time, links a learned match and pre-fills a new product. */
    fun suggested(list: List<ScanSuggestion>, first: Boolean, brands: Boolean, strains: Boolean) {
        suggestions = list
        if (!first) return
        autoLinked(list)?.let {
            product = it
            matched = true
        }
        val prefill = breakdown?.let { newProductPrefill(it, list, brands, strains) }
        newBrand = prefill?.brand
        newStrain = prefill?.strain
        newName = prefill?.name ?: ""
    }
}

/** A heading for a card, read as one by screen readers. */
@Composable
private fun CardTitle(text: String) {
    Text(text, style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
}

/**
 * The shop header card, first on the review: "Green Leaf ✓" when the
 * receipt's numbers name a saved spot, "Green Leaf?" with Yes and Change
 * for a name-only guess, and "Which shop is this?" otherwise, with every
 * way to answer as a visible button. [picker] is shown under the unknown
 * state's buttons (the photo or the list of spots, when open).
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ShopCard(
    state: ShopState,
    spotName: (String) -> String,
    headerName: String?,
    hasPhoto: Boolean,
    hasSpots: Boolean,
    onChange: (ShopState) -> Unit,
    onTapName: () -> Unit,
    onPickSpot: () -> Unit,
    picker: @Composable ColumnScope.() -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.receipt_shop), style = MaterialTheme.typography.labelLarge)
            val change: @Composable () -> Unit = {
                TextButton(onClick = { onChange(ShopState.Unknown) }) { Text(stringResource(R.string.receipt_shop_change)) }
            }
            when (state) {
                is ShopState.Recognised, is ShopState.Confirmed -> {
                    CardTitle(stringResource(R.string.receipt_shop_known, spotName(state.spot()!!)))
                    change()
                }
                is ShopState.Guessed -> {
                    CardTitle(stringResource(R.string.receipt_shop_guess, spotName(state.spot)))
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(onClick = { onChange(ShopState.Confirmed(state.spot)) }) { Text(stringResource(R.string.yes)) }
                        OutlinedButton(onClick = { onChange(ShopState.Unknown) }) { Text(stringResource(R.string.receipt_shop_change)) }
                    }
                }
                is ShopState.New -> {
                    CardTitle(stringResource(R.string.receipt_shop_new))
                    OutlinedTextField(
                        state.name,
                        { onChange(ShopState.New(it)) },
                        label = { Text(stringResource(R.string.receipt_shop_name)) },
                        singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    change()
                }
                ShopState.None -> {
                    CardTitle(stringResource(R.string.receipt_shop_not_recorded))
                    change()
                }
                ShopState.Unknown -> {
                    CardTitle(stringResource(R.string.receipt_shop_question))
                    if (!headerName.isNullOrBlank()) {
                        Text(stringResource(R.string.receipt_shop_says, headerName), style = MaterialTheme.typography.bodyMedium)
                    }
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        if (hasPhoto) OutlinedButton(onClick = onTapName) { Text(stringResource(R.string.receipt_shop_tap)) }
                        if (hasSpots) OutlinedButton(onClick = onPickSpot) { Text(stringResource(R.string.receipt_shop_pick)) }
                        OutlinedButton(onClick = { onChange(ShopState.New(headerName.orEmpty())) }) { Text(stringResource(R.string.receipt_shop_new)) }
                        TextButton(onClick = { onChange(ShopState.None) }) { Text(stringResource(R.string.receipt_no_shop)) }
                    }
                    picker()
                }
            }
        }
    }
}

/** "Part of the receipt is cut off. Retake?" with Retake and Keep. */
@Composable
internal fun CutOffCard(onRetake: () -> Unit, onKeep: () -> Unit) =
    RetakeCard(stringResource(R.string.receipt_cut_off), stringResource(R.string.receipt_keep), onRetake, onKeep)

/** A problem with a photo, [text], with Retake and [keep] (which dismisses it). */
@Composable
internal fun RetakeCard(text: String, keep: String, onRetake: () -> Unit, onKeep: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.errorContainer),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(text, style = MaterialTheme.typography.bodyLarge)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = onRetake) { Text(stringResource(R.string.receipt_retake)) }
                OutlinedButton(onClick = onKeep) { Text(keep) }
            }
        }
    }
}

/** What an item card needs from the screen beyond its own state. */
internal class ItemCardContext(
    val rowText: (Int) -> String,
    /** The card's size, with what a pack's amount covers decided. */
    val size: (ItemDraft) -> PackageSizeData?,
    val productLabel: (String) -> String,
    val products: List<Pair<String, String>>,
    val showPrice: Boolean,
    val showBrand: Boolean,
    val showStrain: Boolean,
)

/**
 * One item on the receipt: the printed line, its price, size, pack and
 * quantity, and what product it is. A product that learned this exact line
 * is linked with a "matched" chip that one tap undoes ([onUndoMatch]);
 * otherwise the core's suggestions are "?" chips, and "New product" is
 * pre-filled from the line. [onJoin] and [onSplit] (before the given row)
 * change how the printed rows group into items; null hides them.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun ItemCard(
    item: ItemDraft,
    context: ItemCardContext,
    onJoin: (() -> Unit)?,
    onSplit: (Int) -> Unit,
    onRemove: () -> Unit,
    onUndoMatch: () -> Unit,
    onTextChange: (String) -> Unit,
    onMerge: (Pair<String, String>) -> Unit,
) {
    OutlinedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            // The printed line, row by row, with "Split here" between rows.
            if (item.rows.isEmpty()) {
                OutlinedTextField(
                    item.text,
                    onTextChange,
                    label = { Text(stringResource(R.string.receipt_item)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            } else {
                item.rows.forEachIndexed { i, row ->
                    if (i > 0) {
                        TextButton(onClick = { onSplit(i) }) { Text(stringResource(R.string.receipt_split)) }
                    }
                    Text(context.rowText(row), style = MaterialTheme.typography.bodyLarge)
                }
            }
            val details = itemDetails(item.breakdown)
            if (details.isNotEmpty()) Text(details, style = MaterialTheme.typography.bodyMedium)
            SizeChoice(item, context.size(item))

            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (context.showPrice) {
                    OutlinedTextField(
                        item.price,
                        { item.price = it },
                        label = { Text(stringResource(R.string.receipt_price)) },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                        singleLine = true,
                        modifier = Modifier.width(128.dp),
                    )
                }
                QuantityStepper(item.quantity) { item.quantity = it }
            }

            ProductChoice(item, context, onUndoMatch)
            // Two products learned this line (after a sync): one tap to merge them.
            if (item.product == null) {
                learnedPair(item.suggestions)?.let { pair ->
                    Text(stringResource(R.string.receipt_learned_both), style = MaterialTheme.typography.bodyMedium)
                    OutlinedButton(onClick = { onMerge(pair) }) { Text(stringResource(R.string.receipt_merge)) }
                }
            }

            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (onJoin != null) OutlinedButton(onClick = onJoin) { Text(stringResource(R.string.receipt_join)) }
                TextButton(onClick = onRemove) { Text(stringResource(R.string.remove)) }
            }
        }
    }
}

/** "Flower · Rosin": the type and subtype the line's breakdown found. */
@Composable
private fun itemDetails(breakdown: ItemBreakdownData?): String {
    if (breakdown == null) return ""
    val parts = mutableListOf<String>()
    breakdown.form?.let { parts += label(it) }
    breakdown.subtype?.let { parts += label(it) }
    return parts.joinToString(" · ")
}

/** "0.5 g each · pack of 5", "3.5 g in all · pack of 10", "100 mg", "pack of 5". */
@Composable
internal fun sizeText(size: PackageSizeData): String {
    val parts = mutableListOf<String>()
    val milli = size.milli
    val unit = size.unit
    if (milli != null && unit != null) {
        val amount = sizeAmountText(milli, unit)
        parts += when (size.per) {
            PER_EACH -> stringResource(R.string.receipt_size_each, amount)
            PER_TOTAL -> stringResource(R.string.receipt_size_in_all, amount)
            else -> amount
        }
    }
    size.pack?.let { parts += pluralStringResource(R.plurals.receipt_pack, it.toInt(), it.toInt()) }
    return parts.joinToString(" · ")
}

/**
 * The card's size: "0.5 g each · pack of 5" with Each and In all chips
 * when there is a pack with an amount (the core reads the amount as each
 * piece's unless that is implausibly large for the product), and "Change
 * size" opening its amount, unit and pack.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SizeChoice(item: ItemDraft, size: PackageSizeData?) {
    if (size != null) Text(sizeText(size), style = MaterialTheme.typography.bodyMedium)
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        if (size?.milli != null && size.pack != null) {
            FilterChip(
                selected = size.per == PER_EACH,
                onClick = { item.per = PER_EACH },
                label = { Text(stringResource(R.string.receipt_per_each)) },
            )
            FilterChip(
                selected = size.per == PER_TOTAL,
                onClick = { item.per = PER_TOTAL },
                label = { Text(stringResource(R.string.receipt_per_in_all)) },
            )
        }
        TextButton(onClick = { item.editingSize = !item.editingSize }) {
            Text(stringResource(if (size == null) R.string.receipt_size_add else R.string.receipt_size_change))
        }
    }
    if (item.editingSize) SizeFields(size) { typed ->
        item.sizeEdited = true
        item.sizeEdit = typed
        item.editingSize = false
    }
}

/** A size's amount, unit (g, mg or ml) and pack count, to change; [onDone] with the typed size (null for none). */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SizeFields(size: PackageSizeData?, onDone: (PackageSizeData?) -> Unit) {
    var amount by remember(size) { mutableStateOf(size?.milli?.let(::formatMilli).orEmpty()) }
    var unit by remember(size) { mutableStateOf(size?.unit?.takeIf { it in SIZE_UNITS } ?: SIZE_UNITS.first()) }
    var pack by remember(size) { mutableStateOf(size?.pack?.toString().orEmpty()) }
    val valid = typedSizeValid(amount, pack)
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedTextField(
            amount,
            { amount = it },
            label = { Text(stringResource(R.string.receipt_size_amount)) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            singleLine = true,
            isError = !valid,
            modifier = Modifier.width(128.dp),
        )
        OutlinedTextField(
            pack,
            { pack = it },
            label = { Text(stringResource(R.string.receipt_size_pack)) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            singleLine = true,
            isError = !valid,
            modifier = Modifier.width(128.dp),
        )
    }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (u in SIZE_UNITS) {
            FilterChip(selected = unit == u, onClick = { unit = u }, label = { Text(u) })
        }
    }
    if (!valid) Text(stringResource(R.string.receipt_size_invalid), color = MaterialTheme.colorScheme.error)
    OutlinedButton(onClick = { onDone(typedSize(amount, unit, pack)) }, enabled = valid) { Text(stringResource(R.string.receipt_size_done)) }
}

/** How many were bought, with − and + buttons. */
@Composable
private fun QuantityStepper(quantity: UInt, onChange: (UInt) -> Unit) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        IconButton(onClick = { onChange(quantity - 1u) }, enabled = quantity > 1u) {
            Icon(painterResource(R.drawable.ic_remove), contentDescription = stringResource(R.string.receipt_quantity_less))
        }
        Text(stringResource(R.string.receipt_quantity, quantity.toInt()))
        IconButton(onClick = { onChange(quantity + 1u) }, enabled = quantity < MAX_QUANTITY) {
            Icon(painterResource(R.drawable.ic_add), contentDescription = stringResource(R.string.receipt_quantity_more))
        }
    }
}

/** The card's product: the linked one as a chip, or the suggestions and "New product". */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ProductChoice(item: ItemDraft, context: ItemCardContext, onUndoMatch: () -> Unit) {
    val linked = item.product
    if (linked != null) {
        val name = context.productLabel(linked)
        val description = if (item.matched) {
            stringResource(R.string.receipt_matched_description, name)
        } else {
            stringResource(R.string.receipt_linked_description, name)
        }
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilterChip(
                selected = true,
                onClick = {
                    if (item.matched) onUndoMatch()
                    item.product = null
                    item.matched = false
                },
                label = { Text(if (item.matched) stringResource(R.string.receipt_matched, name) else name) },
                modifier = Modifier.semantics { contentDescription = description },
            )
            TextButton(onClick = {
                item.product = null
                item.matched = false
            }) { Text(stringResource(R.string.receipt_shop_change)) }
        }
        return
    }

    val offered = offeredSuggestions(item.suggestions).filter {
        (it.recordType != "brand" || context.showBrand) && (it.recordType != "strain" || context.showStrain)
    }
    if (offered.isNotEmpty()) {
        Text(stringResource(R.string.receipt_is_it), style = MaterialTheme.typography.bodyMedium)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (s in offered) {
                when (s.recordType) {
                    "product" -> FilterChip(
                        selected = false,
                        onClick = {
                            item.product = s.id
                            // Picked by the user: it learns this line on save.
                            item.matched = false
                        },
                        label = { Text(stringResource(R.string.receipt_suggestion, context.productLabel(s.id).ifBlank { s.name })) },
                    )
                    "brand" -> {
                        val chosen = item.newBrand == s.id
                        FilterChip(
                            selected = chosen,
                            onClick = { item.newBrand = if (chosen) null else s.id },
                            label = { Text(stringResource(if (chosen) R.string.receipt_brand else R.string.receipt_suggestion_brand, s.name)) },
                        )
                    }
                    "strain" -> {
                        val chosen = item.newStrain == s.id
                        FilterChip(
                            selected = chosen,
                            onClick = {
                                item.newStrain = if (chosen) null else s.id
                                // With a strain the product needs no name of its own.
                                val leftover = item.breakdown?.name?.trim().orEmpty()
                                if (!chosen && item.newName == leftover) item.newName = ""
                                if (chosen && item.newName.isBlank()) item.newName = leftover
                            },
                            label = { Text(stringResource(if (chosen) R.string.receipt_strain else R.string.receipt_suggestion_strain, s.name)) },
                        )
                    }
                }
            }
        }
    }
    if (context.products.isNotEmpty()) {
        Dropdown(
            label = stringResource(R.string.receipt_other_product),
            options = listOf<Pair<String?, String>>(null to stringResource(R.string.receipt_new_product)) + context.products,
            selected = null,
            onSelect = { id -> if (id != null) item.product = id },
            modifier = Modifier.fillMaxWidth(),
        )
    }
    // "New product", pre-filled from the line.
    Text(stringResource(R.string.receipt_new_product), style = MaterialTheme.typography.titleSmall)
    val summary = listOfNotNull(
        item.newBrand?.let { id -> item.suggestions.firstOrNull { it.id == id }?.name },
        item.newStrain?.let { id -> item.suggestions.firstOrNull { it.id == id }?.name },
        item.breakdown?.form?.let { label(it) },
        item.breakdown?.subtype?.takeIf { item.breakdown?.form != null }?.let { label(it) },
    ).joinToString(" · ")
    if (summary.isNotEmpty()) Text(summary, style = MaterialTheme.typography.bodyMedium)
    OutlinedTextField(
        item.newName,
        { item.newName = it },
        label = { Text(stringResource(R.string.product_name)) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}

/** More than this on one line is a misread. */
private const val MAX_QUANTITY = 999u
