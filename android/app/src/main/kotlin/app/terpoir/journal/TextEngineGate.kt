// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.app.Activity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
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
import app.terpoir.report.ReportButton
import app.terpoir.scan.EngineStatus
import app.terpoir.scan.Scanner
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.flow.catch

/**
 * Shows [content] (a text scan) once the scanner's text engine is ready.
 * An engine that must be downloaded first (Play) is offered, never fetched
 * on its own: the user agrees, or closes and types instead.
 *
 * Anything the engine throws (Play's delivery services can, for instance on
 * a copy not installed from Google Play) shows as a failed download, which
 * the user can retry or close, never as a crash.
 */
@Composable
fun TextEngineGate(scanner: Scanner, title: String, onClose: () -> Unit, content: @Composable () -> Unit) {
    val engine = remember(scanner) { scanner.textEngine() }
    var status by remember(engine) { mutableStateOf<EngineStatus?>(null) }
    var installing by remember(engine) { mutableStateOf(false) }
    LaunchedEffect(engine) {
        status = try {
            engine.status()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            EngineStatus.Failed(e)
        }
    }
    LaunchedEffect(installing) {
        if (installing) {
            engine.install()
                .catch { emit(EngineStatus.Failed(it)) }
                .collect { status = it }
            installing = false
        }
    }
    // The install flow reports what the store does after the dialog; only a
    // refusal needs handling here.
    val confirmation = rememberLauncherForActivityResult(ActivityResultContracts.StartIntentSenderForResult()) {
        if (it.resultCode != Activity.RESULT_OK) {
            installing = false
            status = EngineStatus.Failed(IllegalStateException("store confirmation refused (result ${it.resultCode})"))
        }
    }

    when (val current = status) {
        null -> Unit
        EngineStatus.Ready -> content()
        else -> EditorScaffold(title, onClose, onSave = null) {
            Text(stringResource(engine.downloadPrompt), style = MaterialTheme.typography.bodyMedium)
            when (current) {
                EngineStatus.NotInstalled, is EngineStatus.Failed -> {
                    if (current is EngineStatus.Failed) {
                        Text(stringResource(R.string.ocr_download_failed), style = MaterialTheme.typography.bodySmall)
                        ReportButton(R.string.ocr_download_failed, current.error)
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(onClick = { installing = true }) {
                            Text(stringResource(if (current is EngineStatus.Failed) R.string.ocr_retry else R.string.ocr_download))
                        }
                        OutlinedButton(onClick = onClose) { Text(stringResource(R.string.ocr_not_now)) }
                    }
                }
                is EngineStatus.Downloading -> {
                    Text(stringResource(R.string.ocr_downloading))
                    val fraction = current.fraction
                    if (fraction == null) {
                        LinearProgressIndicator(Modifier.fillMaxWidth())
                    } else {
                        LinearProgressIndicator(progress = { fraction }, modifier = Modifier.fillMaxWidth())
                    }
                }
                is EngineStatus.NeedsConfirmation -> {
                    LaunchedEffect(current) {
                        runCatching { current.confirm(confirmation) }.onFailure {
                            installing = false
                            status = EngineStatus.Failed(it)
                        }
                    }
                    Text(stringResource(R.string.ocr_downloading))
                }
                EngineStatus.Ready -> Unit
            }
        }
    }
}
