// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import app.terpoir.R
import app.terpoir.core.LocalTimeData
import app.terpoir.core.SnippetData
import app.terpoir.core.TagData
import app.terpoir.core.TagRefData
import app.terpoir.core.Vocabulary
import java.time.Instant
import java.time.ZoneId
import kotlinx.coroutines.launch

/**
 * The follow-up check-in: one screen to update a session's effects, mood
 * and rating, and add a note to its stream. Opened from the "Quick
 * check-in?" reminder after unlock.
 */
@Composable
fun CheckInScreen(repo: JournalRepository, vocab: Vocabulary, sessionId: String, onDone: () -> Unit) {
    val scope = rememberCoroutineScope()
    var draft by remember { mutableStateOf<SessionDraft?>(null) }
    var missing by remember { mutableStateOf(false) }
    var customEffects by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var note by remember { mutableStateOf("") }
    LaunchedEffect(sessionId) {
        customEffects = repo.tags(EFFECT_CATEGORY)
        val session = repo.session(sessionId)
        if (session == null) missing = true else draft = SessionDraft.of(session)
    }
    if (missing) {
        // Deleted since the check-in was scheduled.
        LaunchedEffect(Unit) { onDone() }
        return
    }
    val current = draft ?: return
    EditorScaffold(
        title = stringResource(R.string.check_in_screen),
        onClose = onDone,
        onSave = {
            val result = current.toData()
            if (result is SessionDraft.Result.Valid) {
                scope.launch {
                    repo.saveSession(sessionId, result.data)
                    if (note.isNotBlank()) {
                        val now = System.currentTimeMillis()
                        val offset = ZoneId.systemDefault().rules.getOffset(Instant.ofEpochMilli(now)).totalSeconds / 60
                        repo.saveSnippet(null, SnippetData(sessionId, LocalTimeData(now, offset.toShort()), note.trim(), emptyList()))
                    }
                    onDone()
                }
            }
        },
    ) {
        Text(stringResource(R.string.check_in_prompt), style = MaterialTheme.typography.bodyMedium)
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
        Text(stringResource(R.string.session_mood_after), style = MaterialTheme.typography.labelLarge)
        ScaleRow(current.moodAfter, { draft = current.copy(moodAfter = it) }, stringResource(R.string.scale_low), stringResource(R.string.scale_high))
        Text(stringResource(R.string.session_rating), style = MaterialTheme.typography.labelLarge)
        ScaleRow(current.rating, { draft = current.copy(rating = it) }, stringResource(R.string.rating_low), stringResource(R.string.rating_high))
        OutlinedTextField(
            value = note,
            onValueChange = { note = it },
            label = { Text(stringResource(R.string.check_in_note)) },
            minLines = 2,
            modifier = Modifier.fillMaxWidth(),
        )
    }
}
