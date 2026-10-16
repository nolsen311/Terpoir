// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.graphics.Bitmap
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.CoreException
import app.terpoir.core.LocalTimeData
import app.terpoir.core.ProductData
import app.terpoir.core.PurchaseData
import app.terpoir.core.PurchaseLineData
import app.terpoir.core.ReceiptRowItemData
import app.terpoir.core.SpotData
import app.terpoir.core.TagRefData
import app.terpoir.core.TextRowData
import app.terpoir.core.describeReceiptItem
import app.terpoir.core.parseReceiptRowGroups
import app.terpoir.core.parseReceiptRows
import app.terpoir.core.receiptCutOff
import app.terpoir.scan.CameraScreen
import app.terpoir.scan.CaptureMode
import app.terpoir.scan.Scanner
import java.math.BigDecimal
import java.text.NumberFormat
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId
import java.util.Currency
import java.util.Locale
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** What the scan screens need: the scanner and the COA-link setting. */
class ScanServices(val scanner: Scanner, val linksAllowed: () -> Boolean)

/** A price in minor units, formatted in its currency. */
fun formatPrice(minorUnits: Long, currencyCode: String?): String {
    val currency = currencyCode?.let { runCatching { Currency.getInstance(it) }.getOrNull() }
    val digits = currency?.defaultFractionDigits?.coerceAtLeast(0) ?: 2
    val amount = BigDecimal.valueOf(minorUnits, digits)
    return if (currency == null) {
        "$amount ${currencyCode.orEmpty()}".trim()
    } else {
        NumberFormat.getCurrencyInstance().apply { this.currency = currency }.format(amount)
    }
}

/** Minor units from typed text in [currency], or null if it is not an amount. */
internal fun parsePrice(text: String, currency: String): Long? {
    val digits = runCatching { Currency.getInstance(currency).defaultFractionDigits }.getOrNull()?.coerceIn(0, 3) ?: 2
    val milli = (parseMilli(text) as? Parsed.Milli)?.value ?: return null
    var divisor = 1L
    repeat(3 - digits) { divisor *= 10 }
    return if (milli % divisor == 0L) milli / divisor else null
}

private fun localCurrency(): String =
    runCatching { Currency.getInstance(Locale.getDefault()).currencyCode }.getOrNull() ?: "USD"

private enum class Phase { CAMERA, READING, REVIEW }

/** What the shop card has open under "Which shop is this?". */
private enum class ShopPicker { PHOTO, SPOTS }

