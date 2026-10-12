// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.session.JournalSession.RecoveryCheck

/**
 * Restoring a journal from an encrypted export on a new device: the
 * recovery key opens it. The key is checked against the export before
 * anything is written; [onRestore] then sets up the journal.
 */
@Composable
fun RestoreScreen(check: (String) -> RecoveryCheck, onRestore: (String) -> Unit, onCancel: () -> Unit) {
    var typed by remember { mutableStateOf("") }
    var result by remember { mutableStateOf<RecoveryCheck?>(null) }
    Column(modifier = Modifier.padding(24.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Text(stringResource(R.string.restore_title), style = MaterialTheme.typography.headlineSmall)
        Text(stringResource(R.string.restore_explainer))
        OutlinedTextField(
            value = typed,
            onValueChange = {
                typed = it
                result = null
            },
            label = { Text(stringResource(R.string.confirm_label)) },
            modifier = Modifier.fillMaxWidth(),
        )
        result?.let { Text(stringResource(it.message), color = MaterialTheme.colorScheme.error) }
        Button(onClick = {
            val outcome = check(typed)
            if (outcome == RecoveryCheck.MATCHES) onRestore(typed) else result = outcome
        }) { Text(stringResource(R.string.restore_button)) }
        TextButton(onClick = onCancel) { Text(stringResource(R.string.cancel)) }
    }
}
