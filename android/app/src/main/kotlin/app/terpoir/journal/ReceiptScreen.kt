// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Checkbox
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.LocalTimeData
import app.terpoir.core.ProductData
import app.terpoir.core.PurchaseData
import app.terpoir.core.PurchaseLineData
import app.terpoir.core.SpotData
import app.terpoir.core.TagRefData
import app.terpoir.core.parseReceipt
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

/** One receipt line in the checklist. */
private class ItemDraft(name: String, price: String, product: String?) {
    var include by mutableStateOf(true)
    var name by mutableStateOf(name)
    var price by mutableStateOf(price)
    var product by mutableStateOf(product)
}

/** Where the purchase was: no shop, a saved spot, or a new Re-up Spot. */
private sealed interface ShopChoice {
    data object None : ShopChoice

    data class Existing(val id: String) : ShopChoice

    data object New : ShopChoice
}

private enum class Phase { CAMERA, READING, REVIEW }

/**
 * Receipt scan: one photo becomes a checklist of products with prices, the
 * date and the shop. The photo is read on the device and discarded at once;
 * only what the user confirms is saved, as one purchase.
 */
@Composable
fun ReceiptScreen(repo: JournalRepository, scan: ScanServices, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var phase by remember { mutableStateOf(Phase.CAMERA) }
    var shopText by remember { mutableStateOf("") }
    var shop by remember { mutableStateOf<ShopChoice>(ShopChoice.None) }
    var day by remember { mutableStateOf<Long?>(LocalDate.now().toEpochDay()) }
    var currency by remember { mutableStateOf(localCurrency()) }
    var noText by remember { mutableStateOf(false) }
    val items = remember { mutableStateListOf<ItemDraft>() }
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var spots by remember { mutableStateOf<List<Entry<SpotData>>>(emptyList()) }
    var invalid by remember { mutableStateOf(false) }

    when (phase) {
        Phase.CAMERA -> CameraScreen(
            mode = CaptureMode.PHOTO,
            title = stringResource(R.string.scan_receipt),
            hint = stringResource(R.string.scan_receipt_hint),
            onClose = onDone,
            onPhoto = { photo ->
                phase = Phase.READING
                scope.launch {
                    val text = scan.scanner.read(photo).text
                    products = repo.products()
                    spots = repo.spots()
                    if (text == null) {
                        noText = true
                    } else {
                        val receipt = parseReceipt(text)
                        receipt.dateDay?.let { day = it }
                        shopText = receipt.shop.orEmpty()
                        shop = when {
                            shopText.isBlank() -> ShopChoice.None
                            else -> repo.matchSpot(shopText)?.let { ShopChoice.Existing(it) } ?: ShopChoice.New
                        }
                        for (item in receipt.items) {
                            val match = repo.findProducts(item.name, null, emptyList()).firstOrNull()?.id
                            items += ItemDraft(item.name, formatMilli(item.priceMinor * 10), match)
                        }
                    }
                    phase = Phase.REVIEW
                }
            },
        )
        Phase.READING -> EditorScaffold(stringResource(R.string.scan_receipt), onDone, {}, saveEnabled = false) {
            Text(stringResource(R.string.scan_reading))
        }
        Phase.REVIEW -> EditorScaffold(
            title = stringResource(R.string.scan_receipt),
            onClose = onDone,
            saveEnabled = items.any { it.include && it.name.isNotBlank() },
            onSave = {
                val code = currency.trim().uppercase()
                val chosen = items.filter { it.include && it.name.isNotBlank() }
                val prices = chosen.map { if (it.price.isBlank()) null else parsePrice(it.price, code) }
                if (code.length != 3 || chosen.zip(prices).any { (item, price) -> item.price.isNotBlank() && price == null }) {
                    invalid = true
                    return@EditorScaffold
                }
                scope.launch {
                    val spotId = when (val choice = shop) {
                        ShopChoice.None -> null
                        is ShopChoice.Existing -> choice.id
                        ShopChoice.New -> repo.saveSpot(
                            null,
                            SpotData(shopText.trim(), listOf(TagRefData.Builtin("re_up")), null, null, null, null, false),
                        )
                    }
                    val lines = chosen.zip(prices).map { (item, price) ->
                        val product = item.product ?: repo.saveProduct(
                            null,
                            ProductData(item.name.trim(), null, null, emptyList(), null, null, null, null, emptyList(), null, null, emptyList()),
                        )
                        PurchaseLineData(null, product, null, price, code.takeIf { price != null })
                    }
                    val date = LocalDate.ofEpochDay(day ?: LocalDate.now().toEpochDay())
                    val zoned = date.atTime(LocalTime.NOON).atZone(ZoneId.systemDefault())
                    val `when` = LocalTimeData(zoned.toInstant().toEpochMilli(), (zoned.offset.totalSeconds / 60).toShort())
                    repo.savePurchase(null, PurchaseData(`when`, spotId, lines))
                    onDone()
                }
            },
        ) {
            if (noText) Text(stringResource(R.string.scan_no_text_engine), style = MaterialTheme.typography.bodySmall)
            Text(stringResource(R.string.receipt_check), style = MaterialTheme.typography.bodyMedium)
            SectionTitle(stringResource(R.string.receipt_shop))
            val newSpot = stringResource(R.string.receipt_new_spot, shopText.ifBlank { "…" })
            Dropdown(
                label = stringResource(R.string.receipt_shop),
                options = listOf<Pair<ShopChoice, String>>(ShopChoice.None to stringResource(R.string.receipt_no_shop)) +
                    spots.map { ShopChoice.Existing(it.id) to it.data.name } +
                    (ShopChoice.New to newSpot),
                selected = shop,
                onSelect = { shop = it },
            )
            if (shop == ShopChoice.New) {
                OutlinedTextField(shopText, { shopText = it }, label = { Text(stringResource(R.string.receipt_shop_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
            }
            DayField(stringResource(R.string.receipt_date), day) { day = it }
            OutlinedTextField(currency, { currency = it.take(3) }, label = { Text(stringResource(R.string.receipt_currency)) }, singleLine = true)

            SectionTitle(stringResource(R.string.receipt_items))
            if (items.isEmpty()) Text(stringResource(R.string.receipt_no_items))
            val newProduct = stringResource(R.string.receipt_new_product)
            for (item in items) {
                OutlinedCard(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Checkbox(checked = item.include, onCheckedChange = { item.include = it })
                            OutlinedTextField(item.name, { item.name = it }, label = { Text(stringResource(R.string.receipt_item)) }, singleLine = true, modifier = Modifier.weight(1f))
                        }
                        OutlinedTextField(
                            item.price,
                            { item.price = it },
                            label = { Text(stringResource(R.string.receipt_price)) },
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            singleLine = true,
                        )
                        Dropdown(
                            label = stringResource(R.string.receipt_product),
                            options = listOf<Pair<String?, String>>(null to newProduct) + products.map { it.id to it.data.name },
                            selected = item.product,
                            onSelect = { item.product = it },
                        )
                    }
                }
            }
            OutlinedButton(onClick = { items += ItemDraft("", "", null) }) { Text(stringResource(R.string.receipt_add_item)) }
            if (invalid) Text(stringResource(R.string.error_amount), color = MaterialTheme.colorScheme.error)
        }
    }
}

