// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.LookupSiteData
import app.terpoir.core.lookupAddress
import app.terpoir.core.lookupTemplateFromExample
import app.terpoir.core.lookupTemplateValid
import kotlinx.coroutines.launch

/**
 * "Look up on the web" for a strain: pick a saved site and the browser
 * opens it searching for [strain]. The app fetches nothing itself; the
 * sites live in the journal (docs/design-decisions.md, "Catalog lookups").
 */
@Composable
fun WebLookup(repo: JournalRepository, strain: String, strainId: String?) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sites by remember { mutableStateOf<List<LookupSiteData>>(emptyList()) }
    var menu by remember { mutableStateOf(false) }
    var managing by remember { mutableStateOf(false) }
    var noBrowser by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { sites = repo.lookupSites() }

    Box {
        OutlinedButton(onClick = { if (sites.isEmpty()) managing = true else menu = true }) {
            Text(stringResource(R.string.web_lookups))
        }
        DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
            for (site in sites) {
                DropdownMenuItem(
                    text = { Text(site.name) },
                    onClick = {
                        menu = false
                        val address = lookupAddress(site.template, strain) ?: return@DropdownMenuItem
                        // A page shared back afterwards is most likely about this strain.
                        LastLookup.strainId = strainId
                        LastLookup.name = strain
                        noBrowser = try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(address)))
                            false
                        } catch (_: ActivityNotFoundException) {
                            true
                        }
                    },
                )
            }
            DropdownMenuItem(
                text = { Text(stringResource(R.string.web_lookup_manage)) },
                onClick = {
                    menu = false
                    managing = true
                },
            )
        }
    }
    if (noBrowser) Text(stringResource(R.string.coa_no_browser), color = MaterialTheme.colorScheme.error)
    if (managing) {
        LookupSitesDialog(
            sites = sites,
            example = strain,
            onSave = { id, name, template ->
                scope.launch {
                    repo.saveLookupSite(id, name, template)
                    sites = repo.lookupSites()
                }
            },
            onDelete = { id ->
                scope.launch {
                    repo.deleteLookupSite(id)
                    sites = repo.lookupSites()
                }
            },
            onClose = { managing = false },
        )
    }
}

/**
 * The saved sites, each with a delete, and a form to add one: its name and
 * the address of a search the user did there for any strain (the core
 * turns that into a template), or a template written with `{strain}`.
 */
@Composable
private fun LookupSitesDialog(
    sites: List<LookupSiteData>,
    example: String,
    onSave: (String?, String, String) -> Unit,
    onDelete: (String) -> Unit,
    onClose: () -> Unit,
) {
    var name by remember { mutableStateOf("") }
    var address by remember { mutableStateOf("") }
    var searched by remember { mutableStateOf(example) }
    var invalid by remember { mutableStateOf(false) }
    AlertDialog(
        onDismissRequest = onClose,
        title = { Text(stringResource(R.string.web_lookup_sites)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                for (site in sites) {
                    Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.fillMaxWidth()) {
                        Text(site.name, modifier = Modifier.weight(1f))
                        IconButton(onClick = { onDelete(site.id) }) {
                            Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.web_lookup_delete, site.name))
                        }
                    }
                }
                Text(stringResource(R.string.web_lookup_add_how), style = MaterialTheme.typography.bodySmall)
                OutlinedTextField(name, { name = it }, label = { Text(stringResource(R.string.web_lookup_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(
                    address,
                    {
                        address = it
                        invalid = false
                    },
                    label = { Text(stringResource(R.string.web_lookup_address)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(searched, { searched = it }, label = { Text(stringResource(R.string.web_lookup_searched)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
                if (invalid) Text(stringResource(R.string.web_lookup_invalid), color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
            }
        },
        confirmButton = {
            TextButton(
                enabled = name.isNotBlank() && address.isNotBlank(),
                onClick = {
                    // A template as typed, else learned from the search address.
                    val template = address.trim().takeIf { lookupTemplateValid(it) }
                        ?: lookupTemplateFromExample(address, searched)
                    if (template == null) {
                        invalid = true
                    } else {
                        onSave(null, name, template)
                        name = ""
                        address = ""
                    }
                },
            ) { Text(stringResource(R.string.web_lookup_add)) }
        },
        dismissButton = { TextButton(onClick = onClose) { Text(stringResource(R.string.close)) } },
    )
}
