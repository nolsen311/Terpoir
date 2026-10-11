// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.TagData
import app.terpoir.core.TagProvenance
import app.terpoir.core.TagRefData
import app.terpoir.core.TagSuggestion
import app.terpoir.core.Vocabulary

/** The phrase a suggestion came from, quoted from [text] by its UTF-16 span. */
fun phraseOf(text: String, suggestion: TagSuggestion): String {
    val start = suggestion.start.toInt().coerceIn(0, text.length)
    val end = suggestion.end.toInt().coerceIn(start, text.length)
    return text.substring(start, end)
}

/**
 * A note's tags: confirmed ones (removable), then keyword suggestions, each
 * quoting the phrase it came from, to accept or reject. Only confirmed
 * tags are stored, with where they came from (docs/design-decisions.md,
 * "Tag extraction").
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun NoteTags(
    text: String,
    tags: List<TagProvenance>,
    suggestions: List<TagSuggestion>,
    customNames: Map<String, String>,
    vocab: Vocabulary,
    customTags: List<Entry<TagData>>,
    onAccept: (TagSuggestion) -> Unit,
    onReject: (TagSuggestion) -> Unit,
    onAdd: (TagRefData) -> Unit,
    onRemove: (TagProvenance) -> Unit,
) {
    var picking by remember { mutableStateOf(false) }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        SectionTitle(stringResource(R.string.note_tags))
        if (tags.isNotEmpty()) {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (t in tags) {
                    InputChip(
                        selected = true,
                        onClick = { onRemove(t) },
                        label = { Text(tagLabel(t.tag, customNames)) },
                        trailingIcon = {
                            Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.remove))
                        },
                    )
                }
            }
        }
        if (suggestions.isNotEmpty()) {
            Text(stringResource(R.string.note_suggestions), style = MaterialTheme.typography.labelLarge)
            for (s in suggestions) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    AssistChip(onClick = { onAccept(s) }, label = { Text(tagLabel(s.tag, customNames)) })
                    Text("“${phraseOf(text, s)}”", style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
                    TextButton(onClick = { onAccept(s) }) { Text(stringResource(R.string.note_accept)) }
                    TextButton(onClick = { onReject(s) }) { Text(stringResource(R.string.note_reject)) }
                }
            }
        }
        OutlinedButton(onClick = { picking = true }) { Text(stringResource(R.string.note_add_tag)) }
    }

    if (picking) {
        val chosen = tags.map { it.tag }.toSet()
        AlertDialog(
            onDismissRequest = { picking = false },
            title = { Text(stringResource(R.string.note_add_tag)) },
            text = {
                Column(modifier = Modifier.verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    val groups = listOf(
                        R.string.tag_group_effects to vocab.coreEffects + customOf(customTags, EFFECT_CATEGORY),
                        R.string.tag_group_flavors to vocab.flavors + customOf(customTags, "flavor"),
                        R.string.tag_group_moods to vocab.moods + customOf(customTags, "mood"),
                        R.string.tag_group_terpenes to vocab.terpenes + customOf(customTags, "terpene"),
                    )
                    for ((title, slugsOrIds) in groups) {
                        Text(stringResource(title), style = MaterialTheme.typography.labelLarge)
                        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            for (key in slugsOrIds) {
                                val tag = if (key.startsWith(CUSTOM_PREFIX)) {
                                    TagRefData.Custom(key.removePrefix(CUSTOM_PREFIX))
                                } else {
                                    TagRefData.Builtin(key)
                                }
                                FilterChip(
                                    selected = tag in chosen,
                                    enabled = tag !in chosen,
                                    onClick = {
                                        onAdd(tag)
                                        picking = false
                                    },
                                    label = { Text(tagLabel(tag, customNames)) },
                                )
                            }
                        }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { picking = false }) { Text(stringResource(R.string.close)) } },
        )
    }
}

private const val CUSTOM_PREFIX = "custom:"

private fun customOf(tags: List<Entry<TagData>>, category: String): List<String> =
    tags.filter { it.data.category == category }.map { CUSTOM_PREFIX + it.id }
