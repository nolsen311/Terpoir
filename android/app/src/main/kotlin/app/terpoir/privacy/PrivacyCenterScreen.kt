// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import kotlinx.coroutines.launch

/**
 * The Privacy Center: every toggle with its card, a status summary, and
 * "Restore maximum privacy". Relaxing a protection first shows its
 * disclosure; returning to the private state needs no confirmation.
 *
 * @param onChanged called after any action ran, so the caller can apply
 *   window-level settings at once.
 * @param tour the onboarding tour: an introduction, and closing finishes
 *   setup.
 */
@Composable
fun PrivacyCenterScreen(
    center: PrivacyCenter,
    onChanged: () -> Unit,
    onClose: () -> Unit,
    tour: Boolean = false,
) {
    val scope = rememberCoroutineScope()
    // Toggle state lives in settings, not Compose; bumped after each action.
    var revision by remember { mutableIntStateOf(0) }
    var confirming by remember { mutableStateOf<PrivacyToggle?>(null) }
    fun run(action: suspend () -> Unit) {
        scope.launch {
            action()
            revision++
            onChanged()
        }
    }
    // During the tour, Back leaves the app as usual instead of skipping it.
    BackHandler(enabled = !tour, onBack = onClose)

    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Text(stringResource(R.string.privacy_center), style = MaterialTheme.typography.headlineMedium)
        if (tour) Text(stringResource(R.string.privacy_tour_intro))
        val relaxed = revision.let { center.relaxedCount() }
        Text(
            if (relaxed == 0) {
                stringResource(R.string.privacy_all_on)
            } else {
                pluralStringResource(R.plurals.privacy_relaxed, relaxed, relaxed)
            },
        )
        Button(onClick = { run { center.restoreMaximumPrivacy() } }) {
            Text(stringResource(R.string.privacy_restore))
        }
        for ((group, toggles) in center.groups) {
            Text(stringResource(group.title), style = MaterialTheme.typography.titleMedium)
            for (toggle in toggles) {
                val on = revision.let { toggle.isOn() }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text(stringResource(toggle.card.title))
                    Switch(
                        checked = on,
                        onCheckedChange = { wanted ->
                            if (wanted == toggle.privateState) {
                                run { toggle.set(wanted) }
                            } else {
                                confirming = toggle
                            }
                        },
                    )
                }
                Card(toggle.card)
            }
        }
        if (tour) {
            Button(onClick = onClose) { Text(stringResource(R.string.privacy_tour_finish)) }
        } else {
            TextButton(onClick = onClose) { Text(stringResource(R.string.privacy_close)) }
        }
    }

    confirming?.let { toggle ->
        AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text(stringResource(toggle.card.title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(toggle.card.whatLeaves))
                    Text(stringResource(toggle.card.whoCanSee))
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    confirming = null
                    run { toggle.set(!toggle.privateState) }
                }) { Text(stringResource(R.string.privacy_relax)) }
            },
            dismissButton = {
                TextButton(onClick = { confirming = null }) { Text(stringResource(R.string.privacy_keep)) }
            },
        )
    }
}

@Composable
private fun Card(card: PrivacyCard) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        for (line in listOf(card.whatItDoes, card.whatLeaves, card.whoCanSee, card.howToUndo)) {
            Text(stringResource(line), style = MaterialTheme.typography.bodySmall)
        }
    }
}

private val PrivacyGroup.title: Int
    get() = when (this) {
        PrivacyGroup.LOCATION -> R.string.privacy_group_location
        PrivacyGroup.NOTIFICATIONS -> R.string.privacy_group_notifications
        PrivacyGroup.CATALOGS -> R.string.privacy_group_catalogs
        PrivacyGroup.OS_INTEGRATION -> R.string.privacy_group_os_integration
        PrivacyGroup.PHOTOS -> R.string.privacy_group_photos
        PrivacyGroup.SYNC_AND_KEYS -> R.string.privacy_group_sync_and_keys
        PrivacyGroup.DEVICE_SECURITY -> R.string.privacy_group_device_security
    }
