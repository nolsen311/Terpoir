// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.ui.theme.AppearanceSettings
import app.terpoir.ui.theme.ThemeChoice

/**
 * Settings: the theme (this phone only; following the phone's setting
 * until the user picks Light or Dark), and the way into "What I track".
 */
@Composable
fun SettingsScreen(appearance: AppearanceSettings, onWhatITrack: () -> Unit, onClose: () -> Unit) {
    EditorScaffold(title = stringResource(R.string.settings_title), onClose = onClose, onSave = {}, saveEnabled = false) {
        SectionTitle(stringResource(R.string.theme_title))
        Text(stringResource(R.string.theme_intro), style = MaterialTheme.typography.bodySmall)
        Column(Modifier.selectableGroup()) {
            for (choice in ThemeChoice.entries) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .selectable(
                            selected = choice == appearance.theme,
                            role = Role.RadioButton,
                            onClick = { appearance.choose(choice) },
                        )
                        .padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(selected = choice == appearance.theme, onClick = null)
                    Text(stringResource(choice.title), style = MaterialTheme.typography.titleSmall, modifier = Modifier.padding(start = 8.dp))
                }
            }
        }
        SectionTitle(stringResource(R.string.track_title))
        Text(stringResource(R.string.settings_track_hint), style = MaterialTheme.typography.bodySmall)
        OutlinedButton(onClick = onWhatITrack) { Text(stringResource(R.string.track_title)) }
    }
}
