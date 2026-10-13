// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
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
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.map.GoogleMapsCard
import app.terpoir.privacy.SensitiveClipboard
import app.terpoir.session.JournalSession.RecoveryCheck
import kotlinx.coroutines.CancellationException

@Composable
private fun Page(content: @Composable () -> Unit) {
    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        horizontalAlignment = Alignment.Start,
    ) { content() }
}

/**
 * The age gate. The platform signal comes first, and a minor sees only
 * "Terpoir is for adults". A signal cannot pass the gate (its top range is
 * 18+, below many legal cannabis ages), so everyone else is asked. The
 * signal is read once and never stored; neither is a "no".
 */
@Composable
fun AgeGate(signals: AgeSignals, onAdult: () -> Unit) {
    val context = LocalContext.current
    var category by remember { mutableStateOf<AgeCategory?>(null) }
    var declined by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        category = try {
            signals.category(context)
        } catch (e: CancellationException) {
            throw e
        } catch (_: Exception) {
            AgeCategory.UNKNOWN
        }
    }
    Page {
        when {
            category == null -> Unit
            category == AgeCategory.MINOR || declined -> {
                Text(stringResource(R.string.age_adults_only), style = MaterialTheme.typography.headlineSmall)
                Text(stringResource(R.string.age_adults_only_body))
            }
            else -> {
                Text(stringResource(R.string.age_title), style = MaterialTheme.typography.headlineSmall)
                Text(stringResource(R.string.age_question))
                Button(onClick = onAdult) { Text(stringResource(R.string.age_yes)) }
                OutlinedButton(onClick = { declined = true }) { Text(stringResource(R.string.age_no)) }
                if (signals !== AgeSignals.NONE) {
                    Text(stringResource(R.string.age_signal_disclosure), style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
}

/** The recovery key, once: backup options, then confirm or skip. */
@Composable
fun RecoveryKeyBackup(
    recoveryKey: String,
    check: (String) -> RecoveryCheck,
    onResolved: (RecoveryBackup) -> Unit,
) {
    var confirming by remember { mutableStateOf(false) }
    if (confirming) {
        ConfirmRecoveryKey(check, onConfirmed = { onResolved(RecoveryBackup.CONFIRMED) }, onBack = { confirming = false })
    } else {
        ShowRecoveryKey(recoveryKey, onSaved = { confirming = true }, onSkip = { onResolved(RecoveryBackup.SKIPPED) })
    }
}

@Composable
private fun ShowRecoveryKey(recoveryKey: String, onSaved: () -> Unit, onSkip: () -> Unit) {
    val context = LocalContext.current
    var skipping by remember { mutableStateOf(false) }
    Page {
        Text(stringResource(R.string.recovery_title), style = MaterialTheme.typography.headlineSmall)
        Text(stringResource(R.string.recovery_explainer))
        Text(
            recoveryKey,
            fontFamily = FontFamily.Monospace,
            style = MaterialTheme.typography.titleMedium,
            modifier = Modifier.fillMaxWidth(),
        )
        Text(stringResource(R.string.recovery_paper))
        OutlinedButton(onClick = { SensitiveClipboard.copy(context, recoveryKey) }) {
            Text(stringResource(R.string.recovery_copy))
        }
        Text(stringResource(R.string.recovery_copy_disclosure), style = MaterialTheme.typography.bodySmall)
        Text(stringResource(R.string.recovery_never_ask), style = MaterialTheme.typography.bodySmall)
        Button(onClick = onSaved) { Text(stringResource(R.string.recovery_saved)) }
        TextButton(onClick = { skipping = true }) { Text(stringResource(R.string.recovery_skip)) }
    }
    if (skipping) {
        AlertDialog(
            onDismissRequest = { skipping = false },
            title = { Text(stringResource(R.string.recovery_skip_title)) },
            text = { Text(stringResource(R.string.recovery_skip_body)) },
            confirmButton = {
                TextButton(onClick = onSkip) { Text(stringResource(R.string.recovery_skip_confirm)) }
            },
            dismissButton = {
                TextButton(onClick = { skipping = false }) { Text(stringResource(R.string.recovery_skip_back)) }
            },
        )
    }
}

@Composable
private fun ConfirmRecoveryKey(check: (String) -> RecoveryCheck, onConfirmed: () -> Unit, onBack: () -> Unit) {
    var typed by remember { mutableStateOf("") }
    var result by remember { mutableStateOf<RecoveryCheck?>(null) }
    Page {
        Text(stringResource(R.string.confirm_title), style = MaterialTheme.typography.headlineSmall)
        Text(stringResource(R.string.confirm_explainer))
        OutlinedTextField(
            value = typed,
            onValueChange = {
                typed = it
                result = null
            },
            label = { Text(stringResource(R.string.confirm_label)) },
            textStyle = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace),
            // No suggestions or autocorrect for a secret; learning is off app-wide.
            keyboardOptions = KeyboardOptions(
                capitalization = KeyboardCapitalization.Characters,
                autoCorrectEnabled = false,
                keyboardType = KeyboardType.Password,
            ),
            modifier = Modifier.fillMaxWidth(),
        )
        result?.let { Text(stringResource(it.message), color = MaterialTheme.colorScheme.error) }
        Button(onClick = {
            val outcome = check(typed)
            if (outcome == RecoveryCheck.MATCHES) onConfirmed() else result = outcome
        }) { Text(stringResource(R.string.confirm_check)) }
        TextButton(onClick = onBack) { Text(stringResource(R.string.confirm_show_again)) }
    }
}

internal val RecoveryCheck.message: Int
    get() = when (this) {
        RecoveryCheck.MATCHES -> R.string.confirm_matches
        RecoveryCheck.INCOMPLETE -> R.string.confirm_incomplete
        RecoveryCheck.INVALID_CHARACTER -> R.string.confirm_invalid_character
        RecoveryCheck.TYPO -> R.string.confirm_typo
        RecoveryCheck.WRONG_KEY -> R.string.confirm_wrong_key
    }

/**
 * Play builds with a Google Maps key: whether to use Google Maps for the
 * online map, an active choice with neither answer preselected
 * (docs/design-decisions.md, "Map picker"). "Use Google Maps" turns the
 * map on with Google as its provider; "Not now" leaves the map off, with
 * OpenFreeMap ready if it is turned on later.
 */
@Composable
fun GoogleMapChoice(onUseGoogle: () -> Unit, onNotNow: () -> Unit) {
    Page {
        Text(stringResource(R.string.map_choice_title), style = MaterialTheme.typography.headlineSmall)
        Text(stringResource(R.string.map_choice_body))
        GoogleMapsCard()
        OutlinedButton(onClick = onUseGoogle) { Text(stringResource(R.string.google_maps_use)) }
        OutlinedButton(onClick = onNotNow) { Text(stringResource(R.string.map_choice_not_now)) }
    }
}

/**
 * Right after the age gate: a new journal, or one the user already has.
 * Both answers carry the same weight. There is nowhere to look a journal
 * up (no account, no server, and app data is left out of phone backups),
 * so the app asks.
 */
@Composable
fun JournalChoice(onNew: () -> Unit, onExisting: () -> Unit) {
    Text(stringResource(R.string.journal_choice_title), style = MaterialTheme.typography.headlineSmall)
    Text(stringResource(R.string.setup_explainer))
    OutlinedButton(onClick = onNew, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.journal_choice_new)) }
    OutlinedButton(onClick = onExisting, modifier = Modifier.fillMaxWidth()) { Text(stringResource(R.string.journal_choice_existing)) }
}

/**
 * Where the existing journal is: an export file or a sync folder, each
 * opened with the recovery key. Pairing with another phone isn't built
 * yet, so it is shown but can't be chosen.
 */
@Composable
fun ExistingJournal(onExport: () -> Unit, onFolder: () -> Unit, onBack: () -> Unit) {
    Text(stringResource(R.string.existing_title), style = MaterialTheme.typography.headlineSmall)
    Text(stringResource(R.string.existing_explainer))
    Source(R.string.existing_from_export, R.string.existing_from_export_hint, onClick = onExport)
    Source(R.string.existing_from_folder, R.string.existing_from_folder_hint, onClick = onFolder)
    Source(R.string.existing_from_phone, R.string.existing_from_phone_hint, onClick = null)
    TextButton(onClick = onBack) { Text(stringResource(R.string.back)) }
}

@Composable
private fun Source(title: Int, hint: Int, onClick: (() -> Unit)?) {
    Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        OutlinedButton(onClick = { onClick?.invoke() }, enabled = onClick != null, modifier = Modifier.fillMaxWidth()) {
            Text(stringResource(title))
        }
        Text(stringResource(hint), style = MaterialTheme.typography.bodySmall)
    }
}
