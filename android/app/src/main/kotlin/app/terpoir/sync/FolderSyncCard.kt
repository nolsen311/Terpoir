// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.sync

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.Journal
import app.terpoir.journal.formatWhen
import app.terpoir.report.ReportButton
import java.time.ZoneId
import kotlinx.coroutines.launch

/**
 * Folder sync in the Privacy Center, while its toggle is on: choose the
 * folder (system picker), sync, and when it last synced. A green check
 * shows the last sync worked; each sync first proves the folder keeps a
 * file ([FolderSync.sync]), and choosing a folder syncs at once.
 */
@Composable
fun FolderSyncCard(sync: FolderSync, journal: Journal) {
    val scope = rememberCoroutineScope()
    var revision by remember { mutableIntStateOf(0) }
    var message by remember { mutableStateOf<String?>(null) }
    var failed by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var needsUpdate by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<Throwable?>(null) }
    val doneText = stringResource(R.string.sync_done)
    val failedText = stringResource(R.string.sync_failed)
    val noAccessText = stringResource(R.string.sync_no_access)
    val importedTemplate = R.plurals.sync_imported
    var imported by remember { mutableIntStateOf(-1) }
    fun syncNow() {
        busy = true
        scope.launch {
            val attempt = runCatching { sync.sync(journal) }
            val outcome = attempt.getOrNull()
            error = attempt.exceptionOrNull()
            failed = outcome == null
            imported = outcome?.imported ?: -1
            needsUpdate = outcome?.needsUpdate == true
            message = if (outcome == null) failedText else doneText
            busy = false
            revision++
        }
    }
    // A new folder syncs at once, so a folder that doesn't keep the files
    // shows up now rather than at the next unlock.
    val pick = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        if (sync.choose(uri)) {
            syncNow()
        } else {
            failed = true
            message = noAccessText
            revision++
        }
    }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(stringResource(R.string.sync_title), style = MaterialTheme.typography.titleMedium)
        val folder = revision.let { sync.folder }
        if (folder == null) {
            Text(stringResource(R.string.sync_no_folder), style = MaterialTheme.typography.bodySmall)
            Button(onClick = { pick.launch(null) }) { Text(stringResource(R.string.sync_choose)) }
            message?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error) }
            return@Column
        }
        val last = revision.let { sync.lastSyncMs }
        val working = revision.let { sync.working }
        when {
            busy -> Text(stringResource(R.string.sync_syncing), style = MaterialTheme.typography.bodySmall)
            working == true -> WorkingBadge()
            // A failure here may be from the sync at unlock, not one started on this screen.
            working == false && message == null ->
                Text(stringResource(R.string.sync_last_failed), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }
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
                onClick = ::syncNow,
            ) { Text(stringResource(R.string.sync_now)) }
            OutlinedButton(onClick = { pick.launch(null) }) { Text(stringResource(R.string.sync_change)) }
        }
        message?.let {
            Text(it, style = MaterialTheme.typography.bodySmall, color = if (failed) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.onSurface)
            if (failed) ReportButton(R.string.sync_failed, error, folder)
        }
        if (imported > 0) Text(pluralStringResource(importedTemplate, imported, imported), style = MaterialTheme.typography.bodySmall)
        if (needsUpdate) Text(stringResource(R.string.sync_needs_update), style = MaterialTheme.typography.bodySmall)
    }
}

/** A green check in a circle and "Working": the last sync worked. */
@Composable
private fun WorkingBadge() {
    val circle = MaterialTheme.colorScheme.primary
    val tick = MaterialTheme.colorScheme.onPrimary
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
        Canvas(Modifier.size(18.dp)) {
            drawCircle(circle)
            val w = size.width
            val check = Path().apply {
                moveTo(w * 0.28f, w * 0.52f)
                lineTo(w * 0.44f, w * 0.68f)
                lineTo(w * 0.73f, w * 0.36f)
            }
            drawPath(check, tick, style = Stroke(width = w * 0.12f, cap = StrokeCap.Round, join = StrokeJoin.Round))
        }
        Text(stringResource(R.string.sync_working), style = MaterialTheme.typography.labelLarge, color = circle)
    }
}
