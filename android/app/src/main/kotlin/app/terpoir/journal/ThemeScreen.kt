// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.ui.theme.AppearanceSettings
import app.terpoir.ui.theme.CardTones
import app.terpoir.ui.theme.ThemeChoice

/**
 * The theme, from Settings: the phone's setting (the default), Light or
 * Dark, kept on this phone only. A preview of the app's usual controls sits
 * above the choices, so a change shows at once; it is there for the themes
 * that may come later too.
 */
@Composable
fun ThemeScreen(appearance: AppearanceSettings, onClose: () -> Unit) {
    EditorScaffold(title = stringResource(R.string.theme_title), onClose = onClose, onSave = null) {
        ThemePreview()
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
    }
}

/**
 * Sample controls in the current theme: the page, a session card, a chip,
 * a switch and both kinds of button. They look live but do nothing, and
 * screen readers hear one "Preview" instead of them.
 */
@Composable
private fun ThemePreview() {
    val description = stringResource(R.string.theme_preview)
    Surface(
        color = MaterialTheme.colorScheme.background,
        shape = MaterialTheme.shapes.large,
        border = BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant),
        modifier = Modifier.fillMaxWidth().clearAndSetSemantics { contentDescription = description },
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Card(
                colors = CardDefaults.cardColors(containerColor = CardTones.raised),
                border = CardTones.border,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(
                            stringResource(R.string.theme_sample_when),
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            modifier = Modifier.weight(1f),
                        )
                        Text("★★★★☆", color = MaterialTheme.colorScheme.tertiary)
                    }
                    Text(stringResource(R.string.theme_sample_title), style = MaterialTheme.typography.titleLarge)
                    Text(stringResource(R.string.theme_sample_detail), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    AssistChip(onClick = {}, label = { Text(stringResource(R.string.theme_sample_effect)) })
                }
            }
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.theme_sample_switch), style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                Switch(checked = true, onCheckedChange = null)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = {}) { Text(stringResource(R.string.theme_sample_save)) }
                OutlinedButton(onClick = {}) { Text(stringResource(R.string.theme_sample_cancel)) }
            }
        }
    }
}
