// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.journal.EditorScaffold
import app.terpoir.privacy.PrivacySettings
import app.terpoir.voice.DictationControls
import app.terpoir.voice.appendSpoken
import app.terpoir.voice.rememberDictation

/**
 * A new note while the journal is locked (write-only capture). The core
 * seals it to the capture public key as soon as it is saved, so nothing
 * here can be read back, edited or tagged until the journal is unlocked;
 * then it appears marked "added while locked" for the user to keep or
 * discard. Voice or typing as in the unlocked note screen.
 *
 * [onSave] returns false if the note could not be sealed.
 */
@Composable
fun LockedCaptureScreen(
    settings: PrivacySettings,
    onSave: (String) -> Boolean,
    onUnlock: () -> Unit,
    onClose: () -> Unit,
) {
    var text by remember { mutableStateOf("") }
    var voice by remember { mutableStateOf(settings.voiceFirst && settings.voiceIntroSeen) }
    var failed by remember { mutableStateOf(false) }
    val dictation = rememberDictation { spoken -> text = appendSpoken(text, spoken) }
    LaunchedEffect(voice) { if (voice && dictation.available) dictation.start() }

    EditorScaffold(
        title = stringResource(R.string.capture_locked_title),
        onClose = onClose,
        saveEnabled = text.isNotBlank() && !dictation.listening,
        onSave = {
            failed = !onSave(text.trim())
            if (!failed) text = ""
        },
    ) {
        Text(stringResource(R.string.capture_locked_explainer), style = MaterialTheme.typography.bodyMedium)
        if (voice) DictationControls(dictation)
        OutlinedTextField(
            value = text,
            onValueChange = { text = it },
            label = { Text(stringResource(R.string.note_text)) },
            modifier = Modifier.fillMaxWidth().heightIn(min = 160.dp),
        )
        if (failed) Text(stringResource(R.string.capture_locked_failed), color = MaterialTheme.colorScheme.error)
        OutlinedButton(onClick = {
            if (voice) dictation.cancel()
            voice = !voice
        }) { Text(stringResource(if (voice) R.string.voice_switch_to_typing else R.string.voice_switch_to_voice)) }
        Text(stringResource(R.string.voice_nearby), style = MaterialTheme.typography.bodySmall)
        TextButton(onClick = onUnlock) { Text(stringResource(R.string.capture_locked_unlock)) }
    }
}
