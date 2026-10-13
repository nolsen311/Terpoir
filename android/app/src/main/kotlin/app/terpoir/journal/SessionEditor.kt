// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import app.terpoir.R
import app.terpoir.alerts.CheckIns
import app.terpoir.core.ProductData
import app.terpoir.core.SnippetData
import app.terpoir.core.SpotData
import app.terpoir.core.TagData
import app.terpoir.core.TagRefData
import app.terpoir.core.Vocabulary
import app.terpoir.core.defaultMethodForForm
import app.terpoir.core.doseUnitsForMethod
import app.terpoir.privacy.PrivacySettings
import app.terpoir.share.SessionShare
import java.time.Instant
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.ZoneId
import java.time.ZoneOffset
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

/** Creates (`id` null) or edits a session. */
@Composable
fun SessionEditor(
    repo: JournalRepository,
    vocab: Vocabulary,
    settings: PrivacySettings,
    checkIns: CheckIns,
    id: String?,
    onDone: () -> Unit,
    spotForNew: String? = null,
) {
    val scope = rememberCoroutineScope()
    var draft by remember { mutableStateOf<SessionDraft?>(null) }
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var productLabels by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    var customEffects by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var customSymptoms by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var error by remember { mutableStateOf<DraftError?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }
    var notes by remember { mutableStateOf<List<Entry<SnippetData>>>(emptyList()) }
    var spots by remember { mutableStateOf<List<Entry<SpotData>>>(emptyList()) }
    // A note open over the editor (null id: a new one), so the session's
    // unsaved edits survive it.
    var openNote by remember { mutableStateOf<NoteTarget?>(null) }
    // Follow-up check-in: null follows the default for the session's methods.
    var checkInChoice by remember { mutableStateOf<Boolean?>(null) }
    var checkInMinutes by remember { mutableStateOf<String?>(null) }
    var sharing by remember { mutableStateOf(false) }

    LaunchedEffect(id) {
        products = repo.products()
        productLabels = repo.productLabels()
        spots = repo.spots()
        customEffects = repo.tags(EFFECT_CATEGORY)
        customSymptoms = repo.tags(SYMPTOM_CATEGORY)
        draft = id?.let { repo.session(it) }?.let(SessionDraft::of) ?: SessionDraft.new(
            System.currentTimeMillis(),
            currentOffsetMin(System.currentTimeMillis()),
            spot = spotForNew,
        )
    }
    LaunchedEffect(id, openNote) { if (id != null && openNote == null) notes = repo.sessionSnippets(id) }
    val current = draft ?: return
    val tracking = LocalTracking.current
    if (sharing && id != null) {
        SessionShare(repo, id, settings.targetedSymptoms) { sharing = false }
        return
    }
    openNote?.let { target ->
        NoteScreen(repo, settings, vocab, target.id, onDone = { openNote = null }, session = id)
        return
    }

    val checkInDefault = CheckIns.defaultMinutes(current.lines.map { it.method })
    val checkIn = settings.followUpCheckIns && (checkInChoice ?: (checkInDefault != null))
    val checkInAfter = checkInMinutes ?: (checkInDefault ?: DEFAULT_CHECK_IN_MIN).toString()

    EditorScaffold(
        title = stringResource(if (id == null) R.string.session_new else R.string.session_edit),
        onClose = onDone,
        onSave = {
            when (val result = current.toData()) {
                is SessionDraft.Result.Invalid -> error = result.error
                is SessionDraft.Result.Valid -> scope.launch {
                    val saved = repo.saveSession(id, result.data)
                    val minutes = checkInAfter.trim().toIntOrNull()?.takeIf { it > 0 }
                    if (checkIn && minutes != null) checkIns.schedule(saved, current.whenMs + minutes * 60_000L)
                    onDone()
                }
            }
        },
    ) {
        WhenRow(current.whenMs, current.utcOffsetMin) { ms, offset -> draft = current.copy(whenMs = ms, utcOffsetMin = offset) }

        SectionTitle(stringResource(R.string.session_products))
        current.lines.forEachIndexed { index, line ->
            LineEditor(
                line = line,
                products = products,
                productLabels = productLabels,
                spotNames = spots.associate { it.id to it.data.name },
                vocab = vocab,
                onChange = { changed -> draft = current.copy(lines = current.lines.toMutableList().also { it[index] = changed }) },
                onRemove = { draft = current.copy(lines = current.lines.toMutableList().also { it.removeAt(index) }) },
            )
        }
        OutlinedButton(onClick = {
            val method = vocab.methods.first()
            draft = current.copy(lines = current.lines + LineDraft(method = method, doseUnit = doseUnitsForMethod(method).firstOrNull()))
        }) { Text(stringResource(R.string.session_add_product)) }

        // "What I track": hidden data points are left out here, and the
        // draft carries their saved values unchanged.
        val showIntention = tracking.shows(DataPoint.INTENTION)
        val showSocial = tracking.shows(DataPoint.SOCIAL)
        val showMood = tracking.shows(DataPoint.MOOD)
        if (showIntention || showSocial || showMood) SectionTitle(stringResource(R.string.session_intention))
        if (showIntention) SingleChoiceChips(vocab.intentions, current.intention) { draft = current.copy(intention = it) }
        if (showSocial) SingleChoiceChips(vocab.social, current.social) { draft = current.copy(social = it) }
        if (showMood) {
            Text(stringResource(R.string.session_mood_before), style = MaterialTheme.typography.labelLarge)
            ScaleRow(current.moodBefore, { draft = current.copy(moodBefore = it) }, stringResource(R.string.scale_low), stringResource(R.string.scale_high))
        }

        SectionTitle(stringResource(R.string.session_effects))
        EffectsEditor(
            vocab = vocab,
            custom = customEffects,
            effects = current.effects,
            onChange = { draft = current.copy(effects = it) },
            onNewEffect = { name ->
                scope.launch {
                    val tagId = repo.saveTag(null, TagData(name, EFFECT_CATEGORY))
                    customEffects = repo.tags(EFFECT_CATEGORY)
                    draft = draft?.let { d -> d.copy(effects = d.effects + (TagRefData.Custom(tagId) to DEFAULT_INTENSITY)) }
                }
            },
        )

        // Hidden unless turned on in the Privacy Center; while hidden, the
        // draft carries any saved symptoms unchanged.
        if (settings.targetedSymptoms) {
            SectionTitle(stringResource(R.string.session_symptoms))
            SymptomsEditor(
                vocab = vocab,
                custom = customSymptoms,
                symptoms = current.symptoms,
                onChange = { draft = current.copy(symptoms = it) },
                onNewSymptom = { name ->
                    scope.launch {
                        val tagId = repo.saveTag(null, TagData(name, SYMPTOM_CATEGORY))
                        customSymptoms = repo.tags(SYMPTOM_CATEGORY)
                        draft = draft?.let { d -> d.copy(symptoms = d.symptoms + (TagRefData.Custom(tagId) to NOT_RATED)) }
                    }
                },
            )
        }

        if (tracking.shows(DataPoint.TIMELINE)) {
            SectionTitle(stringResource(R.string.session_timeline))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                MinutesField(stringResource(R.string.session_onset), current.onset, Modifier.weight(1f)) { draft = current.copy(onset = it) }
                MinutesField(stringResource(R.string.session_peak), current.peak, Modifier.weight(1f)) { draft = current.copy(peak = it) }
                MinutesField(stringResource(R.string.session_duration), current.duration, Modifier.weight(1f)) { draft = current.copy(duration = it) }
            }
        }

        SectionTitle(stringResource(R.string.session_outcome))
        if (showMood) {
            Text(stringResource(R.string.session_mood_after), style = MaterialTheme.typography.labelLarge)
            ScaleRow(current.moodAfter, { draft = current.copy(moodAfter = it) }, stringResource(R.string.scale_low), stringResource(R.string.scale_high))
        }
        Text(stringResource(R.string.session_rating), style = MaterialTheme.typography.labelLarge)
        ScaleRow(current.rating, { draft = current.copy(rating = it) }, stringResource(R.string.rating_low), stringResource(R.string.rating_high))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(stringResource(R.string.session_would_repeat), style = MaterialTheme.typography.labelLarge)
            FilterChip(
                selected = current.wouldRepeat == true,
                onClick = { draft = current.copy(wouldRepeat = if (current.wouldRepeat == true) null else true) },
                label = { Text(stringResource(R.string.yes)) },
            )
            FilterChip(
                selected = current.wouldRepeat == false,
                onClick = { draft = current.copy(wouldRepeat = if (current.wouldRepeat == false) null else false) },
                label = { Text(stringResource(R.string.no)) },
            )
        }

        SectionTitle(stringResource(R.string.session_spot))
        Dropdown(
            label = stringResource(R.string.session_spot),
            options = listOf<Pair<String?, String>>(null to stringResource(R.string.session_no_spot)) + spots.map { it.id to it.data.name },
            selected = current.spot,
            onSelect = { draft = current.copy(spot = it) },
        )

        if (settings.followUpCheckIns) {
            SectionTitle(stringResource(R.string.check_in_section))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                FilterChip(
                    selected = checkIn,
                    onClick = { checkInChoice = !checkIn },
                    label = { Text(stringResource(R.string.check_in_later)) },
                )
                if (checkIn) {
                    MinutesField(stringResource(R.string.check_in_after), checkInAfter, Modifier.weight(1f)) { checkInMinutes = it }
                }
            }
            if (checkIn) NotificationPermission()
        }

        SectionTitle(stringResource(R.string.session_notes))
        if (id == null) {
            Text(stringResource(R.string.session_notes_after_save), style = MaterialTheme.typography.bodySmall)
        } else {
            for (note in notes) {
                TextButton(onClick = { openNote = NoteTarget(note.id) }) {
                    Text(note.data.text, maxLines = 2, overflow = TextOverflow.Ellipsis)
                }
            }
            OutlinedButton(onClick = { openNote = NoteTarget(null) }) { Text(stringResource(R.string.note_new)) }
        }

        error?.let {
            Text(
                stringResource(if (it == DraftError.DOSE) R.string.error_dose else R.string.error_timeline),
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (id != null) {
            // Shares what is saved, not unsaved edits.
            OutlinedButton(onClick = { sharing = true }) { Text(stringResource(R.string.share)) }
            TextButton(onClick = { confirmDelete = true }) {
                Text(stringResource(R.string.session_delete), color = MaterialTheme.colorScheme.error)
            }
        }
    }

    if (confirmDelete && id != null) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.session_delete_title)) },
            text = { Text(stringResource(R.string.session_delete_body)) },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    scope.launch {
                        repo.delete(id)
                        onDone()
                    }
                }) { Text(stringResource(R.string.delete), color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton(onClick = { confirmDelete = false }) { Text(stringResource(R.string.cancel)) } },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun LineEditor(
    line: LineDraft,
    products: List<Entry<ProductData>>,
    productLabels: Map<String, String>,
    spotNames: Map<String, String>,
    vocab: Vocabulary,
    onChange: (LineDraft) -> Unit,
    onRemove: () -> Unit,
) {
    OutlinedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            val none = stringResource(R.string.session_no_product)
            Dropdown(
                label = stringResource(R.string.session_product),
                options = listOf<Pair<String?, String>>(null to none) + products.map { it.id to (productLabels[it.id] ?: it.data.name) }.sortedBy { it.second.lowercase() },
                selected = line.product,
                onSelect = { productId ->
                    val form = products.firstOrNull { it.id == productId }?.data?.form
                    val method = form?.let(::defaultMethodForForm) ?: line.method
                    onChange(
                        line.copy(
                            product = productId,
                            method = method,
                            doseUnit = unitFor(method, line.doseUnit),
                            // A purchase line belongs to one product.
                            purchaseId = line.purchaseId.takeIf { productId == line.product },
                            purchaseLineId = line.purchaseLineId.takeIf { productId == line.product },
                            shop = line.shop.takeIf { productId == line.product },
                        ),
                    )
                },
            )
            // "From:" only when the product is bought at more than one shop;
            // with one, insights already know where it came from.
            val tracking = LocalTracking.current
            val shops = products.firstOrNull { it.id == line.product }?.data?.shops.orEmpty().filter { it in spotNames }
            if (shops.size > 1 && tracking.shows(DataPoint.PRICE_AND_SHOP)) {
                Text(stringResource(R.string.session_line_from), style = MaterialTheme.typography.labelLarge)
                FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    for (shop in shops) {
                        val selected = line.shop == shop
                        FilterChip(
                            selected = selected,
                            onClick = { onChange(line.copy(shop = if (selected) null else shop)) },
                            label = { Text(spotNames.getValue(shop)) },
                        )
                    }
                }
            }
            Dropdown(
                label = stringResource(R.string.session_method),
                options = vocab.methods.map { it to label(it) },
                selected = line.method,
                onSelect = { method -> onChange(line.copy(method = method, doseUnit = unitFor(method, line.doseUnit))) },
            )
            val units = doseUnitsForMethod(line.method)
            if (units.isNotEmpty() && tracking.shows(DataPoint.DOSE)) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = line.dose,
                        onValueChange = { onChange(line.copy(dose = it)) },
                        label = { Text(stringResource(R.string.session_dose)) },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )
                    Dropdown(
                        label = stringResource(R.string.session_unit),
                        options = units.map { it to label(it) },
                        selected = line.doseUnit ?: units.first(),
                        onSelect = { onChange(line.copy(doseUnit = it)) },
                        modifier = Modifier.weight(1f),
                    )
                }
            }
            TextButton(onClick = onRemove) { Text(stringResource(R.string.remove)) }
        }
    }
}

