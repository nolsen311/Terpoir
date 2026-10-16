// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.capture

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.privacy.PrivacySettings

/**
 * Shows or hides the "New note" and "New entry" app shortcuts. A setting, not a privacy
 * toggle: they are on by default and reveal nothing the app icon
 * doesn't (docs/design-decisions.md, "Quick capture").
 */
@Composable
fun ShortcutCard(settings: PrivacySettings) {
    val context = LocalContext.current
    var shown by remember { mutableStateOf(settings.newNoteShortcut) }
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(stringResource(R.string.capture_shortcut_title), style = MaterialTheme.typography.titleMedium)
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(stringResource(R.string.capture_shortcut_switch), modifier = Modifier.weight(1f).padding(end = 8.dp))
            Switch(
                checked = shown,
                onCheckedChange = { wanted ->
                    settings.newNoteShortcut = wanted
                    QuickCapture.setShortcut(context, wanted)
                    shown = wanted
                },
            )
        }
        Text(stringResource(R.string.capture_shortcut_note), style = MaterialTheme.typography.bodySmall)
    }
}
