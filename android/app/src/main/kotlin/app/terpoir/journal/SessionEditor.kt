// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.ProductData
import app.terpoir.core.TagData
import app.terpoir.core.TagRefData
import app.terpoir.core.Vocabulary
import app.terpoir.core.defaultMethodForForm
import app.terpoir.core.doseUnitsForMethod
import java.time.Instant
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.ZoneId
import java.time.ZoneOffset
import kotlin.math.roundToInt
import kotlinx.coroutines.launch

/** Creates (`id` null) or edits a session. */
@Composable
fun SessionEditor(repo: JournalRepository, vocab: Vocabulary, id: String?, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var draft by remember { mutableStateOf<SessionDraft?>(null) }
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var customEffects by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var error by remember { mutableStateOf<DraftError?>(null) }
    var confirmDelete by remember { mutableStateOf(false) }

    LaunchedEffect(id) {
        products = repo.products()
        customEffects = repo.tags(EFFECT_CATEGORY)
        draft = id?.let { repo.session(it) }?.let(SessionDraft::of) ?: SessionDraft.new(
            System.currentTimeMillis(),
            currentOffsetMin(System.currentTimeMillis()),
        )
    }
    val current = draft ?: return

    EditorScaffold(
        title = stringResource(if (id == null) R.string.session_new else R.string.session_edit),
        onClose = onDone,
        onSave = {
            when (val result = current.toData()) {
                is SessionDraft.Result.Invalid -> error = result.error
                is SessionDraft.Result.Valid -> scope.launch {
                    repo.saveSession(id, result.data)
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
                vocab = vocab,
                onChange = { changed -> draft = current.copy(lines = current.lines.toMutableList().also { it[index] = changed }) },
                onRemove = { draft = current.copy(lines = current.lines.toMutableList().also { it.removeAt(index) }) },
            )
        }
        OutlinedButton(onClick = {
            val method = vocab.methods.first()
            draft = current.copy(lines = current.lines + LineDraft(method = method, doseUnit = doseUnitsForMethod(method).firstOrNull()))
        }) { Text(stringResource(R.string.session_add_product)) }

        SectionTitle(stringResource(R.string.session_intention))
        SingleChoiceChips(vocab.intentions, current.intention) { draft = current.copy(intention = it) }
        SingleChoiceChips(vocab.social, current.social) { draft = current.copy(social = it) }
        Text(stringResource(R.string.session_mood_before), style = MaterialTheme.typography.labelLarge)
        ScaleRow(current.moodBefore, { draft = current.copy(moodBefore = it) }, stringResource(R.string.scale_low), stringResource(R.string.scale_high))

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

        SectionTitle(stringResource(R.string.session_timeline))
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            MinutesField(stringResource(R.string.session_onset), current.onset, Modifier.weight(1f)) { draft = current.copy(onset = it) }
            MinutesField(stringResource(R.string.session_peak), current.peak, Modifier.weight(1f)) { draft = current.copy(peak = it) }
            MinutesField(stringResource(R.string.session_duration), current.duration, Modifier.weight(1f)) { draft = current.copy(duration = it) }
        }

        SectionTitle(stringResource(R.string.session_outcome))
        Text(stringResource(R.string.session_mood_after), style = MaterialTheme.typography.labelLarge)
        ScaleRow(current.moodAfter, { draft = current.copy(moodAfter = it) }, stringResource(R.string.scale_low), stringResource(R.string.scale_high))
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

        error?.let {
            Text(
                stringResource(if (it == DraftError.DOSE) R.string.error_dose else R.string.error_timeline),
                color = MaterialTheme.colorScheme.error,
            )
        }
        if (id != null) {
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

@Composable
private fun LineEditor(
    line: LineDraft,
    products: List<Entry<ProductData>>,
    vocab: Vocabulary,
    onChange: (LineDraft) -> Unit,
    onRemove: () -> Unit,
) {
    OutlinedCard(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            val none = stringResource(R.string.session_no_product)
            Dropdown(
                label = stringResource(R.string.session_product),
                options = listOf<Pair<String?, String>>(null to none) + products.map { it.id to it.data.name },
                selected = line.product,
                onSelect = { productId ->
                    val form = products.firstOrNull { it.id == productId }?.data?.form
                    val method = form?.let(::defaultMethodForForm) ?: line.method
                    onChange(line.copy(product = productId, method = method, doseUnit = unitFor(method, line.doseUnit)))
                },
            )
            Dropdown(
                label = stringResource(R.string.session_method),
                options = vocab.methods.map { it to label(it) },
                selected = line.method,
                onSelect = { method -> onChange(line.copy(method = method, doseUnit = unitFor(method, line.doseUnit))) },
            )
            val units = doseUnitsForMethod(line.method)
            if (units.isNotEmpty()) {
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
@Composable
private fun EffectsEditor(
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

private const val DEFAULT_INTENSITY = 3
private const val MAX_INTENSITY = 5