/** Keeps the current unit when the method allows it, else the method's usual one. */
private fun unitFor(method: String, current: String?): String? {
    val units = doseUnitsForMethod(method)
    return if (current in units) current else units.firstOrNull()
}

@OptIn(ExperimentalLayoutApi::class)
/** Asks for the notification permission (Android 13+) when it is missing. */
@Composable
private fun NotificationPermission() {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
    val context = LocalContext.current
    fun granted() = ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED
    var allowed by remember { mutableStateOf(granted()) }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { allowed = granted() }
    if (allowed) return
    Text(stringResource(R.string.check_in_needs_notifications), style = MaterialTheme.typography.bodySmall)
    OutlinedButton(onClick = { ask.launch(Manifest.permission.POST_NOTIFICATIONS) }) {
        Text(stringResource(R.string.notify_setup_allow))
    }
}

/** The tag category of the user's own targeted symptoms. */
private const val SYMPTOM_CATEGORY = "symptom"

/** A targeted symptom the user hasn't rated (unsure, or not yet). */
private const val NOT_RATED = "not_rated"

/** Results in display order, with their labels. */
private val SYMPTOM_RESULTS = listOf(
    "helped" to R.string.symptom_helped,
    "did_not_help" to R.string.symptom_did_not_help,
    NOT_RATED to R.string.symptom_not_rated,
)

