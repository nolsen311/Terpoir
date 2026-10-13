// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.fromHtml
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import app.terpoir.R

/**
 * The privacy policy, shown as text from the bundled copy of the website's
 * page. Nothing is fetched; links to other sites open in the browser only
 * when tapped.
 */
@Composable
fun PrivacyPolicyScreen(onClose: () -> Unit) {
    val context = LocalContext.current
    val blocks = remember {
        PrivacyPolicy.parse(context.assets.open(PrivacyPolicy.ASSET).bufferedReader().use { it.readText() })
    }
    val links = TextLinkStyles(SpanStyle(color = MaterialTheme.colorScheme.primary, textDecoration = TextDecoration.Underline))
    BackHandler(onBack = onClose)
    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        for (block in blocks) {
            val text = AnnotatedString.fromHtml(block.html, linkStyles = links)
            when (block.kind) {
                PolicyBlockKind.TITLE -> Text(text, style = MaterialTheme.typography.headlineMedium)
                PolicyBlockKind.HEADING -> Text(text, style = MaterialTheme.typography.titleLarge, modifier = Modifier.padding(top = 12.dp))
                PolicyBlockKind.SUBHEADING -> Text(text, style = MaterialTheme.typography.titleMedium, modifier = Modifier.padding(top = 4.dp))
                PolicyBlockKind.PARAGRAPH -> Text(text, style = MaterialTheme.typography.bodyMedium)
                PolicyBlockKind.ITEM -> Text(AnnotatedString("• ") + text, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(start = 8.dp))
            }
        }
        TextButton(onClick = onClose) { Text(stringResource(R.string.privacy_close)) }
    }
}
