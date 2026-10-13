// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.sync

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.Journal
import app.terpoir.journal.formatWhen
import java.time.ZoneId
import kotlinx.coroutines.launch

/**
 * Folder sync in the Privacy Center, while its toggle is on: choose the
 * folder (system picker), sync now, and when it last synced.
 */
@Composable
fun FolderSyncCard(sync: FolderSync, journal: Journal) {
    val scope = rememberCoroutineScope()
    var revision by remember { mutableIntStateOf(0) }
    var message by remember { mutableStateOf<String?>(null) }
    var failed by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var needsUpdate by remember { mutableStateOf(false) }
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        if (uri != null) {
            sync.choose(uri)
            revision++
        }
    }
    val doneText = stringResource(R.string.sync_done)
    val failedText = stringResource(R.string.sync_failed)
    val importedTemplate = R.plurals.sync_imported
    var imported by remember { mutableIntStateOf(-1) }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(stringResource(R.string.sync_title), style = MaterialTheme.typography.titleMedium)
        val folder = revision.let { sync.folder }
        if (folder == null) {
            Text(stringResource(R.string.sync_no_folder), style = MaterialTheme.typography.bodySmall)
            Button(onClick = { pick.launch(null) }) { Text(stringResource(R.string.sync_choose)) }
            return@Column
        }
        val last = revision.let { sync.lastSyncMs }
        Text(
            if (last == 0L) {
                stringResource(R.string.sync_never)
            } else {
                val offset = ZoneId.systemDefault().rules.getOffset(java.time.Instant.ofEpochMilli(last)).totalSeconds / 60
                stringResource(R.string.sync_last, formatWhen(last, offset.toShort()))
            },
            style = MaterialTheme.typography.bodySmall,
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                enabled = !busy,
                onClick = {
                    busy = true
                    scope.launch {
                        val outcome = runCatching { sync.sync(journal) }.getOrNull()
                        failed = outcome == null
                        imported = outcome?.imported ?: -1
                        needsUpdate = outcome?.needsUpdate == true
                        message = if (outcome == null) failedText else doneText
                        busy = false
                        revision++
                    }
                },
            ) { Text(stringResource(R.string.sync_now)) }
            OutlinedButton(onClick = { pick.launch(null) }) { Text(stringResource(R.string.sync_change)) }
        }
        message?.let {
            Text(it, style = MaterialTheme.typography.bodySmall, color = if (failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
        }
        if (imported > 0) Text(pluralStringResource(importedTemplate, imported, imported), style = MaterialTheme.typography.bodySmall)
        if (needsUpdate) Text(stringResource(R.string.sync_needs_update), style = MaterialTheme.typography.bodySmall)
    }
}
