// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AssistChip
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SuggestionChip
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import app.terpoir.R

/** A full-screen editor: close, title, save; scrolling content. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EditorScaffold(
    title: String,
    onClose: () -> Unit,
    // Null on screens that apply each change at once (settings, views):
    // no Save button, so nothing suggests a change could be lost.
    onSave: (() -> Unit)?,
    saveEnabled: Boolean = true,
    floatingActionButton: (@Composable () -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    BackHandler(onBack = onClose)
    Scaffold(
        floatingActionButton = { floatingActionButton?.invoke() },
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    IconButton(onClick = onClose) {
                        Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.close))
                    }
                },
                actions = {
                    if (onSave != null) TextButton(onClick = onSave, enabled = saveEnabled) { Text(stringResource(R.string.save)) }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.padding(padding).verticalScroll(rememberScrollState()).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            content()
            // Room so a floating button doesn't cover the last field.
            if (floatingActionButton != null) Spacer(Modifier.height(72.dp))
        }
    }
}

/** A section heading inside an editor. */
@Composable
fun SectionTitle(text: String) {
    Text(text, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 8.dp))
}

/** Pick at most one of [options] (slugs); tapping the selected one clears it. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SingleChoiceChips(options: List<String>, selected: String?, onSelect: (String?) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (option in options) {
            FilterChip(
                selected = option == selected,
                onClick = { onSelect(if (option == selected) null else option) },
                label = { Text(label(option)) },
            )
        }
    }
}

/** A 1–5 scale; tapping the selected value clears it. */
@Composable
fun ScaleRow(value: Int?, onChange: (Int?) -> Unit, low: String, high: String) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (n in 1..5) {
                FilterChip(
                    selected = value == n,
                    onClick = { onChange(if (value == n) null else n) },
                    label = { Text(n.toString()) },
                )
            }
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text(low, style = MaterialTheme.typography.bodySmall)
            Text(high, style = MaterialTheme.typography.bodySmall)
        }
    }
}

/**
 * The rating faces, 1 to 5, from meh to mind-blown (maintainer, 2026-10-15).
 * The rating is still stored as a number; the faces are only how it shows.
 */
val RATING_FACES = listOf("\uD83E\uDEE4", "\uD83D\uDE11", "\uD83D\uDE00", "\uD83E\uDEE0", "\uD83E\uDD2F")

/** Stand-ins where a face is missing: 🫤 and 🫠 came with Emoji 14, which Android 11 and 12 lack. */
private val RATING_FALLBACK = listOf("\uD83D\uDE15", "\uD83D\uDE11", "\uD83D\uDE00", "\uD83E\uDD74", "\uD83E\uDD2F")

/** The face for [rating] (1–5) that this device can draw. */
fun ratingFace(rating: Int): String {
    val face = RATING_FACES[rating - 1]
    return if (android.graphics.Paint().hasGlyph(face)) face else RATING_FALLBACK[rating - 1]
}

/** One step down: the lowest rating clears it. */
fun ratingDown(value: Int?): Int? = when (value) {
    // Unrated shows the middle face, so − starts one below it.
    null -> RATING_MIDDLE - 1
    else -> if (value <= 1) null else (value - 1).coerceAtMost(5)
}

/** One step up from the face shown (the middle one when unrated); never past the top. */
fun ratingUp(value: Int?): Int = ((value ?: RATING_MIDDLE) + 1).coerceIn(1, 5)

/** The face an unrated session shows, faded (maintainer, 2026-10-16). */
const val RATING_MIDDLE = 3

/**
 * A 1–5 rating as − and + around a face, centred (maintainer, 2026-10-16).
 * Unrated, it shows the middle face faded: + and − step from there, and
 * tapping the face picks it. − at the lowest clears the rating. Screen
 * readers hear the rating's name.
 */
