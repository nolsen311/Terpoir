// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.Manifest
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.FilterChip
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.SpotData
import app.terpoir.core.TagData
import app.terpoir.core.TagRefData
import app.terpoir.location.CurrentLocation
import app.terpoir.location.Fix
import kotlinx.coroutines.launch

/** Built-in spot purposes, in display order. */
val SPOT_PURPOSES = listOf("blaze", "munch", "re_up", "chill")

private const val SPOT_PURPOSE_CATEGORY = "spot_purpose"

/** "Spot N": the first number no saved "Spot N" uses (no place names, ever). */
fun nextSpotName(existing: List<String>, prefix: String): String {
    val used = existing.mapNotNull { name -> name.takeIf { it.startsWith("$prefix ") }?.removePrefix("$prefix ")?.toIntOrNull() }.toSet()
    return "$prefix ${generateSequence(1) { it + 1 }.first { it !in used }}"
}

/**
 * Creates or edits a spot: a name the user picks (a neutral "Spot N" by
 * default; never looked up from an address), what it's for, and optional
 * coordinates, approximate unless the user asks for precise. Fields not
 * shown here (geofence radius, purchase prompts) are kept as saved.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SpotEditor(repo: JournalRepository, id: String?, onDone: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var original by remember { mutableStateOf<SpotData?>(null) }
    var loaded by remember { mutableStateOf(false) }
    var name by remember { mutableStateOf("") }
    var purposes by remember { mutableStateOf<List<TagRefData>>(emptyList()) }
    var fix by remember { mutableStateOf<Fix?>(null) }
    var customPurposes by remember { mutableStateOf<List<Entry<TagData>>>(emptyList()) }
    var locating by remember { mutableStateOf(false) }
    var locationProblem by remember { mutableStateOf<Int?>(null) }
    var newPurpose by remember { mutableStateOf(false) }
    val defaultPrefix = stringResource(R.string.spot_default_prefix)

    fun locate() {
        locating = true
        locationProblem = null
        scope.launch {
            val got = CurrentLocation.get(context)
            locating = false
            if (got == null) locationProblem = R.string.spot_location_failed else fix = got
        }
    }
    val askLocation = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { result ->
        if (result.values.any { it }) locate() else locationProblem = R.string.spot_location_denied
    }

    LaunchedEffect(id) {
        customPurposes = repo.tags(SPOT_PURPOSE_CATEGORY)
        val spot = id?.let { repo.spot(it) }
        if (spot != null) {
            original = spot
            name = spot.name
            purposes = spot.purposes
            val lat = spot.latE7
            val lon = spot.lonE7
            val accuracy = spot.accuracyM
            if (lat != null && lon != null && accuracy != null) fix = Fix(lat, lon, accuracy.toInt())
        } else {
            name = nextSpotName(repo.spots().map { it.data.name }, defaultPrefix)
        }
        loaded = true
    }
    if (!loaded) return

    EditorScaffold(
        title = stringResource(if (id == null) R.string.spot_new else R.string.spot_edit),
        onClose = onDone,
        saveEnabled = name.isNotBlank() && !locating,
        onSave = {
            val f = fix
            val data = SpotData(
                name = name.trim(),
                purposes = purposes,
                latE7 = f?.latE7,
                lonE7 = f?.lonE7,
                accuracyM = f?.accuracyM?.toUInt(),
                radiusM = original?.radiusM,
                purchasePrompts = original?.purchasePrompts ?: false,
            )
            scope.launch {
                repo.saveSpot(id, data)
                onDone()
            }
        },
    ) {
        OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.spot_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
        Text(stringResource(R.string.spot_name_note), style = MaterialTheme.typography.bodySmall)

        SectionTitle(stringResource(R.string.spot_purposes))
        val customNames = customPurposes.associate { it.id to it.data.name }
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            val all = SPOT_PURPOSES.map { TagRefData.Builtin(it) } + customPurposes.map { TagRefData.Custom(it.id) }
            for (purpose in all) {
                val selected = purpose in purposes
                FilterChip(
                    selected = selected,
                    onClick = { purposes = if (selected) purposes - purpose else purposes + purpose },
                    label = { Text(tagLabel(purpose, customNames)) },
                )
            }
        }
        OutlinedButton(onClick = { newPurpose = true }) { Text(stringResource(R.string.spot_new_purpose)) }

        SectionTitle(stringResource(R.string.spot_location))
        val current = fix
        if (current == null) {
            Text(stringResource(R.string.spot_location_none), style = MaterialTheme.typography.bodyMedium)
        } else {
            Text(stringResource(R.string.spot_location_saved, current.accuracyM), style = MaterialTheme.typography.bodyMedium)
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(
                enabled = !locating,
                onClick = { askLocation.launch(arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION)) },
            ) { Text(stringResource(R.string.spot_location_approximate)) }
            OutlinedButton(
                enabled = !locating,
                onClick = {
                    askLocation.launch(arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION))
                },
            ) { Text(stringResource(R.string.spot_location_precise)) }
        }
        if (current != null) TextButton(onClick = { fix = null }) { Text(stringResource(R.string.spot_location_clear)) }
        if (locating) Text(stringResource(R.string.spot_locating))
        locationProblem?.let { Text(stringResource(it), color = MaterialTheme.colorScheme.error) }
        Text(stringResource(R.string.spot_location_note), style = MaterialTheme.typography.bodySmall)

        if (id != null) DeleteButton(R.string.spot_delete, R.string.spot_delete_body) { scope.launch { repo.delete(id); onDone() } }
    }

    if (newPurpose) {
        NameDialog(
            title = stringResource(R.string.spot_new_purpose),
            onDismiss = { newPurpose = false },
            onConfirm = { text ->
                newPurpose = false
                scope.launch {
                    val tag = repo.saveTag(null, TagData(text, SPOT_PURPOSE_CATEGORY))
                    customPurposes = repo.tags(SPOT_PURPOSE_CATEGORY)
                    purposes = purposes + TagRefData.Custom(tag)
                }
            },
        )
    }
}
