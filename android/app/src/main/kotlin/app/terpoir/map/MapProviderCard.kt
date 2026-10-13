// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
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
import app.terpoir.journal.JournalRepository
import app.terpoir.privacy.PrivacySettings
import kotlinx.coroutines.launch

/**
 * The online map's provider (docs/design-decisions.md, "Map picker"):
 * OpenFreeMap, with no key, or MapTiler with the user's own key. The key is
 * kept in the journal's encrypted local metadata, on this phone only; its
 * disclosure is shown before it can be saved. Removing it deletes it and
 * switches back to OpenFreeMap.
 */
@Composable
fun MapProviderCard(repo: JournalRepository, settings: PrivacySettings) {
    val scope = rememberCoroutineScope()
    var provider by remember { mutableStateOf(settings.mapProvider) }
    var saved by remember { mutableStateOf<Boolean?>(null) }
    var typed by remember { mutableStateOf("") }
    var invalid by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        val has = repo.mapTilerKey() != null
        // The flag follows the journal (a reset or restore loses the key).
        settings.mapTilerKeySaved = has
        if (!has && settings.mapProvider == MapProvider.MAPTILER) settings.mapProvider = MapProvider.DEFAULT
        provider = settings.mapProvider
        saved = has
    }
    val hasKey = saved ?: return

    fun choose(choice: MapProvider) {
        settings.mapProvider = choice
        provider = choice
    }

    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(stringResource(R.string.map_provider_title), style = MaterialTheme.typography.titleMedium)
        Column(Modifier.selectableGroup()) {
            ProviderRow(stringResource(R.string.map_provider_openfreemap), provider == MapProvider.OPENFREEMAP, enabled = true) {
                choose(MapProvider.OPENFREEMAP)
            }
            ProviderRow(stringResource(R.string.map_provider_maptiler), provider == MapProvider.MAPTILER, enabled = hasKey) {
                choose(MapProvider.MAPTILER)
            }
        }
        Text(stringResource(R.string.map_provider_note), style = MaterialTheme.typography.bodySmall)

        Text(stringResource(R.string.maptiler_key_title), style = MaterialTheme.typography.titleSmall)
        Text(stringResource(R.string.maptiler_key_leaves), style = MaterialTheme.typography.bodySmall)
        Text(stringResource(R.string.maptiler_key_who), style = MaterialTheme.typography.bodySmall)
        if (hasKey) {
            Text(stringResource(R.string.maptiler_key_saved), style = MaterialTheme.typography.bodyMedium)
            OutlinedButton(onClick = {
                scope.launch {
                    repo.setMapTilerKey(null)
                    settings.mapTilerKeySaved = false
                    choose(MapProvider.DEFAULT)
                    saved = false
                }
            }) { Text(stringResource(R.string.maptiler_key_remove)) }
        } else {
            OutlinedTextField(
                value = typed,
                onValueChange = {
                    typed = it.trim()
                    invalid = false
                },
                label = { Text(stringResource(R.string.maptiler_key_field)) },
                singleLine = true,
                isError = invalid,
                supportingText = if (invalid) {
                    { Text(stringResource(R.string.maptiler_key_invalid)) }
                } else {
                    null
                },
                modifier = Modifier.fillMaxWidth(),
            )
            OutlinedButton(
                enabled = typed.isNotEmpty(),
                onClick = {
                    scope.launch {
                        val ok = runCatching { repo.setMapTilerKey(typed) }.isSuccess
                        if (ok) {
                            settings.mapTilerKeySaved = true
                            choose(MapProvider.MAPTILER)
                            typed = ""
                            saved = true
                        } else {
                            invalid = true
                        }
                    }
                },
            ) { Text(stringResource(R.string.maptiler_key_save)) }
        }
    }
}

@Composable
private fun ProviderRow(label: String, selected: Boolean, enabled: Boolean, onClick: () -> Unit) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .selectable(selected = selected, enabled = enabled, role = Role.RadioButton, onClick = onClick),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RadioButton(selected = selected, onClick = null, enabled = enabled)
        Text(label, style = MaterialTheme.typography.bodyLarge)
    }
}
