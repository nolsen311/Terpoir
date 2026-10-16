// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.BrandData
import app.terpoir.core.CannabinoidAmountData
import app.terpoir.core.IdentifierData
import app.terpoir.core.ProductData
import app.terpoir.core.ProductMatch
import app.terpoir.core.PurchaseData
import app.terpoir.core.SpotData
import app.terpoir.core.StrainData
import app.terpoir.core.TagData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData
import app.terpoir.core.TerpeneLevelData
import app.terpoir.core.Vocabulary
import app.terpoir.core.classifyCode
import app.terpoir.core.parseLabel
import app.terpoir.core.subtypesForForm
import app.terpoir.scan.CameraScreen
import app.terpoir.scan.CaptureMode
import app.terpoir.scan.DecodedCode
import app.terpoir.share.ProductShare
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
    val tracking = LocalTracking.current
    var original by remember { mutableStateOf<ProductData?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var form by remember { mutableStateOf<String?>(null) }
    // Optional, under the type: built-in or the user's own (a tag).
    var subtype by remember { mutableStateOf<TagRefData?>(null) }
    var customSubtypes by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var addingSubtype by remember { mutableStateOf(false) }
    var brand by remember { mutableStateOf<String?>(null) }
    var strains by remember { mutableStateOf<List<String>>(emptyList()) }
    var thc by remember { mutableStateOf("") }
    var thcUnit by remember { mutableStateOf(vocab.labelUnits.first()) }
    var cbd by remember { mutableStateOf("") }
    var cbdUnit by remember { mutableStateOf(vocab.labelUnits.first()) }
    // Other label cannabinoids (CBN, CBG, THCa, CBDV), as typed.
    var others by remember { mutableStateOf<List<OtherAmount>>(emptyList()) }
    var batch by remember { mutableStateOf("") }
    var packageDay by remember { mutableStateOf<Long?>(null) }
    var terpenes by remember { mutableStateOf<List<TagIntensity>>(emptyList()) }
    var identifiers by remember { mutableStateOf<List<IdentifierData>>(emptyList()) }
    // Learned receipt names ("Seen on receipts as"); each delete takes effect at once.
    var receiptNames by remember { mutableStateOf<List<String>>(emptyList()) }
    var shops by remember { mutableStateOf<List<String>>(emptyList()) }
    var reUpSpots by remember { mutableStateOf<List<Entry<SpotData>>>(emptyList()) }
    var purchases by remember { mutableStateOf<List<Entry<PurchaseData>>>(emptyList()) }
    var spotNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    var brands by remember { mutableStateOf<List<Entry<BrandData>>>(emptyList()) }
    var allStrains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var invalid by remember { mutableStateOf(false) }
    var newBrand by remember { mutableStateOf(false) }
    var camera by remember { mutableStateOf<ProductScan?>(null) }
    var reading by remember { mutableStateOf(false) }
    var scanNote by remember { mutableStateOf<Int?>(null) }
    var known by remember { mutableStateOf<Pair<ProductMatch, String>?>(null) }
    var duplicate by remember { mutableStateOf<Pair<ProductMatch, ProductData>?>(null) }
    var sharing by remember { mutableStateOf(false) }
    // "Merge with…": the other products to pick from, the one picked (in
    // the dialog), and the merge confirmed, done at Save (maintainer,
    // 2026-10-16) so closing the editor cancels it.
    var mergeChoices by remember { mutableStateOf<List<Pair<String, String>>?>(null) }
    var mergeWith by remember { mutableStateOf<EditorMerge?>(null) }
    var pendingMerge by remember { mutableStateOf<EditorMerge?>(null) }

    LaunchedEffect(id) {
        brands = repo.brands()
        allStrains = repo.strains()
        customSubtypes = repo.tags(SUBTYPE_CATEGORY)
        id?.let { repo.product(it) }?.let { p ->
            original = p
            name = p.name
            form = p.form
            subtype = p.subtype
            brand = p.brand
            strains = p.strains
            thc = p.thcMilli?.let(::formatMilli).orEmpty()
            p.thcUnit?.let { thcUnit = it }
            cbd = p.cbdMilli?.let(::formatMilli).orEmpty()
            p.cbdUnit?.let { cbdUnit = it }
            others = p.cannabinoids.map { OtherAmount(it.cannabinoid, formatMilli(it.milli), it.unit) }
            batch = p.batch.orEmpty()
            packageDay = p.packageDateDay
            terpenes = p.terpenes
            identifiers = p.identifiers
            shops = p.shops
            receiptNames = p.receiptNames
        }
        val spots = repo.spots()
        spotNames = spots.associate { it.id to it.data.name }
        reUpSpots = spots.filter { s -> s.data.purposes.any { it == TagRefData.Builtin("re_up") } || s.id in shops }
        if (id != null) purchases = repo.purchasesOf(id)
        loaded = true
    }
    if (!loaded) return
    if (sharing && id != null) {
        ProductShare(repo, id) { sharing = false }
        return
    }

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
            ProductScan.LABEL -> TextEngineGate(scan.scanner, stringResource(R.string.scan_label), close) {
                CameraScreen(
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
                                // Nothing is read into a hidden data point ("What I track").
                                if (tracking.shows(DataPoint.POTENCY)) {
                                    label.thcMilli?.let { thc = formatMilli(it) }
                                    label.thcUnit?.let { thcUnit = it }
                                    label.cbdMilli?.let { cbd = formatMilli(it) }
                                    label.cbdUnit?.let { cbdUnit = it }
                                }
                                if (tracking.shows(DataPoint.BATCH)) {
                                    label.batch?.let { batch = it }
                                    label.packageDateDay?.let { packageDay = it }
                                }
                                if (tracking.shows(DataPoint.TERPENES) && label.terpenes.isNotEmpty()) terpenes = label.terpenes
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
        }
        return
    }

    fun data(): ProductData? {
        val thcMilli = parseMilli(thc)
        val cbdMilli = parseMilli(cbd)
        if (thcMilli == Parsed.Invalid || cbdMilli == Parsed.Invalid) return null
        val cannabinoids = mutableListOf<CannabinoidAmountData>()
        for (other in others) {
            when (val parsed = parseMilli(other.text)) {
                Parsed.Invalid -> return null
                is Parsed.Milli -> cannabinoids += CannabinoidAmountData(other.cannabinoid, parsed.value, other.unit)
                Parsed.Empty -> Unit // Cleared: removed.
            }
        }
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
            shops = shops,
            cannabinoids = cannabinoids,
            subtype = subtype,
            // Not edited here yet: kept as saved.
            defaultSize = original?.defaultSize,
            // Learned from receipts, and forgotten here one by one.
            receiptNames = receiptNames,
        )
    }

    EditorScaffold(
        title = stringResource(if (id == null) R.string.product_new else R.string.product_edit),
        onClose = onDone,
        // The name is optional (maintainer, 2026-10-15): a product can be
        // known by its strain, brand or type instead; it needs one of them.
        saveEnabled = !reading && (name.isNotBlank() || strains.isNotEmpty() || brand != null || form != null),
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
                    // The edits are saved first, so a merge takes them in.
                    pendingMerge?.let { m ->
                        if (id != null) {
                            if (m.keepThis) repo.mergeProducts(id, m.otherId) else repo.mergeProducts(m.otherId, id)
                        }
                    }
                    onDone()
                }
            }
        },
        // One "Scan" button for the label or the package code (maintainer, 2026-10-15).
        floatingActionButton = {
            var choosing by remember { mutableStateOf(false) }
            Box {
                FloatingActionButton(onClick = { if (!reading) choosing = true }) {
                    Icon(painterResource(R.drawable.ic_scan), contentDescription = stringResource(R.string.product_scan))
                }
                DropdownMenu(expanded = choosing, onDismissRequest = { choosing = false }) {
                    DropdownMenuItem(text = { Text(stringResource(R.string.scan_label)) }, onClick = {
                        choosing = false
                        camera = ProductScan.LABEL
                    })
                    DropdownMenuItem(text = { Text(stringResource(R.string.scan_code)) }, onClick = {
                        choosing = false
                        camera = ProductScan.CODE
                    })
                }
            }
        },
    ) {
        if (reading) Text(stringResource(R.string.scan_reading))
        scanNote?.let { Text(stringResource(it), style = MaterialTheme.typography.bodySmall) }

        OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.product_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        SectionTitle(stringResource(R.string.product_form))
        SingleChoiceChips(vocab.forms, form) { picked ->
            // A subtype belongs to one type: changing the type clears it.
            if (picked != form) subtype = null
            form = picked
        }
        form?.let { f ->
            SubtypeChips(
                builtins = subtypesForForm(f),
                custom = customSubtypes.filter { it.data.form == f },
                selected = subtype,
                onSelect = { subtype = it },
                onAdd = { addingSubtype = true },
            )
        }
        if (addingSubtype) {
            form?.let { f ->
                SubtypeDialog(
                    form = f,
                    methods = vocab.methods,
                    onDismiss = { addingSubtype = false },
                    onConfirm = { subtypeName, method ->
                        addingSubtype = false
                        scope.launch {
                            val tagId = repo.saveTag(null, TagData(subtypeName, SUBTYPE_CATEGORY, form = f, method = method))
                            customSubtypes = repo.tags(SUBTYPE_CATEGORY)
                            subtype = TagRefData.Custom(tagId)
                        }
                    },
                )
            }
        }

        // "What I track": hidden data points are left out here; their saved
        // values are carried unchanged.
        if (tracking.shows(DataPoint.BRAND)) {
            // The field's own label names it; no section title above it
            // (maintainer, 2026-10-16).
            val noBrand = stringResource(R.string.product_no_brand)
            Dropdown(
                label = stringResource(R.string.product_brand),
                options = listOf<Pair<String?, String>>(null to noBrand) + brands.map { it.id to it.data.name },
                selected = brand,
                onSelect = { brand = it },
            )
            OutlinedButton(onClick = { newBrand = true }) { Text(stringResource(R.string.brand_new)) }
        }

        if (tracking.shows(DataPoint.STRAIN)) {
            // A long strain list is searched, not shown whole (maintainer,
            // 2026-10-16): the chosen strains, then a search to add more.
            SectionTitle(stringResource(R.string.product_strains))
            SearchPicker(
                all = allStrains.map { it.id to it.data.name },
                selected = strains,
                searchLabel = stringResource(R.string.strain_search),
                missingLabel = stringResource(R.string.strain_parent_missing),
                onAdd = { strains = strains + it },
                onRemove = { strains = strains - it },
                newLabel = { stringResource(R.string.strain_new_named, it) },
                onNew = { typed ->
                    scope.launch {
                        val strainId = repo.saveStrain(null, StrainData(typed, null, emptyList(), null, emptyList(), emptyList(), emptyList()))
                        allStrains = repo.strains()
                        strains = strains + strainId
                    }
                },
            )
        }

        val showPotency = tracking.shows(DataPoint.POTENCY)
        val showTerpenes = tracking.shows(DataPoint.TERPENES) && terpenes.isNotEmpty()
        val showBatch = tracking.shows(DataPoint.BATCH)
        if (showPotency || showTerpenes || showBatch) SectionTitle(stringResource(R.string.product_label))
        if (showPotency) {
            LabelRow(stringResource(R.string.product_thc), thc, { thc = it }, vocab, thcUnit) { thcUnit = it }
            LabelRow(stringResource(R.string.product_cbd), cbd, { cbd = it }, vocab, cbdUnit) { cbdUnit = it }
            for ((i, other) in others.withIndex()) {
                LabelRow(
                    label(other.cannabinoid),
                    other.text,
                    { text -> others = others.toMutableList().also { it[i] = other.copy(text = text) } },
                    vocab,
                    other.unit,
                ) { unit -> others = others.toMutableList().also { it[i] = other.copy(unit = unit) } }
            }
            // The rest, to add; a cleared amount is dropped when saved.
            val missing = vocab.cannabinoids.filter { c -> others.none { it.cannabinoid == c } }
            if (missing.isNotEmpty()) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (c in missing) {
                        AssistChip(
                            onClick = { others = others + OtherAmount(c, "", vocab.labelUnits.first()) },
                            label = { Text(stringResource(R.string.product_add_cannabinoid, label(c))) },
                        )
                    }
                }
            }
        }
        if (showTerpenes) {
            Text(stringResource(R.string.product_terpenes), style = MaterialTheme.typography.labelLarge)
            TerpeneList(terpenes) { t -> terpenes = terpenes - t }
        }
        if (showBatch) {
            OutlinedTextField(batch, { batch = it }, label = { Text(stringResource(R.string.product_batch)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
            DayField(stringResource(R.string.product_package_date), packageDay) { packageDay = it }
        }
        if (invalid) Text(stringResource(R.string.error_amount), color = MaterialTheme.colorScheme.error)

        if (identifiers.isNotEmpty()) {
            SectionTitle(stringResource(R.string.product_identifiers))
            IdentifierList(identifiers, scan.linksAllowed()) { i -> identifiers = identifiers - i }
        }

        // "Seen on receipts as": the receipt lines that link here on their
        // own, each with a delete (docs/design-decisions.md, "Simpler review").
        if (id != null && receiptNames.isNotEmpty()) {
            SectionTitle(stringResource(R.string.product_receipt_names))
            Text(stringResource(R.string.product_receipt_names_hint), style = MaterialTheme.typography.bodySmall)
            for (text in receiptNames) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(text, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    IconButton(onClick = {
                        scope.launch {
                            repo.forgetReceiptName(id, text)
                            receiptNames = receiptNames - text
                        }
                    }) {
                        Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.product_receipt_name_forget, text))
                    }
                }
            }
        }

        // Where it is bought: receipts add their shop; the user can add or
        // remove one at any time.
        if (reUpSpots.isNotEmpty() && tracking.shows(DataPoint.PRICE_AND_SHOP)) {
            SectionTitle(stringResource(R.string.product_shops))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (spot in reUpSpots) {
                    val selected = spot.id in shops
                    FilterChip(
                        selected = selected,
                        onClick = { shops = if (selected) shops - spot.id else shops + spot.id },
                        label = { Text(spot.data.name) },
                    )
                }
            }
        }

        if (purchases.isNotEmpty() && tracking.shows(DataPoint.PURCHASES)) {
            SectionTitle(stringResource(R.string.product_purchases))
            val showPrice = tracking.shows(DataPoint.PRICE_AND_SHOP)
            for (purchase in purchases) {
                val line = purchase.data.lines.first { it.product == id }
                val price = line.priceMinorUnits?.takeIf { showPrice }?.let { formatPrice(it, line.priceCurrency) }
                val where = purchase.data.spot?.takeIf { showPrice }?.let { spotNames[it] }
                Text(
                    listOfNotNull(formatWhen(purchase.data.`when`.unixMs, purchase.data.`when`.utcOffsetMin), where, price)
                        .joinToString(" · "),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
        if (id != null) {
            // Shares what is saved, not unsaved edits.
            OutlinedButton(onClick = { sharing = true }) { Text(stringResource(R.string.share)) }
            // "Merge with…": another record of the same product joins this one.
            OutlinedButton(onClick = {
                scope.launch {
                    val labels = repo.productLabels()
                    mergeChoices = repo.products().filter { it.id != id }
                        .map { it.id to (labels[it.id] ?: it.data.name) }
                        .sortedBy { it.second.lowercase() }
                }
            }) { Text(stringResource(R.string.product_merge)) }
            mergeChoices?.let { choices ->
                if (choices.isEmpty()) {
                    Text(stringResource(R.string.product_merge_none), style = MaterialTheme.typography.bodySmall)
                } else {
                    Dropdown(
                        label = stringResource(R.string.product_merge_pick),
                        options = listOf<Pair<String?, String>>(null to "") + choices,
                        selected = null,
                        onSelect = { picked ->
                            choices.firstOrNull { it.first == picked }?.let { (otherId, otherName) ->
                                scope.launch {
                                    // Keeps the one recording more by default, as the receipt review does.
                                    mergeWith = EditorMerge(otherId, otherName, repo.productToKeep(id, otherId) == id)
                                }
                            }
                        },
                        modifier = Modifier.fillMaxWidth(),
                    )
                }
            }
            pendingMerge?.let { m ->
                val thisName = name.ifBlank { stringResource(R.string.product_merge_this) }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        stringResource(R.string.product_merge_pending, m.otherName, if (m.keepThis) thisName else m.otherName),
                        style = MaterialTheme.typography.bodyMedium,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = { pendingMerge = null }) { Text(stringResource(R.string.merge_undo)) }
                }
            }
            DeleteButton(R.string.product_delete, R.string.product_delete_body) { scope.launch { repo.delete(id); onDone() } }
        }
    }

    mergeWith?.let { m ->
        val thisName = name.ifBlank { stringResource(R.string.product_merge_this) }
        MergeProductsDialog(
            keep = if (m.keepThis) thisName else m.otherName,
            other = if (m.keepThis) m.otherName else thisName,
            onSwap = { mergeWith = m.copy(keepThis = !m.keepThis) },
            note = stringResource(R.string.product_merge_on_save),
            onConfirm = {
                mergeWith = null
                mergeChoices = null
                pendingMerge = m
            },
            onDismiss = { mergeWith = null },
        )
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

    if (newBrand) {
        NameDialog(
            title = stringResource(R.string.brand_new),
            onDismiss = { newBrand = false },
            onConfirm = { text ->
                newBrand = false
                scope.launch {
                    brand = repo.saveBrand(null, BrandData(text))
                    brands = repo.brands()
                }
            },
        )
    }
}

