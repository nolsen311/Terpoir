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
import app.terpoir.core.IdentifierData
import app.terpoir.core.ProductData
import app.terpoir.core.ProductMatch
import app.terpoir.core.PurchaseData
import app.terpoir.core.StrainData
import app.terpoir.core.TagIntensity
import app.terpoir.core.Vocabulary
import app.terpoir.core.classifyCode
import app.terpoir.core.parseLabel
import app.terpoir.scan.CameraScreen
import app.terpoir.scan.CaptureMode
import app.terpoir.scan.DecodedCode
import kotlinx.coroutines.launch

/**
 * Creates or edits a product. Label and code scans fill fields for the user
 * to check; nothing from a scan is saved until they save. Saving a new
 * product that matches one already in the journal asks first.
 *
 * @param onOpenProduct replaces this editor with another product's.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProductEditor(
    repo: JournalRepository,
    vocab: Vocabulary,
    scan: ScanServices,
    id: String?,
    onOpenProduct: (String) -> Unit,
    onDone: () -> Unit,
) {
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
    var packageDay by remember { mutableStateOf<Long?>(null) }
    var terpenes by remember { mutableStateOf<List<TagIntensity>>(emptyList()) }
    var identifiers by remember { mutableStateOf<List<IdentifierData>>(emptyList()) }
    var purchases by remember { mutableStateOf<List<Entry<PurchaseData>>>(emptyList()) }
    var spotNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    var brands by remember { mutableStateOf<List<Entry<BrandData>>>(emptyList()) }
    var allStrains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var invalid by remember { mutableStateOf(false) }
    var newName by remember { mutableStateOf<NewKind?>(null) }
    var camera by remember { mutableStateOf<ProductScan?>(null) }
    var reading by remember { mutableStateOf(false) }
    var scanNote by remember { mutableStateOf<Int?>(null) }
    var known by remember { mutableStateOf<Pair<ProductMatch, String>?>(null) }
    var duplicate by remember { mutableStateOf<Pair<ProductMatch, ProductData>?>(null) }

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
            packageDay = p.packageDateDay
            terpenes = p.terpenes
            identifiers = p.identifiers
        }
        if (id != null) {
            purchases = repo.purchasesOf(id)
            spotNames = repo.spots().associate { it.id to it.data.name }
        }
        loaded = true
    }
    if (!loaded) return

    /** Adds a scanned code; a code already on another product offers to open it. */
    suspend fun addCode(code: DecodedCode) {
        val scanned = classifyCode(code.text, code.twoDimensional) ?: return
        if (identifiers.none { it.code == scanned.code }) {
            identifiers = identifiers + IdentifierData(scanned.code, scanned.kind)
        }
        val match = repo.findProducts(null, null, listOf(scanned.code)).firstOrNull { it.id != id }
        if (match != null) known = match to (repo.product(match.id)?.name.orEmpty())
    }

    camera?.let { kind ->
        val close = { camera = null }
        when (kind) {
            ProductScan.CODE -> CameraScreen(
                mode = CaptureMode.CODE,
                title = stringResource(R.string.scan_code),
                hint = stringResource(R.string.scan_code_hint),
                onClose = close,
                onCode = { code ->
                    camera = null
                    scope.launch { addCode(code) }
                },
            )
            ProductScan.LABEL -> CameraScreen(
                mode = CaptureMode.PHOTO,
                title = stringResource(R.string.scan_label),
                hint = stringResource(R.string.scan_label_hint),
                onClose = close,
                onPhoto = { photo ->
                    camera = null
                    reading = true
                    scope.launch {
                        val result = scan.scanner.read(photo)
                        result.code?.let { addCode(it) }
                        val text = result.text
                        scanNote = if (text == null) {
                            R.string.scan_no_text_engine
                        } else {
                            val label = parseLabel(text)
                            label.thcMilli?.let { thc = formatMilli(it) }
                            label.thcUnit?.let { thcUnit = it }
                            label.cbdMilli?.let { cbd = formatMilli(it) }
                            label.cbdUnit?.let { cbdUnit = it }
                            label.batch?.let { batch = it }
                            label.packageDateDay?.let { packageDay = it }
                            if (label.terpenes.isNotEmpty()) terpenes = label.terpenes
                            for (tag in label.packageTags) {
                                if (identifiers.none { it.code == tag }) identifiers = identifiers + IdentifierData(tag, "package_tag")
                            }
                            R.string.scan_label_filled
                        }
                        reading = false
                    }
                },
            )
        }
        return
    }

    fun data(): ProductData? {
        val thcMilli = parseMilli(thc)
        val cbdMilli = parseMilli(cbd)
        if (thcMilli == Parsed.Invalid || cbdMilli == Parsed.Invalid) return null
        val thcValue = (thcMilli as? Parsed.Milli)?.value
        val cbdValue = (cbdMilli as? Parsed.Milli)?.value
        return ProductData(
            name = name.trim(),
            form = form,
            brand = brand,
            strains = strains,
            thcMilli = thcValue,
            thcUnit = thcUnit.takeIf { thcValue != null },
            cbdMilli = cbdValue,
            cbdUnit = cbdUnit.takeIf { cbdValue != null },
            terpenes = terpenes,
            batch = batch.trim().ifEmpty { null },
            packageDateDay = packageDay,
            identifiers = identifiers,
        )
    }

    EditorScaffold(
        title = stringResource(if (id == null) R.string.product_new else R.string.product_edit),
        onClose = onDone,
        saveEnabled = name.isNotBlank() && !reading,
        onSave = {
            val data = data()
            if (data == null) {
                invalid = true
                return@EditorScaffold
            }
            scope.launch {
                val match = if (id == null) {
                    repo.findProducts(data.name, data.brand, data.identifiers.map { it.code }).firstOrNull()
                } else {
                    null
                }
                if (match != null) {
                    duplicate = match to data
                } else {
                    repo.saveProduct(id, data)
                    onDone()
                }
            }
        },
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(onClick = { camera = ProductScan.LABEL }, enabled = !reading) { Text(stringResource(R.string.scan_label)) }
            OutlinedButton(onClick = { camera = ProductScan.CODE }, enabled = !reading) { Text(stringResource(R.string.scan_code)) }
        }
        if (reading) Text(stringResource(R.string.scan_reading))
        scanNote?.let { Text(stringResource(it), style = MaterialTheme.typography.bodySmall) }

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
        if (terpenes.isNotEmpty()) {
            Text(stringResource(R.string.product_terpenes), style = MaterialTheme.typography.labelLarge)
            TerpeneList(terpenes) { t -> terpenes = terpenes - t }
        }
        OutlinedTextField(batch, { batch = it }, label = { Text(stringResource(R.string.product_batch)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        DayField(stringResource(R.string.product_package_date), packageDay) { packageDay = it }
        if (invalid) Text(stringResource(R.string.error_amount), color = MaterialTheme.colorScheme.error)

        if (identifiers.isNotEmpty()) {
            SectionTitle(stringResource(R.string.product_identifiers))
            IdentifierList(identifiers, scan.linksAllowed()) { i -> identifiers = identifiers - i }
        }

        if (purchases.isNotEmpty()) {
            SectionTitle(stringResource(R.string.product_purchases))
            for (purchase in purchases) {
                val line = purchase.data.lines.first { it.product == id }
                val price = line.priceMinorUnits?.let { formatPrice(it, line.priceCurrency) }
                val where = purchase.data.spot?.let { spotNames[it] }
                Text(
                    listOfNotNull(formatWhen(purchase.data.`when`.unixMs, purchase.data.`when`.utcOffsetMin), where, price)
                        .joinToString(" · "),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
        if (id != null) DeleteButton(R.string.product_delete, R.string.product_delete_body) { scope.launch { repo.delete(id); onDone() } }
    }

    known?.let { (match, knownName) ->
        AlertDialog(
            onDismissRequest = { known = null },
            title = { Text(stringResource(R.string.product_known_title)) },
            text = { Text(stringResource(R.string.product_known_body, knownName)) },
            confirmButton = {
                TextButton(onClick = {
                    known = null
                    onOpenProduct(match.id)
                }) { Text(stringResource(R.string.product_known_open)) }
            },
            dismissButton = { TextButton(onClick = { known = null }) { Text(stringResource(R.string.product_known_stay)) } },
        )
    }

    duplicate?.let { (match, data) ->
        AlertDialog(
            onDismissRequest = { duplicate = null },
            title = { Text(stringResource(R.string.product_duplicate_title)) },
            text = { Text(stringResource(if (match.byCode) R.string.product_duplicate_code else R.string.product_duplicate_name)) },
            confirmButton = {
                TextButton(onClick = {
                    duplicate = null
                    scope.launch {
                        val existing = repo.product(match.id)
                        if (existing != null) repo.saveProduct(match.id, mergeBatch(existing, data))
                        onDone()
                    }
                }) { Text(stringResource(R.string.product_duplicate_merge)) }
            },
            dismissButton = {
                TextButton(onClick = {
                    duplicate = null
                    scope.launch {
                        repo.saveProduct(null, data)
                        onDone()
                    }
                }) { Text(stringResource(R.string.product_duplicate_new)) }
            },
        )
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

private enum class ProductScan { LABEL, CODE }

/**
 * "Same product, new batch": keeps the existing product's name, brand and
 * strains, and takes the new batch, package date and label values where
 * the new entry has them. Scanned codes from both are kept.
 */
fun mergeBatch(existing: ProductData, new: ProductData): ProductData = existing.copy(
    form = existing.form ?: new.form,
    thcMilli = new.thcMilli ?: existing.thcMilli,
    thcUnit = if (new.thcMilli != null) new.thcUnit else existing.thcUnit,
    cbdMilli = new.cbdMilli ?: existing.cbdMilli,
    cbdUnit = if (new.cbdMilli != null) new.cbdUnit else existing.cbdUnit,
    terpenes = new.terpenes.ifEmpty { existing.terpenes },
    batch = new.batch ?: existing.batch,
    packageDateDay = new.packageDateDay ?: existing.packageDateDay,
    identifiers = existing.identifiers + new.identifiers.filter { n -> existing.identifiers.none { it.code == n.code } },
)

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
internal fun NameDialog(title: String, onDismiss: () -> Unit, onConfirm: (String) -> Unit) {
    var text by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { OutlinedTextField(text, { text = it }, singleLine = true, label = { Text(stringResource(R.string.name)) }) },
        confirmButton = { TextButton(onClick = { onConfirm(text.trim()) }, enabled = text.isNotBlank()) { Text(stringResource(R.string.add)) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}
