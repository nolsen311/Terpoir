// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.Manifest
import android.content.pm.PackageManager
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
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
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
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
    // The saved session: `id`, or the new session once saved to add a note.
    var sessionId by remember(id) { mutableStateOf(id) }

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
    LaunchedEffect(sessionId, openNote) { sessionId?.let { if (openNote == null) notes = repo.sessionSnippets(it) } }
    val current = draft ?: return
    val tracking = LocalTracking.current
    val shared = sessionId
    if (sharing && shared != null) {
        SessionShare(repo, shared, settings.targetedSymptoms) { sharing = false }
        return
    }
    openNote?.let { target ->
        NoteScreen(repo, settings, vocab, target.id, onDone = { openNote = null }, session = sessionId)
        return
    }

    val checkInDefault = CheckIns.defaultMinutes(current.lines.map { it.method })
    val checkIn = settings.followUpCheckIns && (checkInChoice ?: (checkInDefault != null))
    val checkInAfter = checkInMinutes ?: (checkInDefault ?: DEFAULT_CHECK_IN_MIN).toString()

    EditorScaffold(
        title = stringResource(if (sessionId == null) R.string.session_new else R.string.session_edit),
        onClose = onDone,
        onSave = {
            when (val result = current.toData()) {
                is SessionDraft.Result.Invalid -> error = result.error
                is SessionDraft.Result.Valid -> scope.launch {
                    val saved = repo.saveSession(sessionId, result.data)
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

        // Spot, check-in and notes: one row of icons, each opening its own
        // menu or dialog.
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceEvenly) {
            SpotAction(spots, current.spot) { draft = current.copy(spot = it) }
            if (settings.followUpCheckIns) {
                CheckInAction(
                    on = checkIn,
                    minutes = checkInAfter,
                    onToggle = { checkInChoice = it },
                    onMinutes = { checkInMinutes = it },
                )
            }
            NotesAction(
                notes = notes,
                onOpen = { openNote = NoteTarget(it) },
                onNew = {
                    // A note belongs to a saved session, so a new session is
                    // saved first; the editor stays open.
                    if (sessionId != null) {
                        openNote = NoteTarget(null)
                    } else {
                        when (val result = current.toData()) {
                            is SessionDraft.Result.Invalid -> error = result.error
                            is SessionDraft.Result.Valid -> scope.launch {
                                sessionId = repo.saveSession(null, result.data)
                                openNote = NoteTarget(null)
                            }
                        }
                    }
                },
            )
        }

        error?.let {
            Text(
                stringResource(if (it == DraftError.DOSE) R.string.error_dose else R.string.error_timeline),
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (sessionId != null) {
            // Shares what is saved, not unsaved edits.
            OutlinedButton(onClick = { sharing = true }) { Text(stringResource(R.string.share)) }
            TextButton(onClick = { confirmDelete = true }) {
                Text(stringResource(R.string.session_delete), color = MaterialTheme.colorScheme.error)
            }
        }
    }

    val deleting = sessionId
    if (confirmDelete && deleting != null) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text(stringResource(R.string.session_delete_title)) },
            text = { Text(stringResource(R.string.session_delete_body)) },
            confirmButton = {
                TextButton(onClick = {
                    confirmDelete = false
                    scope.launch {
                        repo.delete(deleting)
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
    var adding by remember { mutableStateOf(false) }
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (tag in all) {
            val selected = tag in effects
            FilterChip(
                selected = selected,
                onClick = { onChange(if (selected) effects - tag else effects + (tag to DEFAULT_INTENSITY)) },
                label = { Text(tagLabel(tag, names)) },
            )
        }
        IconButton(onClick = { adding = true }, modifier = Modifier.align(Alignment.CenterVertically)) {
            Icon(painterResource(R.drawable.ic_add), contentDescription = stringResource(R.string.session_add_effect))
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
    if (adding) NewEffectDialog(onDismiss = { adding = false }) { name ->
        adding = false
        onNewEffect(name)
    }
}

/** Asks for the name of the user's own effect. */
@Composable
private fun NewEffectDialog(onDismiss: () -> Unit, onAdd: (String) -> Unit) {
    var name by remember { mutableStateOf("") }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.session_add_effect)) },
        text = {
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                label = { Text(stringResource(R.string.session_new_effect)) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Sentences, imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { if (name.isNotBlank()) onAdd(name.trim()) }),
                modifier = Modifier.fillMaxWidth().focusRequester(focus),
            )
        },
        confirmButton = {
            TextButton(onClick = { onAdd(name.trim()) }, enabled = name.isNotBlank()) { Text(stringResource(R.string.add)) }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text(stringResource(R.string.cancel)) } },
    )
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

/**
 * The session's date and time, with a calendar button that opens the date
 * picker and then the time picker. Times are local to the device's zone at
 * that moment.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun WhenRow(whenMs: Long, offsetMin: Short, onChange: (Long, Short) -> Unit) {
    var pickDate by remember { mutableStateOf(false) }
    var pickTime by remember { mutableStateOf(false) }
    val local = Instant.ofEpochMilli(whenMs).atOffset(ZoneOffset.ofTotalSeconds(offsetMin * 60)).toLocalDateTime()
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(formatWhen(whenMs, offsetMin), style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
        IconButton(onClick = { pickDate = true }) {
            Icon(painterResource(R.drawable.ic_calendar), contentDescription = stringResource(R.string.session_change_when))
        }
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
                    // The time comes next; cancelling it keeps the new date.
                    pickTime = true
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

/**
 * One of the icons under a session, with a label below. When something is
 * set, a small green badge sits on the icon: a check, or the count when
 * there is more than one. [description] is what a screen reader says.
 */
@Composable
private fun SessionAction(icon: Int, label: String, description: String, count: Int, onClick: () -> Unit) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.width(96.dp)) {
        Box {
            OutlinedIconButton(onClick = onClick) { Icon(painterResource(icon), contentDescription = description) }
            if (count > 0) SetBadge(count, Modifier.align(Alignment.TopEnd).offset(x = 4.dp, y = (-2).dp))
        }
        Text(
            label,
            style = MaterialTheme.typography.labelMedium,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.clearAndSetSemantics { },
        )
    }
}