private enum class ProductScan { LABEL, CODE }

/** "Merge with…" in the product editor: the other product, and whether this one is kept. */
private data class EditorMerge(val otherId: String, val otherName: String, val keepThis: Boolean)

/**
 * "Merge these products?": [keep] is kept and takes in what [other] has
 * (the core's merge_products); [other]'s sessions and purchases move to it
 * and [other] is deleted. [onSwap], when given, keeps the other one
 * instead; [note] is shown under the explanation.
 */
@Composable
internal fun MergeProductsDialog(
    keep: String,
    other: String,
    onSwap: (() -> Unit)?,
    note: String?,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.product_merge_title)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(stringResource(R.string.product_merge_body, keep, other))
                note?.let { Text(it, style = MaterialTheme.typography.bodySmall) }
                if (onSwap != null) OutlinedButton(onClick = onSwap) { Text(stringResource(R.string.product_merge_swap, other)) }
            }
        },
        confirmButton = { TextButton(onClick = onConfirm) { Text(stringResource(R.string.product_merge_confirm)) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
}

/** A label cannabinoid amount as typed in the product editor. */
private data class OtherAmount(val cannabinoid: String, val text: String, val unit: String)

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
    cannabinoids = new.cannabinoids.ifEmpty { existing.cannabinoids },
    terpenes = new.terpenes.ifEmpty { existing.terpenes },
    batch = new.batch ?: existing.batch,
    packageDateDay = new.packageDateDay ?: existing.packageDateDay,
    identifiers = existing.identifiers + new.identifiers.filter { n -> existing.identifiers.none { it.code == n.code } },
    shops = (existing.shops + new.shops).distinct(),
    subtype = existing.subtype ?: new.subtype.takeIf { existing.form == null || existing.form == new.form },
)

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

