// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import app.terpoir.core.BrandData
import app.terpoir.core.ProductData
import app.terpoir.core.StrainData
import app.terpoir.core.Vocabulary
import kotlinx.coroutines.launch

/** Creates or edits a product. Fields not shown here are kept as saved. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProductEditor(repo: JournalRepository, vocab: Vocabulary, id: String?, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var original by remember { mutableStateOf<ProductData?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var form by remember { mutableStateOf<String?>(null) }
    var brand by remember { mutableStateOf<String?>(null) }
    var strains by remember { mutableStateOf<List<String>>(emptyList()) }
    var thc by remember { mutableStateOf("") }
    var thcUnit by remember { mutableStateOf(vocab.labelUnits.first()) }
    var cbd by remember { mutableStateOf("") }
    var cbdUnit by remember { mutableStateOf(vocab.labelUnits.first()) }
    var batch by remember { mutableStateOf("") }
    var brands by remember { mutableStateOf<List<Entry<BrandData>>>(emptyList()) }
    var allStrains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var invalid by remember { mutableStateOf(false) }
    var newName by remember { mutableStateOf<NewKind?>(null) }

    LaunchedEffect(id) {
        brands = repo.brands()
        allStrains = repo.strains()
        id?.let { repo.product(it) }?.let { p ->
            original = p
            name = p.name
            form = p.form
            brand = p.brand
            strains = p.strains
            thc = p.thcMilli?.let(::formatMilli).orEmpty()
            p.thcUnit?.let { thcUnit = it }
            cbd = p.cbdMilli?.let(::formatMilli).orEmpty()
            p.cbdUnit?.let { cbdUnit = it }
            batch = p.batch.orEmpty()
        }
        loaded = true
    }
    if (!loaded) return

    EditorScaffold(
        title = stringResource(if (id == null) R.string.product_new else R.string.product_edit),
        onClose = onDone,
        saveEnabled = name.isNotBlank(),
        onSave = {
            val thcMilli = parseMilli(thc)
            val cbdMilli = parseMilli(cbd)
            if (thcMilli == Parsed.Invalid || cbdMilli == Parsed.Invalid) {
                invalid = true
                return@EditorScaffold
            }
            val thcValue = (thcMilli as? Parsed.Milli)?.value
            val cbdValue = (cbdMilli as? Parsed.Milli)?.value
            val base = original
            val data = ProductData(
                name = name.trim(),
                form = form,
                brand = brand,
                strains = strains,
                thcMilli = thcValue,
                thcUnit = thcUnit.takeIf { thcValue != null },
                cbdMilli = cbdValue,
                cbdUnit = cbdUnit.takeIf { cbdValue != null },
                terpenes = base?.terpenes.orEmpty(),
                batch = batch.trim().ifEmpty { null },
                packageDateDay = base?.packageDateDay,
                identifiers = base?.identifiers.orEmpty(),
            )
            scope.launch {
                repo.saveProduct(id, data)
                onDone()
            }
        },
    ) {
        OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.product_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        SectionTitle(stringResource(R.string.product_form))
        SingleChoiceChips(vocab.forms, form) { form = it }

        SectionTitle(stringResource(R.string.product_brand))
        val noBrand = stringResource(R.string.product_no_brand)
        Dropdown(
            label = stringResource(R.string.product_brand),
            options = listOf<Pair<String?, String>>(null to noBrand) + brands.map { it.id to it.data.name },
            selected = brand,
            onSelect = { brand = it },
        )
        OutlinedButton(onClick = { newName = NewKind.BRAND }) { Text(stringResource(R.string.brand_new)) }

        SectionTitle(stringResource(R.string.product_strains))
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (strain in allStrains) {
                val selected = strain.id in strains
                FilterChip(
                    selected = selected,
                    onClick = { strains = if (selected) strains - strain.id else strains + strain.id },
                    label = { Text(strain.data.name) },
                )
            }
        }
        OutlinedButton(onClick = { newName = NewKind.STRAIN }) { Text(stringResource(R.string.strain_new)) }

        SectionTitle(stringResource(R.string.product_label))
        LabelRow(stringResource(R.string.product_thc), thc, { thc = it }, vocab, thcUnit) { thcUnit = it }
        LabelRow(stringResource(R.string.product_cbd), cbd, { cbd = it }, vocab, cbdUnit) { cbdUnit = it }
        OutlinedTextField(batch, { batch = it }, label = { Text(stringResource(R.string.product_batch)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        if (invalid) Text(stringResource(R.string.error_amount), color = MaterialTheme.colorScheme.error)
        if (id != null) DeleteButton(R.string.product_delete, R.string.product_delete_body) { scope.launch { repo.delete(id); onDone() } }
    }

    newName?.let { kind ->
        NameDialog(
            title = stringResource(if (kind == NewKind.BRAND) R.string.brand_new else R.string.strain_new),
            onDismiss = { newName = null },
            onConfirm = { text ->
                newName = null
                scope.launch {
                    if (kind == NewKind.BRAND) {
                        brand = repo.saveBrand(null, BrandData(text))
                        brands = repo.brands()
                    } else {
                        val strainId = repo.saveStrain(null, StrainData(text, null, emptyList(), null))
                        allStrains = repo.strains()
                        strains = strains + strainId
                    }
                }
            },
        )
    }
}

private enum class NewKind { BRAND, STRAIN }

@Composable
private fun LabelRow(
    title: String,
    value: String,
    onValue: (String) -> Unit,
    vocab: Vocabulary,
    unit: String,
    onUnit: (String) -> Unit,
) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedTextField(
            value = value,
            onValueChange = onValue,
            label = { Text(title) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
            singleLine = true,
            modifier = Modifier.weight(1f),
        )
        Dropdown(
            label = stringResource(R.string.session_unit),
            options = vocab.labelUnits.map { it to label(it) },
            selected = unit,
            onSelect = onUnit,
            modifier = Modifier.weight(1f),
        )
    }
}

/** Creates or edits a strain. Terpenes are kept as saved (edited later). */
@Composable
fun StrainEditor(repo: JournalRepository, vocab: Vocabulary, id: String?, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var original by remember { mutableStateOf<StrainData?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var lineage by remember { mutableStateOf("") }
    var type by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(id) {
        id?.let { repo.strain(it) }?.let {
            original = it
            name = it.name
            lineage = it.lineage.orEmpty()
            type = it.strainType
        }
        loaded = true
    }
    if (!loaded) return
    EditorScaffold(
        title = stringResource(if (id == null) R.string.strain_new else R.string.strain_edit),
        onClose = onDone,
        saveEnabled = name.isNotBlank(),
        onSave = {
            val data = StrainData(name.trim(), lineage.trim().ifEmpty { null }, original?.terpenes.orEmpty(), type)
            scope.launch {
                repo.saveStrain(id, data)
                onDone()
            }
        },
    ) {
        OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.strain_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        OutlinedTextField(lineage, { lineage = it }, label = { Text(stringResource(R.string.strain_lineage)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        SectionTitle(stringResource(R.string.strain_type))
        SingleChoiceChips(vocab.strainTypes, type) { type = it }
        Text(stringResource(R.string.strain_type_note), style = MaterialTheme.typography.bodySmall)
        if (id != null) DeleteButton(R.string.strain_delete, R.string.strain_delete_body) { scope.launch { repo.delete(id); onDone() } }
    }
}

/** Creates or edits a brand. */
@Composable
fun BrandEditor(repo: JournalRepository, id: String?, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var loaded by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    LaunchedEffect(id) {
        id?.let { repo.brand(it) }?.let { name = it.name }
        loaded = true
    }
    if (!loaded) return
    EditorScaffold(
        title = stringResource(if (id == null) R.string.brand_new else R.string.brand_edit),
        onClose = onDone,
        saveEnabled = name.isNotBlank(),
        onSave = {
            scope.launch {
                repo.saveBrand(id, BrandData(name.trim()))
                onDone()
            }
        },
    ) {
        OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.brand_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        if (id != null) DeleteButton(R.string.brand_delete, R.string.brand_delete_body) { scope.launch { repo.delete(id); onDone() } }
    }
}

/** A destructive button behind a confirmation. */
@Composable
fun DeleteButton(label: Int, body: Int, onConfirm: () -> Unit) {
    var confirming by remember { mutableStateOf(false) }
    TextButton(onClick = { confirming = true }) { Text(stringResource(label), color = MaterialTheme.colorScheme.error) }
    if (confirming) {
        AlertDialog(
            onDismissRequest = { confirming = false },
            title = { Text(stringResource(label)) },
            text = { Text(stringResource(body)) },
            confirmButton = {
                TextButton(onClick = {
                    confirming = false
                    onConfirm()
                }) { Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton(onClick = { confirming = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}

@Composable
private fun NameDialog(title: String, onDismiss: () -> Unit, onConfirm: (String) -> Unit) {
    var text by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { OutlinedTextField(text, { text = it }, singleLine = true, label = { Text(stringResource(R.string.name)) }) },
        confirmButton = { TextButton(onClick = { onConfirm(text.trim()) }, enabled = text.isNotBlank()) { Text(stringResource(R.string.add)) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