/** A green circle with a check (one) or the number (more than one), as on the sync card. */
@Composable
private fun SetBadge(count: Int, modifier: Modifier) {
    val circle = MaterialTheme.colorScheme.primary
    val mark = MaterialTheme.colorScheme.onPrimary
    Box(modifier.size(18.dp).clearAndSetSemantics { }, contentAlignment = Alignment.Center) {
        Canvas(Modifier.matchParentSize()) {
            drawCircle(circle)
            if (count == 1) {
                val w = size.width
                val check = Path().apply {
                    moveTo(w * 0.28f, w * 0.52f)
                    lineTo(w * 0.44f, w * 0.68f)
                    lineTo(w * 0.73f, w * 0.36f)
                }
                drawPath(check, mark, style = Stroke(width = w * 0.12f, cap = StrokeCap.Round, join = StrokeJoin.Round))
            }
        }
        if (count > 1) {
            Text(
                if (count > 9) "9+" else count.toString(),
                color = mark,
                // Fixed in dp, like the circle it sits in.
                fontSize = with(LocalDensity.current) { 11.dp.toSp() },
                style = MaterialTheme.typography.labelSmall,
            )
        }
    }
}

/** The session's spot: a pin that opens the list of spots. */
@Composable
private fun SpotAction(spots: List<Entry<SpotData>>, spot: String?, onSelect: (String?) -> Unit) {
    var open by remember { mutableStateOf(false) }
    val name = spots.firstOrNull { it.id == spot }?.data?.name
    Box {
        val label = name ?: stringResource(R.string.session_spot)
        SessionAction(R.drawable.ic_spot_add, label, label, count = if (name != null) 1 else 0) { open = true }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            DropdownMenuItem(text = { Text(stringResource(R.string.session_no_spot)) }, onClick = {
                open = false
                onSelect(null)
            })
            for (entry in spots) {
                DropdownMenuItem(text = { Text(entry.data.name) }, onClick = {
                    open = false
                    onSelect(entry.id)
                })
            }
        }
    }
}

/** The follow-up check-in: an alarm clock that opens its switch and delay. */
@Composable
private fun CheckInAction(on: Boolean, minutes: String, onToggle: (Boolean) -> Unit, onMinutes: (String) -> Unit) {
    var open by remember { mutableStateOf(false) }
    val delay = minutes.trim().toIntOrNull()?.takeIf { it > 0 }
    val label = if (on && delay != null) stringResource(R.string.check_in_in, delay) else stringResource(R.string.check_in_section)
    Box {
        SessionAction(R.drawable.ic_check_in, label, label, count = if (on) 1 else 0) { open = true }
    }
    if (open) {
        AlertDialog(
            onDismissRequest = { open = false },
            title = { Text(stringResource(R.string.check_in_section)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(stringResource(R.string.check_in_later), modifier = Modifier.weight(1f))
                        Switch(checked = on, onCheckedChange = onToggle)
                    }
                    if (on) {
                        MinutesField(stringResource(R.string.check_in_after), minutes, Modifier.fillMaxWidth(), onMinutes)
                        NotificationPermission()
                    }
                }
            },
            confirmButton = { TextButton(onClick = { open = false }) { Text(stringResource(R.string.ok)) } },
        )
    }
}

/**
 * The session's notes: a notebook that starts a new note (by voice or text,
 * as the user prefers), or, once there are notes, lists them with a button
 * for a new one. A new session is saved first ([onNew] does that).
 */
@Composable
private fun NotesAction(notes: List<Entry<SnippetData>>, onOpen: (String) -> Unit, onNew: () -> Unit) {
    var list by remember { mutableStateOf(false) }
    val title = stringResource(R.string.session_notes)
    val description = if (notes.isEmpty()) stringResource(R.string.note_new) else stringResource(R.string.session_notes_count, notes.size)
    SessionAction(R.drawable.ic_note_add, title, description, count = notes.size) {
        if (notes.isEmpty()) onNew() else list = true
    }
    if (list) {
        AlertDialog(
            onDismissRequest = { list = false },
            title = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(title, modifier = Modifier.weight(1f))
                    IconButton(onClick = {
                        list = false
                        onNew()
                    }) { Icon(painterResource(R.drawable.ic_note_add), contentDescription = stringResource(R.string.note_new)) }
                }
            },
            text = {
                Column(Modifier.verticalScroll(rememberScrollState())) {
                    for (note in notes) {
                        Text(
                            note.data.text,
                            maxLines = 2,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier
                                .fillMaxWidth()
                                .clickable {
                                    list = false
                                    onOpen(note.id)
                                }
                                .padding(vertical = 12.dp),
                        )
                    }
                }
            },
            confirmButton = { TextButton(onClick = { list = false }) { Text(stringResource(R.string.close)) } },
        )
    }
}

/** A note to open over the session editor; `id` null for a new one. */
private data class NoteTarget(val id: String?)

/** The check-in delay offered when the session's methods suggest none. */
private const val DEFAULT_CHECK_IN_MIN = 60