/**
 * Creates or edits a strain, in as much detail as the journal's strain
 * detail step allows (docs/design-decisions.md, "Richer strains"): the
 * type always; at Terpenes, which terpenes it is known for, nudged to 3 or
 * fewer; at Detailed, terpene levels, parent strains and cannabinoids too.
 * Fields a lower step (or a hidden data point) leaves out keep their saved
 * values when the strain is saved.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun StrainEditor(repo: JournalRepository, vocab: Vocabulary, id: String?, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    val tracking = LocalTracking.current
    var original by remember { mutableStateOf<StrainData?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var lineage by remember { mutableStateOf("") }
    var type by remember { mutableStateOf<String?>(null) }
    var terpenes by remember { mutableStateOf<List<TagRefData>>(emptyList()) }
    var levels by remember { mutableStateOf<Map<TagRefData, String>>(emptyMap()) }
    var parents by remember { mutableStateOf<List<String>>(emptyList()) }
    var cannabinoids by remember { mutableStateOf<List<String>>(emptyList()) }
    var allStrains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var customTerpenes by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    LaunchedEffect(id) {
        allStrains = repo.strains()
        customTerpenes = repo.tags("terpene")
        id?.let { repo.strain(it) }?.let {
            original = it
            name = it.name
            lineage = it.lineage.orEmpty()
            type = it.strainType
            terpenes = it.terpenes
            levels = it.terpeneLevels.associate { l -> l.terpene to l.level }
            parents = it.parents
            cannabinoids = it.cannabinoids
        }
        loaded = true
    }
    if (!loaded) return
    val step = tracking.strainDetail
    val showTerpenes = tracking.shows(DataPoint.TERPENES) && step != StrainDetail.SIMPLE
    val detailed = step == StrainDetail.DETAILED
    EditorScaffold(
        title = stringResource(if (id == null) R.string.strain_new else R.string.strain_edit),
        onClose = onDone,
        saveEnabled = name.isNotBlank(),
        onSave = {
            val kept = original
            // What this step doesn't show is saved as it was.
            val data = StrainData(
                name = name.trim(),
                lineage = if (step == StrainDetail.SIMPLE) kept?.lineage else lineage.trim().ifEmpty { null },
                terpenes = if (showTerpenes) terpenes else kept?.terpenes.orEmpty(),
                strainType = type,
                terpeneLevels = if (showTerpenes && detailed) {
                    terpenes.mapNotNull { t -> levels[t]?.let { TerpeneLevelData(t, it) } }
                } else {
                    kept?.terpeneLevels.orEmpty()
                },
                parents = if (detailed) parents else kept?.parents.orEmpty(),
                cannabinoids = if (detailed) cannabinoids else kept?.cannabinoids.orEmpty(),
            )
            scope.launch {
                repo.saveStrain(id, data)
                onDone()
            }
        },
    ) {
        OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.strain_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        SectionTitle(stringResource(R.string.strain_type))
        SingleChoiceChips(vocab.strainTypes, type) { type = it }
        Text(stringResource(R.string.strain_type_note), style = MaterialTheme.typography.bodySmall)

        if (showTerpenes) {
            SectionTitle(stringResource(R.string.strain_terpenes))
            Text(stringResource(R.string.strain_terpenes_hint), style = MaterialTheme.typography.bodySmall)
            val options = vocab.terpenes.map { TagRefData.Builtin(it) } + customTerpenes.map { TagRefData.Custom(it.id) }
            val customNames = customTerpenes.associate { it.id to it.data.name }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                // Terpenes saved earlier that aren't offered (a tag since deleted) still show.
                for (tag in (options + terpenes).distinct()) {
                    FilterChip(
                        selected = tag in terpenes,
                        onClick = { terpenes = if (tag in terpenes) terpenes - tag else terpenes + tag },
                        label = { Text(tagLabel(tag, customNames)) },
                    )
                }
            }
            if (terpenes.size > 3) {
                Text(stringResource(R.string.strain_terpenes_many), style = MaterialTheme.typography.bodySmall)
            }
            if (detailed && terpenes.isNotEmpty()) {
                SectionTitle(stringResource(R.string.strain_levels))
                Text(stringResource(R.string.strain_levels_hint), style = MaterialTheme.typography.bodySmall)
                for (tag in terpenes) {
                    Text(tagLabel(tag, customNames), style = MaterialTheme.typography.labelLarge)
                    SingleChoiceChips(vocab.terpeneLevels, levels[tag]) { level ->
                        levels = if (level == null) levels - tag else levels + (tag to level)
                    }
                }
            }
        }

        if (detailed) {
            SectionTitle(stringResource(R.string.strain_parents))
            SearchPicker(
                all = allStrains.filter { it.id != id }.map { it.id to it.data.name },
                selected = parents,
                searchLabel = stringResource(R.string.strain_parent_add),
                missingLabel = stringResource(R.string.strain_parent_missing),
                onAdd = { parents = parents + it },
                onRemove = { parents = parents - it },
                newLabel = { stringResource(R.string.strain_new_named, it) },
                // A parent that isn't in the journal yet becomes a strain with only a name.
                onNew = { typed ->
                    scope.launch {
                        val parentId = repo.saveStrain(null, StrainData(typed, null, emptyList(), null, emptyList(), emptyList(), emptyList()))
                        allStrains = repo.strains()
                        parents = parents + parentId
                    }
                },
            )

            SectionTitle(stringResource(R.string.strain_cannabinoids))
            Text(stringResource(R.string.strain_cannabinoids_hint), style = MaterialTheme.typography.bodySmall)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (c in (vocab.cannabinoids + cannabinoids).distinct()) {
                    FilterChip(
                        selected = c in cannabinoids,
                        onClick = { cannabinoids = if (c in cannabinoids) cannabinoids - c else cannabinoids + c },
                        label = { Text(label(c)) },
                    )
                }
            }
        }

        if (step != StrainDetail.SIMPLE) {
            OutlinedTextField(lineage, { lineage = it }, label = { Text(stringResource(R.string.strain_lineage)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        }
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

/** The tag category of the user's own subtypes. */
internal const val SUBTYPE_CATEGORY = "subtype"

