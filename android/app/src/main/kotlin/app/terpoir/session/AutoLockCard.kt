// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import app.terpoir.R

/**
 * When the journal locks after leaving the app (docs/design-decisions.md,
 * "Memory hygiene and app unlock"). Locking with the device is the default,
 * a recorded exception to "most private default"; the stricter timers are
 * opt-in, and the card says what the default allows. A setting, not a
 * privacy toggle: presets and "Restore maximum privacy" leave it alone.
 */
@Composable
fun AutoLockCard(settings: LockSettings) {
    var selected by remember { mutableStateOf(settings.timeout) }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(stringResource(R.string.app_lock), style = MaterialTheme.typography.titleMedium)
        Column(Modifier.selectableGroup()) {
            for (option in LockTimeout.entries) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .selectable(
                            selected = option == selected,
                            role = Role.RadioButton,
                            onClick = {
                                settings.timeout = option
                                selected = option
                            },
                        ),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(selected = option == selected, onClick = null)
                    Text(stringResource(option.label))
                }
            }
        }
        Text(stringResource(R.string.app_lock_screen_off), style = MaterialTheme.typography.bodySmall)
        if (selected == LockTimeout.WITH_DEVICE) {
            Text(stringResource(R.string.app_lock_with_device_disclosure), style = MaterialTheme.typography.bodySmall)
        }
    }
}

private val LockTimeout.label: Int
    get() = when (this) {
        LockTimeout.WITH_DEVICE -> R.string.app_lock_with_device
        LockTimeout.IMMEDIATELY -> R.string.app_lock_immediately
        LockTimeout.ONE_MINUTE -> R.string.app_lock_one_minute
        LockTimeout.FIVE_MINUTES -> R.string.app_lock_five_minutes
        LockTimeout.FIFTEEN_MINUTES -> R.string.app_lock_fifteen_minutes
    }
