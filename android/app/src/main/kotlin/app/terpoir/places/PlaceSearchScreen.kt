// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.places

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.PlaceData
import app.terpoir.journal.EditorScaffold
import app.terpoir.journal.SectionTitle
import java.io.IOException
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch

/**
 * Finds a spot's location by address or business name. A search goes out
 * only when the user taps Search (never while typing), to OpenStreetMap and
 * the phone's geocoder together; picking a result hands back its place.
 */
@Composable
fun PlaceSearchScreen(search: PlaceSearch, onPick: (PlaceData) -> Unit, onClose: () -> Unit) {
    val scope = rememberCoroutineScope()
    var query by remember { mutableStateOf("") }
    var searching by remember { mutableStateOf(false) }
    // Null until the first search.
    var osm by remember { mutableStateOf<List<PlaceData>?>(null) }
    var phone by remember { mutableStateOf<List<PlaceData>>(emptyList()) }
    var failed by remember { mutableStateOf(false) }

    fun go() {
        val text = query.trim()
        if (text.isEmpty() || searching) return
        searching = true
        scope.launch {
            coroutineScope {
                val fromOsm = async { runCatching { search.openStreetMap(text) } }
                val fromPhone = async { search.phone(text) }
                val got = fromOsm.await()
                failed = got.exceptionOrNull() is IOException
                osm = got.getOrDefault(emptyList())
                phone = fromPhone.await()
            }
            searching = false
        }
    }

    EditorScaffold(title = stringResource(R.string.place_search_title), onClose = onClose, onSave = null) {
        OutlinedTextField(
            value = query,
            onValueChange = { query = it },
            label = { Text(stringResource(R.string.place_search_hint)) },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { go() }),
            modifier = Modifier.fillMaxWidth(),
        )
        Button(onClick = ::go, enabled = query.isNotBlank() && !searching) { Text(stringResource(R.string.place_search_go)) }
        if (searching) Text(stringResource(R.string.place_search_searching))

        val found = osm
        if (found != null && !searching) {
            if (failed) Text(stringResource(R.string.place_search_failed), color = MaterialTheme.colorScheme.error)
            if (found.isEmpty() && phone.isEmpty() && !failed) Text(stringResource(R.string.place_search_none))
            if (found.isNotEmpty()) {
                SectionTitle(stringResource(R.string.place_search_osm))
                Places(found, onPick)
                Text(stringResource(R.string.place_search_attribution), style = MaterialTheme.typography.bodySmall)
            }
            if (phone.isNotEmpty()) {
                SectionTitle(stringResource(R.string.place_search_phone))
                Places(phone, onPick)
            }
        }
    }
}

@Composable
private fun Places(places: List<PlaceData>, onPick: (PlaceData) -> Unit) {
    Column {
        places.forEachIndexed { i, place ->
            if (i > 0) HorizontalDivider()
            Column(Modifier.fillMaxWidth().clickable { onPick(place) }.padding(vertical = 12.dp)) {
                Text(place.name, style = MaterialTheme.typography.titleMedium)
                if (place.detail.isNotEmpty()) Text(place.detail, style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}
