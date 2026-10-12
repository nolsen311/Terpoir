// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.share

import android.content.ClipData
import android.content.Context
import android.content.Intent
import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.journal.EditorScaffold
import app.terpoir.journal.SectionTitle
import app.terpoir.privacy.SensitiveClipboard
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** A field the user can include, with its label and whether it starts included. */
class ShareOption(val id: String, val label: Int, val default: Boolean)

/** How the share is sent. */
private enum class ShareFormat { CARD, TEXT }

/**
 * The share screen (docs/design-decisions.md, "Share an entry"): the user
 * picks fields (private-leaning defaults), sees exactly what will be
 * shared, and sends it through the share sheet or copies the text. A card
 * is served from memory ([ShareProvider]) and dropped when this screen
 * closes; nothing is written to storage or the gallery.
 *
 * @param build the share for the chosen field IDs and mark.
 * @param extra more choices above the fields (the date, for sessions).
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ShareScreen(
    options: List<ShareOption>,
    onClose: () -> Unit,
    extra: @Composable () -> Unit = {},
    build: @Composable (chosen: Set<String>, mark: String?) -> ShareText,
) {
    val context = LocalContext.current
    var chosen by rememberSaveable { mutableStateOf(options.filter { it.default }.map { it.id }.toSet()) }
    var format by rememberSaveable { mutableStateOf(ShareFormat.CARD) }
    var withMark by rememberSaveable { mutableStateOf(true) }
    var markDropped by rememberSaveable { mutableStateOf(false) }
    val share = build(chosen, if (withMark) stringResource(R.string.share_mark) else null)
    var card by remember { mutableStateOf<Pair<ShareText, ByteArray>?>(null) }
    LaunchedEffect(share) {
        val png = withContext(Dispatchers.Default) { renderCard(share) }
        card = share to png
    }
    val preview: ImageBitmap? = remember(card) { card?.second?.let { BitmapFactory.decodeByteArray(it, 0, it.size)?.asImageBitmap() } }
    DisposableEffect(Unit) { onDispose { ShareProvider.clear() } }

    EditorScaffold(title = stringResource(R.string.share_title), onClose = onClose, onSave = {}, saveEnabled = false) {
        Text(stringResource(R.string.share_pick), style = MaterialTheme.typography.bodyMedium)
        extra()
        for (option in options) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(
                    checked = option.id in chosen,
                    onCheckedChange = { chosen = if (it) chosen + option.id else chosen - option.id },
                )
                Text(stringResource(option.label))
            }
        }

        SectionTitle(stringResource(R.string.share_preview))
        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilterChip(selected = format == ShareFormat.CARD, onClick = { format = ShareFormat.CARD }, label = { Text(stringResource(R.string.share_as_card)) })
            FilterChip(selected = format == ShareFormat.TEXT, onClick = { format = ShareFormat.TEXT }, label = { Text(stringResource(R.string.share_as_text)) })
        }
        when (format) {
            ShareFormat.CARD -> preview?.let {
                Image(it, contentDescription = stringResource(R.string.share_preview), contentScale = ContentScale.FillWidth, modifier = Modifier.fillMaxWidth())
            }
            ShareFormat.TEXT -> OutlinedCard(modifier = Modifier.fillMaxWidth()) {
                Text(share.plain(), modifier = Modifier.fillMaxWidth().padding(12.dp), style = MaterialTheme.typography.bodyMedium)
            }
        }

        // The mark is on by default; dropping it is one tap, with no nag.
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Switch(
                checked = withMark,
                onCheckedChange = {
                    withMark = it
                    markDropped = !it
                },
            )
            Text(stringResource(R.string.share_mark_toggle))
        }
        if (markDropped) Text(stringResource(R.string.share_mark_dropped), style = MaterialTheme.typography.bodySmall)

        Text(stringResource(R.string.share_outside), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                enabled = format == ShareFormat.TEXT || card?.first == share,
                onClick = {
                    when (format) {
                        ShareFormat.CARD -> card?.takeIf { it.first == share }?.let { sendCard(context, it.second) }
                        ShareFormat.TEXT -> sendText(context, share.plain())
                    }
                },
            ) { Text(stringResource(R.string.share_send)) }
            OutlinedButton(onClick = { SensitiveClipboard.copy(context, share.plain()) }) { Text(stringResource(R.string.share_copy)) }
        }
        Text(stringResource(R.string.share_copy_note), style = MaterialTheme.typography.bodySmall)
    }
}

private fun sendCard(context: Context, png: ByteArray) {
    val uri = ShareProvider.offer("${context.packageName}.share", png)
    val intent = Intent(Intent.ACTION_SEND).apply {
        type = "image/png"
        putExtra(Intent.EXTRA_STREAM, uri)
        clipData = ClipData.newRawUri(null, uri)
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
    }
    context.startActivity(Intent.createChooser(intent, null))
}

private fun sendText(context: Context, text: String) {
    val intent = Intent(Intent.ACTION_SEND).apply {
        type = "text/plain"
        putExtra(Intent.EXTRA_TEXT, text)
    }
    context.startActivity(Intent.createChooser(intent, null))
}
