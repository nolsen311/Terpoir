// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.LocalTimeData
import app.terpoir.core.PurchaseData
import app.terpoir.core.PurchaseLineData
import app.terpoir.core.SpotData
import app.terpoir.core.TagRefData
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId
import kotlinx.coroutines.launch

/**
 * Logs buying [product] by hand (maintainer, 2026-11-14: feature parity with
 * iOS, docs/design-decisions.md): its date, shop (a saved one or a new Re-up
 * Spot, as a receipt's is saved), price and how many. Shop and price only
 * while "What I track" shows them. Saved as one purchase line, as receipts
 * write them; [onSaved] after saving.
 */
@Composable
fun LogPurchaseDialog(
    repo: JournalRepository,
    product: String,
    shops: List<Entry<SpotData>>,
    showShopAndPrice: Boolean,
    onDismiss: () -> Unit,
    onSaved: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var day by remember { mutableStateOf<Long?>(LocalDate.now().toEpochDay()) }
    // null: no shop; NEW_SHOP: a new one, named below.
    var shop by remember { mutableStateOf<String?>(null) }
    var newShop by remember { mutableStateOf("") }
    var price by remember { mutableStateOf("") }
    var currency by remember { mutableStateOf(localCurrency()) }
    var quantity by remember { mutableStateOf(1) }
    var error by remember { mutableStateOf<Int?>(null) }

    fun save() {
        val code = currency.trim().uppercase()
        val minor = if (showShopAndPrice && price.isNotBlank()) {
            parsePrice(price, code) ?: run {
                error = R.string.purchase_price_invalid
                return
            }
        } else {
            null
        }
        if (shop == NEW_SHOP && newShop.isBlank()) {
            error = R.string.purchase_shop_name_needed
            return
        }
        scope.launch {
            val spot = when (shop) {
                NEW_SHOP -> repo.saveSpot(
                    null,
                    SpotData(newShop.trim(), listOf(TagRefData.Builtin("re_up")), null, null, null, null, false),
                )
                else -> shop
            }
            // Noon on the day, as receipts are dated.
            val zoned = LocalDate.ofEpochDay(day ?: LocalDate.now().toEpochDay())
                .atTime(LocalTime.NOON).atZone(ZoneId.systemDefault())
            val time = LocalTimeData(zoned.toInstant().toEpochMilli(), (zoned.offset.totalSeconds / 60).toShort())
            val line = PurchaseLineData(null, product, null, minor, code.takeIf { minor != null }, quantity = quantity.toUInt())
            repo.savePurchase(null, PurchaseData(time, spot.takeIf { showShopAndPrice }, listOf(line)))
            onSaved()
        }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.purchase_log)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                DayField(stringResource(R.string.receipt_date), day) { day = it }
                if (showShopAndPrice) {
                    Dropdown(
                        label = stringResource(R.string.receipt_shop),
                        options = listOf<Pair<String?, String>>(null to stringResource(R.string.receipt_no_shop)) +
                            shops.map { it.id to it.data.name } +
                            (NEW_SHOP to stringResource(R.string.receipt_shop_new)),
                        selected = shop,
                        onSelect = { shop = it },
                        modifier = Modifier.fillMaxWidth(),
                    )
                    if (shop == NEW_SHOP) {
                        OutlinedTextField(
                            newShop,
                            { newShop = it },
                            label = { Text(stringResource(R.string.receipt_shop_name)) },
                            singleLine = true,
                            modifier = Modifier.fillMaxWidth(),
                        )
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            price,
                            { price = it },
                            label = { Text(stringResource(R.string.purchase_price_optional)) },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                            modifier = Modifier.weight(1f),
                        )
                        OutlinedTextField(
                            currency,
                            { currency = it.take(3) },
                            label = { Text(stringResource(R.string.receipt_currency)) },
                            singleLine = true,
                            modifier = Modifier.width(96.dp),
                        )
                    }
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = { if (quantity > 1) quantity-- }, enabled = quantity > 1) {
                        Text(stringResource(R.string.receipt_quantity_less))
                    }
                    Text(stringResource(R.string.receipt_quantity, quantity), style = MaterialTheme.typography.titleMedium)
                    TextButton(onClick = { if (quantity < 99) quantity++ }) {
                        Text(stringResource(R.string.receipt_quantity_more))
                    }
                }
                error?.let { Text(stringResource(it), color = MaterialTheme.colorScheme.error) }
            }
        },
        confirmButton = { TextButton(onClick = ::save) { Text(stringResource(R.string.save)) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}

private const val NEW_SHOP = "+new"
