// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import app.terpoir.R
import kotlinx.coroutines.launch

/**
 * "What I track" (docs/design-decisions.md): the data points the journal
 * has. Everything starts shown. Turning one off asks whether to keep its
 * values, hidden, or delete them from every entry; the choice syncs, so
 * every paired device shows the same fields. When, product, method,
 * effects and rating are always tracked.
 */
@Composable
fun WhatITrackScreen(repo: JournalRepository, onChanged: () -> Unit, onClose: () -> Unit) {
    val scope = rememberCoroutineScope()
    var tracking by remember { mutableStateOf<Tracking?>(null) }
    var turningOff by remember { mutableStateOf<DataPoint?>(null) }
    var busy by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { tracking = repo.tracking() }

    fun change(action: suspend () -> Unit) {
        busy = true
        scope.launch {
            action()
            tracking = repo.tracking()
            busy = false
            onChanged()
        }
    }

    EditorScaffold(title = stringResource(R.string.track_title), onClose = onClose, onSave = null) {
        val current = tracking ?: return@EditorScaffold
        Text(stringResource(R.string.track_intro), style = MaterialTheme.typography.bodyMedium)
        for (point in DataPoint.entries) {
            // Hiding purchases hides prices and shops with them.
            val coveredByPurchases = point == DataPoint.PRICE_AND_SHOP && !current.shows(DataPoint.PURCHASES)
            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Column(modifier = Modifier.weight(1f).padding(end = 8.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(stringResource(point.title), style = MaterialTheme.typography.titleSmall)
                    Text(
                        stringResource(if (coveredByPurchases) R.string.track_covered_by_purchases else point.description),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
                Switch(
                    checked = current.shows(point),
                    enabled = !busy && !coveredByPurchases,
                    onCheckedChange = { on ->
                        if (on) change { repo.setTracked(point, true) } else turningOff = point
                    },
                )
            }
        }
        if (current.shows(DataPoint.STRAIN)) {
            SectionTitle(stringResource(R.string.strain_detail_title))
            Text(stringResource(R.string.strain_detail_intro), style = MaterialTheme.typography.bodySmall)
            Column(Modifier.selectableGroup()) {
                for (step in StrainDetail.entries) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .selectable(
                                selected = step == current.strainDetail,
                                enabled = !busy,
                                role = Role.RadioButton,
                                onClick = { change { repo.setStrainDetail(step) } },
                            )
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected = step == current.strainDetail, onClick = null)
                        Column(modifier = Modifier.padding(start = 8.dp)) {
                            Text(stringResource(step.title), style = MaterialTheme.typography.titleSmall)
                            Text(stringResource(step.description), style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
        }
        Text(stringResource(R.string.track_always), style = MaterialTheme.typography.bodySmall)
    }

    turningOff?.let { point ->
        val name = stringResource(point.title)
        AlertDialog(
            onDismissRequest = { turningOff = null },
            title = { Text(stringResource(R.string.track_off_title, name)) },
            text = { Text(stringResource(R.string.track_off_body, name)) },
            confirmButton = {
                TextButton(onClick = {
                    turningOff = null
                    change { repo.setTracked(point, false) }
                }) { Text(stringResource(R.string.track_keep_hidden)) }
            },
            dismissButton = {
                Row {
                    TextButton(onClick = { turningOff = null }) { Text(stringResource(R.string.cancel)) }
                    TextButton(onClick = {
                        turningOff = null
                        change {
                            repo.setTracked(point, false)
                            repo.deleteDataPoint(point)
                        }
                    }) { Text(stringResource(R.string.track_delete), color = MaterialTheme.colorScheme.error) }
                }
            },
        )
    }
}