/**
 * Receipt scan: one photo becomes the shop, the date and a card per item,
 * saved as one purchase (docs/design-decisions.md, "Simpler review"). The
 * photo is read on the device, and the core reads the rows, matches the
 * shop and suggests products. A small copy of the photo stays in memory
 * while the user checks it, so they can tap lines the parser missed, and is
 * recycled when the screen closes; nothing is written to storage. Nothing
 * changes in the journal until Save: then the purchase is saved, the lines
 * linked to products teach them their printed names, and a confirmed shop
 * keeps the phone and licence numbers its receipt prints.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ReceiptScreen(repo: JournalRepository, scan: ScanServices, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    // "What I track": with prices and shops hidden, a receipt records what
    // was bought and when, nothing more.
    val tracking = LocalTracking.current
    val showPrice = tracking.shows(DataPoint.PRICE_AND_SHOP)
    val showBrand = tracking.shows(DataPoint.BRAND)
    val showStrain = tracking.shows(DataPoint.STRAIN)
    var phase by remember { mutableStateOf(Phase.CAMERA) }
    var shop by remember { mutableStateOf<ShopState>(if (showPrice) ShopState.Unknown else ShopState.None) }
    var headerName by remember { mutableStateOf<String?>(null) }
    var shopRow by remember { mutableStateOf<Int?>(null) }
    var shopPicker by remember { mutableStateOf<ShopPicker?>(null) }
    var day by remember { mutableStateOf<Long?>(LocalDate.now().toEpochDay()) }
    var currency by remember { mutableStateOf(localCurrency()) }
    var totalMinor by remember { mutableStateOf<Long?>(null) }
    var noText by remember { mutableStateOf(false) }
    var cutOff by remember { mutableStateOf(false) }
    var addingLine by remember { mutableStateOf(false) }
    var rowList by remember { mutableStateOf(false) }
    val items = remember { mutableStateListOf<ItemDraft>() }
    // Learned names the user undid ("matched" chip): product and line text.
    val undone = remember { mutableStateListOf<Pair<String, String>>() }
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var productLabels by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    var spots by remember { mutableStateOf<List<Entry<SpotData>>>(emptyList()) }
    var invalid by remember { mutableStateOf(false) }
    var photo by remember { mutableStateOf<Bitmap?>(null) }
    var rows by remember { mutableStateOf<List<TextRowData>>(emptyList()) }
    // The photo copy lives only while this screen does.
    DisposableEffect(Unit) { onDispose { photo?.recycle() } }

    fun draft(item: ReceiptRowItemData) = ItemDraft(
        rows = item.rows.map { it.toInt() },
        text = item.name,
        price = item.priceMinor?.takeIf { showPrice }?.let { priceText(it, currency.trim().uppercase()) }.orEmpty(),
        quantity = lineQuantity(item.quantity, item.breakdown),
        breakdown = item.breakdown,
    )

    suspend fun suggest(item: ItemDraft, first: Boolean) {
        if (item.text.isBlank()) return
        item.suggested(repo.suggestForReceiptItem(item.text, shop.spot()), first, showBrand, showStrain)
    }

    /** Reads the items again from the user's grouping of the rows; cards whose rows didn't change keep their edits. */
    fun regroup(groups: List<List<Int>>) {
        val scanned = parseReceiptRowGroups(rows, groups.map { g -> g.map { it.toUInt() } })
        val kept = items.filter { it.rows.isNotEmpty() }.associateBy { it.rows }
        val typed = items.filter { it.rows.isEmpty() }
        val next = scanned.items.map { kept[it.rows.map(UInt::toInt)] ?: draft(it) }
        items.clear()
        items += next + typed
        val fresh = next.filter { it !in kept.values }
        scope.launch { fresh.forEach { suggest(it, first = true) } }
    }

    fun groups() = items.filter { it.rows.isNotEmpty() }.map { it.rows }

    fun reset() {
        photo?.recycle()
        photo = null
        rows = emptyList()
        items.clear()
        undone.clear()
        shop = if (showPrice) ShopState.Unknown else ShopState.None
        headerName = null
        shopRow = null
        shopPicker = null
        totalMinor = null
        noText = false
        cutOff = false
        addingLine = false
        rowList = false
        invalid = false
        day = LocalDate.now().toEpochDay()
    }

    /** The user tapped the shop's name: a saved spot by that name is a guess to confirm, else a new shop. */
    fun pickShopRow(row: Int) {
        val text = rows[row].text.trim()
        shopRow = row
        items.removeAll { row in it.rows }
        scope.launch {
            shop = repo.matchSpot(text)?.let { ShopState.Guessed(it) } ?: ShopState.New(text)
            shopPicker = null
        }
    }

    // A different shop re-ranks the suggestions: what the user buys there comes first.
    val spotKey = shop.spot()
    LaunchedEffect(spotKey) {
        if (phase == Phase.REVIEW) items.toList().forEach { suggest(it, first = false) }
    }

    when (phase) {
        Phase.CAMERA -> TextEngineGate(scan.scanner, stringResource(R.string.scan_receipt), onDone) {
            CameraScreen(
                mode = CaptureMode.PHOTO,
                title = stringResource(R.string.scan_receipt),
                hint = stringResource(R.string.scan_receipt_hint),
                onClose = onDone,
                onPhoto = { shot ->
                    phase = Phase.READING
                    scope.launch {
                        val read = scan.scanner.readReceipt(shot)
                        products = repo.products()
                        productLabels = repo.productLabels()
                        spots = repo.spots()
                        photo = read.image
                        val found = read.rows
                        if (found == null) {
                            noText = true
                        } else {
                            rows = found
                            val receipt = parseReceiptRows(found)
                            receipt.dateDay?.let { day = it }
                            totalMinor = receipt.totalMinor
                            val edges = receiptCutOff(found, read.image.width, read.image.height)
                            cutOff = edges.left || edges.top || edges.right || edges.bottom
                            if (showPrice) {
                                // The shop first: its purchases rank the suggestions.
                                val match = repo.matchReceiptShop(found.map { it.text })
                                shop = shopState(match)
                                headerName = match.shop
                                shopRow = (match.shopRow ?: match.row)?.toInt()
                            }
                            items += receipt.items.map(::draft)
                            items.toList().forEach { suggest(it, first = true) }
                        }
                        phase = Phase.REVIEW
                    }
                },
            )
        }
        Phase.READING -> EditorScaffold(stringResource(R.string.scan_receipt), onDone, {}, saveEnabled = false) {
            Text(stringResource(R.string.scan_reading))
        }
        Phase.REVIEW -> EditorScaffold(
            title = stringResource(R.string.scan_receipt),
            onClose = onDone,
            saveEnabled = items.any { it.text.isNotBlank() },
            onSave = {
                val code = currency.trim().uppercase()
                val chosen = items.filter { it.text.isNotBlank() }
                val prices = chosen.map { if (!showPrice || it.price.isBlank()) null else parsePrice(it.price, code) }
                if ((showPrice && code.length != 3) || chosen.zip(prices).any { (item, price) -> showPrice && item.price.isNotBlank() && price == null }) {
                    invalid = true
                    return@EditorScaffold
                }
                scope.launch {
                    val choice = shop
                    val spotId = when (choice) {
                        is ShopState.New -> choice.name.trim().takeIf { it.isNotEmpty() }?.let {
                            repo.saveSpot(null, SpotData(it, listOf(TagRefData.Builtin("re_up")), null, null, null, null, false))
                        }
                        else -> choice.spot()
                    }
                    // A shop the user confirmed keeps its receipt's numbers, so it's recognised next time.
                    if (spotId != null && choice.remembers() && rows.isNotEmpty()) repo.rememberReceiptShop(spotId, rows.map { it.text })
                    val defaults = products.associate { it.id to it.data.defaultSize }
                    val linked = mutableSetOf<Pair<String, String>>()
                    val lines = chosen.zip(prices).map { (item, price) ->
                        val product = item.product ?: repo.saveProduct(
                            null,
                            newProductData(
                                item.newName,
                                item.newBrand.takeIf { showBrand },
                                item.newStrain.takeIf { showStrain },
                                item.breakdown,
                                item.text,
                            ),
                        )
                        // A printed line linked to a product links to it next time.
                        if (item.rows.isNotEmpty()) {
                            linked += product to item.text
                            if (!item.matched) learn { repo.learnReceiptName(product, item.text) }
                        }
                        PurchaseLineData(
                            null,
                            product,
                            null,
                            price,
                            code.takeIf { price != null },
                            // Size and pack as the line prints them, else the product's usual size.
                            size = item.breakdown?.packageSize() ?: defaults[product],
                            quantity = item.quantity,
                        )
                    }
                    for ((product, text) in undone) {
                        if (product to text !in linked) learn { repo.forgetReceiptName(product, text) }
                    }
                    val date = LocalDate.ofEpochDay(day ?: LocalDate.now().toEpochDay())
                    val zoned = date.atTime(LocalTime.NOON).atZone(ZoneId.systemDefault())
                    val `when` = LocalTimeData(zoned.toInstant().toEpochMilli(), (zoned.offset.totalSeconds / 60).toShort())
                    repo.savePurchase(null, PurchaseData(`when`, spotId, lines))
                    // Saved; no hand-off to a session (maintainer, 2026-10-15).
                    onDone()
                }
            },
        ) {
            if (noText) Text(stringResource(R.string.scan_no_text_engine), style = MaterialTheme.typography.bodySmall)
            val image = photo
            val itemRows = items.flatMap { it.rows }.toSet()
            val canTap = image != null && rows.isNotEmpty()

            if (showPrice) {
                ShopCard(
                    state = shop,
                    spotName = { id -> spots.firstOrNull { it.id == id }?.data?.name.orEmpty() },
                    headerName = headerName,
                    hasPhoto = canTap,
                    hasSpots = spots.isNotEmpty(),
                    onChange = { shop = it },
                    onTapName = { shopPicker = if (shopPicker == ShopPicker.PHOTO) null else ShopPicker.PHOTO },
                    onPickSpot = { shopPicker = if (shopPicker == ShopPicker.SPOTS) null else ShopPicker.SPOTS },
                ) {
                    when (shopPicker) {
                        ShopPicker.PHOTO -> if (image != null) {
                            Text(stringResource(R.string.receipt_shop_tap_hint), style = MaterialTheme.typography.bodySmall)
                            ReceiptPhotoView(
                                image = image,
                                rows = rows,
                                itemRows = itemRows,
                                shopRow = shopRow,
                                description = stringResource(R.string.receipt_photo),
                                atTop = true,
                                onTap = ::pickShopRow,
                            )
                            // The list alternative: the header's rows.
                            ReceiptRowList(
                                rows = rows,
                                indices = rows.indices.take(SHOP_ROWS),
                                label = { null },
                                enabled = { true },
                                onPick = ::pickShopRow,
                            )
                        }
                        ShopPicker.SPOTS -> Dropdown(
                            label = stringResource(R.string.receipt_my_spots),
                            options = listOf<Pair<String?, String>>(null to "") + spots.map { it.id to it.data.name },
                            selected = null,
                            onSelect = { id ->
                                if (id != null) {
                                    shop = ShopState.Confirmed(id)
                                    shopPicker = null
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                        )
                        null -> {}
                    }
                }
            }
            if (cutOff) {
                CutOffCard(
                    onRetake = {
                        reset()
                        phase = Phase.CAMERA
                    },
                    onKeep = { cutOff = false },
                )
            }
            DayField(stringResource(R.string.receipt_date), day) { day = it }
            if (showPrice) {
                OutlinedTextField(currency, { currency = it.take(3) }, label = { Text(stringResource(R.string.receipt_currency)) }, singleLine = true)
            }

            SectionTitle(stringResource(R.string.receipt_items))
            if (items.isEmpty()) Text(stringResource(R.string.receipt_no_items))
            val context = ItemCardContext(
                rowText = { rows[it].text },
                productLabel = { id -> productLabels[id] ?: products.firstOrNull { it.id == id }?.data?.name.orEmpty() },
                products = products.map { it.id to (productLabels[it.id] ?: it.data.name) }.sortedBy { it.second.lowercase() },
                showPrice = showPrice,
                showBrand = showBrand,
                showStrain = showStrain,
            )
            for (item in items.toList()) {
                key(item) {
                    if (item.rows.isEmpty()) {
                        // A typed line: broken down and matched as it's typed.
                        LaunchedEffect(item.text) {
                            delay(TYPING_PAUSE_MS)
                            item.breakdown = item.text.takeIf { it.isNotBlank() }?.let { describeReceiptItem(it) }
                            suggest(item, first = false)
                        }
                    }
                    val index = groups().indexOf(item.rows)
                    val joined = if (item.rows.isEmpty()) null else joinWithNext(groups(), index, rows.size, shopRow)
                    ItemCard(
                        item = item,
                        context = context,
                        onJoin = joined?.let { { regroup(it) } },
                        onSplit = { at -> splitGroup(groups(), index, at)?.let(::regroup) },
                        onRemove = { items.remove(item) },
                        onUndoMatch = { item.product?.let { undone += it to item.text } },
                        onTextChange = { item.text = it },
                    )
                }
            }
            if (showPrice) {
                totalMinor?.let { Text(stringResource(R.string.receipt_total, formatPrice(it, currency.trim().uppercase())), style = MaterialTheme.typography.bodyMedium) }
            }

            // "Missing a line?": pick it on the photo or in the list, or type it.
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (canTap) OutlinedButton(onClick = { addingLine = !addingLine }) { Text(stringResource(R.string.receipt_missing)) }
                OutlinedButton(onClick = { items += ItemDraft(emptyList(), "", "", 1u, null) }) { Text(stringResource(R.string.receipt_type_line)) }
            }
            if (addingLine && image != null) {
                Text(stringResource(R.string.receipt_missing_hint), style = MaterialTheme.typography.bodySmall)
                ReceiptPhotoView(
                    image = image,
                    rows = rows,
                    itemRows = itemRows,
                    shopRow = shopRow,
                    description = stringResource(R.string.receipt_photo),
                    atTop = false,
                    onTap = { row -> if (row !in itemRows && row != shopRow) regroup(addRowGroup(groups(), row)) },
                )
                TextButton(onClick = { rowList = !rowList }) {
                    Text(stringResource(if (rowList) R.string.receipt_rows_hide else R.string.receipt_rows_show))
                }
                if (rowList) {
                    val added = stringResource(R.string.receipt_row_added)
                    val shopLabel = stringResource(R.string.receipt_row_shop)
                    ReceiptRowList(
                        rows = rows,
                        indices = rows.indices.toList(),
                        label = { row -> if (row in itemRows) added else if (row == shopRow) shopLabel else null },
                        enabled = { row -> row !in itemRows && row != shopRow },
                        onPick = { row -> regroup(addRowGroup(groups(), row)) },
                    )
                }
            }
            if (invalid) Text(stringResource(R.string.error_amount), color = MaterialTheme.colorScheme.error)
        }
    }
}

/** Learning or forgetting a receipt name is a convenience: a refusal (a line over 200 characters, a deleted product) doesn't stop the save. */
private inline fun learn(block: () -> Unit) {
    try {
        block()
    } catch (_: CoreException) {
        // Nothing to learn from this line.
    }
}

/** How many header rows "Tap the shop's name" lists. */
private const val SHOP_ROWS = 12

/** How long typing pauses before a typed line is matched, in ms. */
private const val TYPING_PAUSE_MS = 400L