/**
 * A product's subtype under its type (docs/design-decisions.md, "Product
 * subtypes"): the built-in ones, then the user's own, and a chip to add one.
 * Optional; tapping the selected one clears it.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SubtypeChips(
    builtins: List<String>,
    custom: List<Entry<TagData>>,
    selected: TagRefData?,
    onSelect: (TagRefData?) -> Unit,
    onAdd: () -> Unit,
) {
    SectionTitle(stringResource(R.string.product_subtype))
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        val options = builtins.map { TagRefData.Builtin(it) to label(it) } +
            custom.map { TagRefData.Custom(it.id) to it.data.name }
        for ((ref, text) in options) {
            FilterChip(
                selected = ref == selected,
                onClick = { onSelect(if (ref == selected) null else ref) },
                label = { Text(text) },
            )
        }
        AssistChip(
            onClick = onAdd,
            label = { Text(stringResource(R.string.product_subtype_add)) },
            leadingIcon = { Icon(painterResource(R.drawable.ic_add), contentDescription = null) },
        )
    }
}

/**
 * Adds the user's own subtype under [form]: a name, and optionally its usual
 * method when it isn't the type's (a patch is taken through the skin, not
 * like a lotion). The method then sets its dose units, timeline and check-in.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SubtypeDialog(
    form: String,
    methods: List<String>,
    onDismiss: () -> Unit,
    onConfirm: (String, String?) -> Unit,
) {
    var text by remember { mutableStateOf("") }
    var method by remember { mutableStateOf<String?>(null) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.product_subtype_new, label(form))) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(text, { text = it }, singleLine = true, label = { Text(stringResource(R.string.name)) })
                Text(stringResource(R.string.product_subtype_method), style = MaterialTheme.typography.bodyMedium)
                Text(stringResource(R.string.product_subtype_method_hint), style = MaterialTheme.typography.bodySmall)
                SingleChoiceChips(methods, method) { method = it }
            }
        },
        confirmButton = {
            TextButton(onClick = { onConfirm(text.trim(), method) }, enabled = text.isNotBlank()) { Text(stringResource(R.string.add)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
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
