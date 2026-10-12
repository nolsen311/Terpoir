// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.CoreException
import java.time.LocalDate
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** What the last export or import did. */
private sealed interface DataResult {
    data object Saved : DataResult

    data object SavedPlaintext : DataResult

    data class Imported(val records: Long) : DataResult

    data class Failed(val message: Int) : DataResult
}

/**
 * "Your data": the encrypted export (the whole journal, which only the
 * recovery key opens), importing one back, and the opt-in plaintext export
 * (a ZIP of JSON and CSV files) behind a strong warning. Files go only
 * where the user picks them in the system file picker.
 */
@Composable
fun DataCard(repo: JournalRepository, masterWrappedByRecovery: () -> ByteArray) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var result by remember { mutableStateOf<DataResult?>(null) }
    val save = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/octet-stream")) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            result = try {
                val bytes = repo.exportArchive(masterWrappedByRecovery())
                withContext(Dispatchers.IO) {
                    checkNotNull(context.contentResolver.openOutputStream(uri, "wt")).use { it.write(bytes) }
                }
                DataResult.Saved
            } catch (_: Exception) {
                DataResult.Failed(R.string.export_failed)
            }
        }
    }
    var warnPlaintext by remember { mutableStateOf(false) }
    val savePlaintext = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument("application/zip")) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            result = try {
                val files = repo.exportPlaintext()
                withContext(Dispatchers.IO) {
                    checkNotNull(context.contentResolver.openOutputStream(uri, "wt")).use { out ->
                        ZipOutputStream(out).use { zip ->
                            for (file in files) {
                                zip.putNextEntry(ZipEntry(file.name))
                                zip.write(file.bytes)
                                zip.closeEntry()
                                // Best effort: the plaintext lingers in memory no longer than needed.
                                file.bytes.fill(0)
                            }
                        }
                    }
                }
                DataResult.SavedPlaintext
            } catch (_: Exception) {
                DataResult.Failed(R.string.export_failed)
            }
        }
    }
    val open = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            result = try {
                val bytes = withContext(Dispatchers.IO) {
                    checkNotNull(context.contentResolver.openInputStream(uri)).use { it.readBytes() }
                }
                DataResult.Imported(repo.importArchive(bytes))
            } catch (_: CoreException) {
                DataResult.Failed(R.string.import_not_this_journal)
            } catch (_: Exception) {
                DataResult.Failed(R.string.import_failed)
            }
        }
    }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(stringResource(R.string.data_title), style = MaterialTheme.typography.titleMedium)
        Text(stringResource(R.string.data_explainer), style = MaterialTheme.typography.bodySmall)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = { save.launch("terpoir-${LocalDate.now()}.terpoir") }) { Text(stringResource(R.string.export_button)) }
            OutlinedButton(onClick = { open.launch(arrayOf("*/*")) }) { Text(stringResource(R.string.import_button)) }
        }
        TextButton(onClick = { warnPlaintext = true }) { Text(stringResource(R.string.plaintext_button)) }
        when (val r = result) {
            null -> Unit
            DataResult.Saved -> Text(stringResource(R.string.export_saved), style = MaterialTheme.typography.bodySmall)
            DataResult.SavedPlaintext -> Text(stringResource(R.string.plaintext_saved), style = MaterialTheme.typography.bodySmall)
            is DataResult.Imported -> Text(
                pluralStringResource(R.plurals.import_done, r.records.toInt(), r.records.toInt()),
                style = MaterialTheme.typography.bodySmall,
            )
            is DataResult.Failed -> Text(stringResource(r.message), color = MaterialTheme.colorScheme.error)
        }
    }

    // A strong warning first: unlike everything else, this file is readable
    // by anyone and anything that gets it.
    if (warnPlaintext) {
        AlertDialog(
            onDismissRequest = { warnPlaintext = false },
            title = { Text(stringResource(R.string.plaintext_title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(R.string.plaintext_warning))
                    Text(stringResource(R.string.plaintext_cloud))
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    warnPlaintext = false
                    savePlaintext.launch("terpoir-plaintext-${LocalDate.now()}.zip")
                }) { Text(stringResource(R.string.plaintext_confirm), color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = {
                TextButton(onClick = { warnPlaintext = false }) { Text(stringResource(R.string.cancel)) }
            },
        )
    }
}
