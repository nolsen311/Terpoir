// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import app.terpoir.R
import app.terpoir.core.LocalTimeData
import app.terpoir.core.SnippetData
import app.terpoir.core.TagData
import app.terpoir.core.TagProvenance
import app.terpoir.core.TagRefData
import app.terpoir.core.TagSuggestion
import app.terpoir.core.Vocabulary
import app.terpoir.privacy.PrivacySettings
import app.terpoir.voice.OnDeviceSpeech
import app.terpoir.voice.SpeechProblem
import java.time.ZonedDateTime
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * A stream note: new ones open in voice mode when the user chose voice
 * first, existing ones open for editing. Voice uses the on-device
 * recognizer only and never falls back to an online one; the audio is not
 * kept. Saved as a stream snippet: standalone, or on [session].
 *
 * Keyword tag suggestions (matched by the core, on the device) appear as
 * the text changes; only the ones the user accepts or adds are saved.
 */
@Composable
fun NoteScreen(
    repo: JournalRepository,
    settings: PrivacySettings,
    vocab: Vocabulary,
    id: String?,
    onDone: () -> Unit,
    session: String? = null,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val speech = remember { OnDeviceSpeech(context) }
    var original by remember { mutableStateOf<SnippetData?>(null) }
    var loaded by remember { mutableStateOf(id == null) }
    var text by remember { mutableStateOf("") }
    var partial by remember { mutableStateOf("") }
    var listening by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<SpeechProblem?>(null) }
    var voice by remember { mutableStateOf(id == null && settings.voiceFirst && settings.voiceIntroSeen) }
    var intro by remember { mutableStateOf(id == null && !settings.voiceIntroSeen) }
    var tags by remember { mutableStateOf<List<TagProvenance>>(emptyList()) }
    var suggestions by remember { mutableStateOf<List<TagSuggestion>>(emptyList()) }
    var rejected by remember { mutableStateOf<Set<TagRefData>>(emptySet()) }
    var customTags by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var micGranted by remember {
        mutableStateOf(ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED)
    }

    fun listen() {
        problem = null
        partial = ""
        listening = true
        speech.listen(
            onPartial = { partial = it },
            onDone = { result, error ->
                listening = false
                partial = ""
                problem = error
                if (result != null) text = listOf(text.trimEnd(), result).filter { it.isNotEmpty() }.joinToString(" ")
            },
        )
    }

    val askMic = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        micGranted = granted
        if (granted) listen() else problem = SpeechProblem.NO_PERMISSION
    }
    fun startVoice() = if (micGranted) listen() else askMic.launch(Manifest.permission.RECORD_AUDIO)

    LaunchedEffect(id) {
        customTags = repo.tags(EFFECT_CATEGORY) + repo.tags("flavor") + repo.tags("mood") + repo.tags("terpene")
        id?.let { repo.snippet(it) }?.let {
            original = it
            text = it.text
            tags = it.tags
        }
        loaded = true
    }
    // Suggestions follow the text, after a short pause in typing.
    LaunchedEffect(text, tags, rejected) {
        delay(SUGGEST_DELAY_MS)
        val have = tags.map { it.tag }.toSet() + rejected
        suggestions = if (text.isBlank()) emptyList() else repo.suggestTags(text).filter { it.tag !in have }
    }
    // Start listening as soon as voice mode opens, as a voice-first button should.
    LaunchedEffect(voice) { if (voice && speech.available) startVoice() }
    DisposableEffect(Unit) { onDispose { speech.cancel() } }
    if (!loaded) return

    EditorScaffold(
        title = stringResource(if (id == null) R.string.note_new else R.string.note_edit),
        onClose = onDone,
        saveEnabled = text.isNotBlank() && !listening,
        onSave = {
            val now = ZonedDateTime.now()
            val data = original?.copy(text = text.trim(), tags = tags) ?: SnippetData(
                session = session,
                at = LocalTimeData(now.toInstant().toEpochMilli(), (now.offset.totalSeconds / 60).toShort()),
                text = text.trim(),
                tags = tags,
            )
            scope.launch {
                repo.saveSnippet(id, data)
                onDone()
            }
        },
    ) {
        if (voice) {
            if (!speech.available) {
                Text(stringResource(R.string.voice_unavailable), style = MaterialTheme.typography.bodyMedium)
            } else {
                Text(stringResource(if (listening) R.string.voice_listening else R.string.voice_on_device), style = MaterialTheme.typography.bodyMedium)
                if (partial.isNotEmpty()) Text(partial, style = MaterialTheme.typography.bodyLarge)
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (listening) {
                        Button(onClick = { speech.stop() }) { Text(stringResource(R.string.voice_stop)) }
                    } else {
                        Button(onClick = { startVoice() }) { Text(stringResource(R.string.voice_speak)) }
                    }
                }
                problem?.let { Text(stringResource(it.message), color = MaterialTheme.colorScheme.error) }
            }
        }
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            label = { Text(stringResource(R.string.note_text)) },
            modifier = Modifier.fillMaxWidth().heightIn(min = 160.dp),
        )
        NoteTags(
            text = text,
            tags = tags,
            suggestions = suggestions,
            customNames = customTags.associate { it.id to it.data.name },
            vocab = vocab,
            customTags = customTags,
            onAccept = { s -> tags = tags + TagProvenance(s.tag, "keyword") },
            onReject = { s -> rejected = rejected + s.tag },
            onAdd = { tag -> tags = tags + TagProvenance(tag, "user") },
            onRemove = { t -> tags = tags - t },
        )
        if (id == null) {
            OutlinedButton(onClick = {
                if (voice) speech.cancel()
                listening = false
                voice = !voice
            }) { Text(stringResource(if (voice) R.string.voice_switch_to_typing else R.string.voice_switch_to_voice)) }
            Text(stringResource(R.string.voice_nearby), style = MaterialTheme.typography.bodySmall)
        }
        if (id != null) DeleteButton(R.string.note_delete, R.string.note_delete_body) { scope.launch { repo.delete(id); onDone() } }
    }

    if (intro) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text(stringResource(R.string.voice_intro_title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(R.string.voice_intro_on_device))
                    Text(stringResource(R.string.voice_intro_keyboards))
                    Text(stringResource(R.string.voice_nearby))
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    settings.voiceFirst = true
                    settings.voiceIntroSeen = true
                    intro = false
                    voice = true
                }) { Text(stringResource(R.string.voice_intro_voice)) }
            },
            dismissButton = {
                TextButton(onClick = {
                    settings.voiceFirst = false
                    settings.voiceIntroSeen = true
                    intro = false
                }) { Text(stringResource(R.string.voice_intro_text)) }
            },
        )
    }
}

private const val SUGGEST_DELAY_MS = 400L

private val SpeechProblem.message: Int
    get() = when (this) {
        SpeechProblem.LANGUAGE_UNAVAILABLE -> R.string.voice_language_unavailable
        SpeechProblem.NOTHING_HEARD -> R.string.voice_nothing_heard
        SpeechProblem.NO_PERMISSION -> R.string.voice_no_permission
        SpeechProblem.OTHER -> R.string.voice_failed
    }