/**
 * Targeted symptoms: symptoms, never conditions. Picking one targets it;
 * each is then rated helped or didn't help, or left "not sure".
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SymptomsEditor(
    vocab: Vocabulary,
    custom: List<Entry<TagData>>,
    symptoms: Map<TagRefData, String>,
    onChange: (Map<TagRefData, String>) -> Unit,
    onNewSymptom: (String) -> Unit,
) {
    val names = custom.associate { it.id to it.data.name }
    Text(stringResource(R.string.session_symptoms_hint), style = MaterialTheme.typography.bodySmall)
    Text(stringResource(R.string.health_reminder), style = MaterialTheme.typography.bodySmall)
    val all = vocab.symptoms.map { TagRefData.Builtin(it) } + custom.map { TagRefData.Custom(it.id) }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (tag in all) {
            val selected = tag in symptoms
            FilterChip(
                selected = selected,
                onClick = { onChange(if (selected) symptoms - tag else symptoms + (tag to NOT_RATED)) },
                label = { Text(tagLabel(tag, names)) },
            )
        }
    }
    for ((tag, result) in symptoms) {
        Text(tagLabel(tag, names), style = MaterialTheme.typography.labelLarge)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((slug, label) in SYMPTOM_RESULTS) {
                FilterChip(
                    selected = result == slug,
                    onClick = { onChange(symptoms + (tag to slug)) },
                    label = { Text(stringResource(label)) },
                )
            }
        }
    }
    var newSymptom by remember { mutableStateOf("") }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedTextField(
            value = newSymptom,
            onValueChange = { newSymptom = it },
            label = { Text(stringResource(R.string.symptom_new)) },
            singleLine = true,
            modifier = Modifier.weight(1f),
        )
        TextButton(
            onClick = {
                onNewSymptom(newSymptom.trim())
                newSymptom = ""
            },
            enabled = newSymptom.isNotBlank(),
        ) { Text(stringResource(R.string.add)) }
    }
}

@Composable
internal fun EffectsEditor(
    vocab: Vocabulary,
    custom: List<Entry<TagData>>,
    effects: Map<TagRefData, Int>,
    onChange: (Map<TagRefData, Int>) -> Unit,
    onNewEffect: (String) -> Unit,
) {
    val names = custom.associate { it.id to it.data.name }
    val all = vocab.coreEffects.map { TagRefData.Builtin(it) } + custom.map { TagRefData.Custom(it.id) }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (tag in all) {
            val selected = tag in effects
            FilterChip(
                selected = selected,
                onClick = { onChange(if (selected) effects - tag else effects + (tag to DEFAULT_INTENSITY)) },
                label = { Text(tagLabel(tag, names)) },
            )
        }
    }
    for ((tag, intensity) in effects) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(tagLabel(tag, names), modifier = Modifier.width(112.dp), style = MaterialTheme.typography.bodyMedium)
            Slider(
                value = intensity.toFloat(),
                onValueChange = { onChange(effects + (tag to it.roundToInt())) },
                valueRange = 0f..MAX_INTENSITY.toFloat(),
                steps = MAX_INTENSITY - 1,
                modifier = Modifier.weight(1f),
            )
            Text(intensity.toString(), style = MaterialTheme.typography.labelLarge)
        }
    }
    var newEffect by remember { mutableStateOf("") }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedTextField(
            value = newEffect,
            onValueChange = { newEffect = it },
            label = { Text(stringResource(R.string.session_new_effect)) },
            singleLine = true,
            modifier = Modifier.weight(1f),
        )
        TextButton(
            onClick = {
                onNewEffect(newEffect.trim())
                newEffect = ""
            },
            enabled = newEffect.isNotBlank(),
        ) { Text(stringResource(R.string.add)) }
    }
}

@Composable
private fun MinutesField(label: String, value: String, modifier: Modifier, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        suffix = { Text(stringResource(R.string.minutes_short)) },
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
        singleLine = true,
        modifier = modifier,
    )
}

/** Date and time buttons with their pickers. Times are local to the device's zone at that moment. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun WhenRow(whenMs: Long, offsetMin: Short, onChange: (Long, Short) -> Unit) {
    var pickDate by remember { mutableStateOf(false) }
    var pickTime by remember { mutableStateOf(false) }
    val local = Instant.ofEpochMilli(whenMs).atOffset(ZoneOffset.ofTotalSeconds(offsetMin * 60)).toLocalDateTime()
    Text(formatWhen(whenMs, offsetMin), style = MaterialTheme.typography.titleLarge)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = { pickDate = true }) { Text(stringResource(R.string.session_change_date)) }
        OutlinedButton(onClick = { pickTime = true }) { Text(stringResource(R.string.session_change_time)) }
    }
    if (pickDate) {
        val state = rememberDatePickerState(initialSelectedDateMillis = local.toLocalDate().atStartOfDay().toInstant(ZoneOffset.UTC).toEpochMilli())
        DatePickerDialog(
            onDismissRequest = { pickDate = false },
            confirmButton = {
                TextButton(onClick = {
                    pickDate = false
                    state.selectedDateMillis?.let {
                        val date = Instant.ofEpochMilli(it).atOffset(ZoneOffset.UTC).toLocalDate()
                        emit(LocalDateTime.of(date, local.toLocalTime()), onChange)
                    }
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { pickDate = false }) { Text(stringResource(R.string.cancel)) } },
        ) { DatePicker(state) }
    }
    if (pickTime) {
        val state = rememberTimePickerState(initialHour = local.hour, initialMinute = local.minute)
        AlertDialog(
            onDismissRequest = { pickTime = false },
            confirmButton = {
                TextButton(onClick = {
                    pickTime = false
                    emit(LocalDateTime.of(local.toLocalDate(), LocalTime.of(state.hour, state.minute)), onChange)
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { pickTime = false }) { Text(stringResource(R.string.cancel)) } },
            text = { TimePicker(state) },
        )
    }
}

private fun emit(local: LocalDateTime, onChange: (Long, Short) -> Unit) {
    val zoned = local.atZone(ZoneId.systemDefault())
    onChange(zoned.toInstant().toEpochMilli(), (zoned.offset.totalSeconds / 60).toShort())
}

private fun currentOffsetMin(nowMs: Long): Short =
    (ZoneId.systemDefault().rules.getOffset(Instant.ofEpochMilli(nowMs)).totalSeconds / 60).toShort()

/** Category of custom effect tags. */
const val EFFECT_CATEGORY = "effect"

internal const val DEFAULT_INTENSITY = 3
private const val MAX_INTENSITY = 5

/** A note to open over the session editor; `id` null for a new one. */
private data class NoteTarget(val id: String?)

/** The check-in delay offered when the session's methods suggest none. */
private const val DEFAULT_CHECK_IN_MIN = 60
