// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.content.ActivityNotFoundException
import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.IdentifierData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData
import app.terpoir.core.classifyCode
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

private const val MS_PER_DAY = 86_400_000L

/** A day number (days since 1970-01-01) as a localized date. */
fun formatDay(day: Long): String =
    DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).format(LocalDate.ofEpochDay(day))

/** A date with a picker, or none. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DayField(title: String, day: Long?, onChange: (Long?) -> Unit) {
    var picking by remember { mutableStateOf(false) }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        OutlinedButton(onClick = { picking = true }) {
            Text(day?.let { "$title: ${formatDay(it)}" } ?: title)
        }
        if (day != null) TextButton(onClick = { onChange(null) }) { Text(stringResource(R.string.clear)) }
    }
    if (picking) {
        // The picker works in UTC midnights, which map to day numbers exactly.
        val state = rememberDatePickerState(initialSelectedDateMillis = day?.let { it * MS_PER_DAY })
        DatePickerDialog(
            onDismissRequest = { picking = false },
            confirmButton = {
                TextButton(onClick = {
                    state.selectedDateMillis?.let { onChange(Math.floorDiv(it, MS_PER_DAY)) }
                    picking = false
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { picking = false }) { Text(stringResource(R.string.cancel)) } },
        ) { DatePicker(state) }
    }
}

/**
 * Scanned identifiers. A web link (such as lab results) opens only when the
 * user turned on "Open lab-results links" in the Privacy Center.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun IdentifierList(identifiers: List<IdentifierData>, linksAllowed: Boolean, onRemove: (IdentifierData) -> Unit) {
    val context = LocalContext.current
    var failed by remember { mutableStateOf(false) }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            for (identifier in identifiers) {
                InputChip(
                    selected = false,
                    onClick = { onRemove(identifier) },
                    label = { Text("${label(identifier.kind)}: ${identifier.code.take(32)}") },
                    trailingIcon = {
                        Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.remove))
                    },
                )
            }
        }
        val links = identifiers.mapNotNull { classifyCode(it.code, it.kind != "barcode")?.link }
        if (links.isNotEmpty()) {
            if (linksAllowed) {
                for (link in links) {
                    OutlinedButton(onClick = {
                        try {
                            context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(link)))
                        } catch (_: ActivityNotFoundException) {
                            failed = true
                        }
                    }) { Text(stringResource(R.string.coa_open)) }
                }
                if (failed) Text(stringResource(R.string.coa_no_browser), color = MaterialTheme.colorScheme.error)
            } else {
                Text(stringResource(R.string.coa_off), style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}

/** Label terpenes (thousandths of a percent), each removable. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TerpeneList(terpenes: List<TagIntensity>, onRemove: (TagIntensity) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (terpene in terpenes) {
            val name = (terpene.tag as? TagRefData.Builtin)?.let { label(it.slug) } ?: stringResource(R.string.terpene_custom)
            InputChip(
                selected = false,
                onClick = { onRemove(terpene) },
                label = { Text("$name ${formatMilli(terpene.value)} %") },
                trailingIcon = {
                    Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.remove))
                },
            )
        }
    }
}