@Composable
fun RatingStepper(value: Int?, onChange: (Int?) -> Unit) {
    val names = listOf(
        stringResource(R.string.rating_1),
        stringResource(R.string.rating_2),
        stringResource(R.string.rating_3),
        stringResource(R.string.rating_4),
        stringResource(R.string.rating_5),
    )
    val current = value?.coerceIn(1, 5)
    val shown = current ?: RATING_MIDDLE
    val state = current?.let { stringResource(R.string.rating_state, names[it - 1], it) } ?: stringResource(R.string.rating_none)
    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(16.dp),
            modifier = Modifier.semantics(mergeDescendants = false) { stateDescription = state },
        ) {
            IconButton(onClick = { onChange(ratingDown(current)) }) {
                Icon(painterResource(R.drawable.ic_remove), contentDescription = stringResource(R.string.rating_lower))
            }
            val pick = stringResource(R.string.rating_pick, names[RATING_MIDDLE - 1])
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier
                    .width(112.dp)
                    .then(
                        if (current == null) {
                            Modifier.clickable(onClickLabel = pick) { onChange(RATING_MIDDLE) }
                        } else {
                            Modifier
                        },
                    ),
            ) {
                Text(
                    remember(shown) { ratingFace(shown) },
                    fontSize = 44.sp,
                    textAlign = TextAlign.Center,
                    modifier = Modifier.alpha(if (current == null) 0.35f else 1f),
                )
                Text(current?.let { names[it - 1] } ?: stringResource(R.string.rating_none), style = MaterialTheme.typography.bodySmall)
            }
            IconButton(onClick = { onChange(ratingUp(current)) }, enabled = current != 5) {
                Icon(painterResource(R.drawable.ic_add), contentDescription = stringResource(R.string.rating_raise))
            }
        }
    }
}

/**
 * Picks several items from a list that can grow long (strains): the chosen
 * ones as chips that a tap removes, and a search field whose matches can be
 * added, at most [SEARCH_PICKER_MATCHES] at a time. Names starting with
 * what was typed come first. When nothing matches exactly, [onNew] offers
 * the typed name as a new item.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SearchPicker(
    all: List<Pair<String, String>>,
    selected: List<String>,
    searchLabel: String,
    missingLabel: String,
    onAdd: (String) -> Unit,
    onRemove: (String) -> Unit,
    newLabel: (@Composable (String) -> String)? = null,
    onNew: ((String) -> Unit)? = null,
) {
    var query by remember { mutableStateOf("") }
    val names = remember(all) { all.toMap() }
    if (selected.isNotEmpty()) {
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (id in selected) {
                val name = names[id] ?: missingLabel
                InputChip(
                    selected = true,
                    onClick = { onRemove(id) },
                    label = { Text(name) },
                    trailingIcon = {
                        Icon(
                            painterResource(R.drawable.ic_close),
                            contentDescription = stringResource(R.string.remove_named, name),
                            modifier = Modifier.size(18.dp),
                        )
                    },
                )
            }
        }
    }
    OutlinedTextField(
        query,
        { query = it },
        label = { Text(searchLabel) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
    val typed = query.trim()
    if (typed.isNotEmpty()) {
        val matches = all
            .filter { (id, name) -> id !in selected && name.contains(typed, ignoreCase = true) }
            .sortedWith(compareBy({ !it.second.startsWith(typed, ignoreCase = true) }, { it.second.lowercase() }))
            .take(SEARCH_PICKER_MATCHES)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for ((id, name) in matches) {
                SuggestionChip(onClick = { onAdd(id); query = "" }, label = { Text(name) })
            }
            val exact = all.any { it.second.equals(typed, ignoreCase = true) }
            if (onNew != null && newLabel != null && !exact) {
                AssistChip(
                    onClick = { onNew(typed); query = "" },
                    label = { Text(newLabel(typed)) },
                    leadingIcon = { Icon(painterResource(R.drawable.ic_add), contentDescription = null, modifier = Modifier.size(18.dp)) },
                )
            }
        }
    }
}

/** How many matches [SearchPicker] shows at once. */
const val SEARCH_PICKER_MATCHES = 8

/** A labelled dropdown over [options] (value to display text). */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun <T> Dropdown(
    label: String,
    options: List<Pair<T, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    modifier: Modifier = Modifier,
) {
    var expanded by remember { mutableStateOf(false) }
    ExposedDropdownMenuBox(expanded = expanded, onExpandedChange = { expanded = it }, modifier = modifier) {
        OutlinedTextField(
            value = options.firstOrNull { it.first == selected }?.second.orEmpty(),
            onValueChange = {},
            readOnly = true,
            label = { Text(label) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded) },
            modifier = Modifier.menuAnchor(ExposedDropdownMenuAnchorType.PrimaryNotEditable).fillMaxWidth(),
        )
        ExposedDropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            for ((value, text) in options) {
                DropdownMenuItem(text = { Text(text) }, onClick = {
                    onSelect(value)
                    expanded = false
                })
            }
        }
    }
}
