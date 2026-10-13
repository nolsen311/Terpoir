// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.voice

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import app.terpoir.R

/**
 * Speaking a note: the on-device recognizer, the microphone permission and
 * the listening state. Never falls back to online recognition; the audio
 * is not kept. Create with [rememberDictation].
 */
class Dictation internal constructor(private val speech: OnDeviceSpeech) {
    /** Whether the phone has an on-device recognizer at all. */
    val available: Boolean get() = speech.available

    /** Listening now. */
    var listening by mutableStateOf(false)
        private set

    /** Words heard so far, before the result is final. */
    var partial by mutableStateOf("")
        private set

    /** Why the last attempt failed, if it did. */
    var problem by mutableStateOf<SpeechProblem?>(null)
        internal set

    /** The microphone isn't allowed yet: the reason shows before Android's prompt. */
    var explaining by mutableStateOf(false)
        private set

    internal var begin: () -> Unit = {}
    internal var askMic: () -> Unit = {}

    internal fun explain() {
        explaining = true
    }

    /** After the explanation: shows Android's microphone prompt. */
    fun allowMic() {
        explaining = false
        askMic()
    }

    /** Starts listening, asking for the microphone first if needed. */
    fun start() = begin()

    /** Stops listening and keeps what was heard. */
    fun stop() = speech.stop()

    /** Stops listening and drops what was heard. */
    fun cancel() {
        speech.cancel()
        listening = false
        partial = ""
    }

    internal fun listen(onText: (String) -> Unit) {
        problem = null
        partial = ""
        listening = true
        speech.listen(
            onPartial = { partial = it },
            onDone = { result, error ->
                listening = false
                partial = ""
                problem = error
                if (result != null) onText(result)
            },
        )
    }
}

/** A [Dictation] that hands each recognized phrase to [onText]. */
@Composable
fun rememberDictation(onText: (String) -> Unit): Dictation {
    val context = LocalContext.current
    val dictation = remember { Dictation(OnDeviceSpeech(context)) }
    val currentOnText by rememberUpdatedState(onText)
    var micGranted by remember {
        mutableStateOf(ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO) == PackageManager.PERMISSION_GRANTED)
    }
    val askMic = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        micGranted = granted
        if (granted) dictation.listen { currentOnText(it) } else dictation.problem = SpeechProblem.NO_PERMISSION
    }
    dictation.begin = {
        if (micGranted) dictation.listen { currentOnText(it) } else dictation.explain()
    }
    dictation.askMic = { askMic.launch(Manifest.permission.RECORD_AUDIO) }
    DisposableEffect(Unit) { onDispose { dictation.cancel() } }
    return dictation
}

/** Appends a recognized phrase to a note's text. */
fun appendSpoken(text: String, spoken: String): String =
    listOf(text.trimEnd(), spoken).filter { it.isNotEmpty() }.joinToString(" ")

/** What the user sees while speaking a note: status, words so far, Speak/Done, problems. */
@Composable
fun DictationControls(dictation: Dictation) {
    if (!dictation.available) {
        Text(stringResource(R.string.voice_unavailable), style = MaterialTheme.typography.bodyMedium)
        return
    }
    if (dictation.explaining) {
        Text(stringResource(R.string.voice_mic_why), style = MaterialTheme.typography.bodyMedium)
        Button(onClick = dictation::allowMic) { Text(stringResource(R.string.continue_label)) }
        return
    }
    Text(stringResource(if (dictation.listening) R.string.voice_listening else R.string.voice_on_device), style = MaterialTheme.typography.bodyMedium)
    if (dictation.partial.isNotEmpty()) Text(dictation.partial, style = MaterialTheme.typography.bodyLarge)
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        if (dictation.listening) {
            Button(onClick = dictation::stop) { Text(stringResource(R.string.voice_stop)) }
        } else {
            Button(onClick = dictation::start) { Text(stringResource(R.string.voice_speak)) }
        }
    }
    dictation.problem?.let { Text(stringResource(it.message), color = MaterialTheme.colorScheme.error) }
}

private val SpeechProblem.message: Int
    get() = when (this) {
        SpeechProblem.LANGUAGE_UNAVAILABLE -> R.string.voice_language_unavailable
        SpeechProblem.NOTHING_HEARD -> R.string.voice_nothing_heard
        SpeechProblem.NO_PERMISSION -> R.string.voice_no_permission
        SpeechProblem.OTHER -> R.string.voice_failed
    }
