// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.net.Uri
import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
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
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.StrainData
import app.terpoir.core.StrainPageData
import app.terpoir.core.TagRefData
import app.terpoir.core.TerpeneLevelData
import app.terpoir.core.parseStrainPage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * A strain's web page the user shared to Terpoir: selected text, or a
 * screenshot read on the phone (docs/design-decisions.md, "Sharing back
 * what you found"). Held in memory only.
 */
data class SharedPage(val text: String?, val image: Uri?) {
    companion object {
        /** The page in an `ACTION_SEND` intent, or null when it carries neither text nor an image. */
        fun from(intent: Intent): SharedPage? {
            val text = intent.getStringExtra(Intent.EXTRA_TEXT)?.takeIf { it.isNotBlank() }
            val image = if (intent.type?.startsWith("image/") == true) {
                if (Build.VERSION.SDK_INT >= 33) {
                    intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(Intent.EXTRA_STREAM)
                }
            } else {
                null
            }
            return if (text == null && image == null) null else SharedPage(text, image)
        }
    }
}

/**
 * The strain the user last opened "Look up on the web" for, in this run of
 * the app only (never written anywhere): a page shared back afterwards is
 * most likely about it.
 */
object LastLookup {
    var strainId: String? = null
    var name: String? = null
}

/** The longest side a shared screenshot is read at. */
private const val MAX_SIDE = 4096

/** Decodes a shared screenshot, scaled down to [MAX_SIDE]; null if it can't be read. */
private suspend fun decode(context: Context, uri: Uri): Bitmap? = withContext(Dispatchers.IO) {
    runCatching {
        val source = ImageDecoder.createSource(context.contentResolver, uri)
        ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
            decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            val side = maxOf(info.size.width, info.size.height)
            if (side > MAX_SIDE) {
                val scale = MAX_SIDE.toFloat() / side
                decoder.setTargetSize((info.size.width * scale).toInt(), (info.size.height * scale).toInt())
            }
        }
    }.getOrNull()
}

/**
 * Reads a shared page and offers what it found for one strain: the last
 * strain looked up, another one, or a new one. Each suggestion has a
 * checkbox; one that would replace something already saved starts
 * unchecked. Nothing is saved until Save, and a screenshot is recycled as
 * soon as it has been read.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SharedPageScreen(repo: JournalRepository, scan: ScanServices, page: SharedPage, onOpenStrain: (String) -> Unit, onDone: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val tracking = LocalTracking.current
    var read by remember { mutableStateOf<StrainPageData?>(null) }
    var failed by remember { mutableStateOf(false) }
    var strains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var strainId by remember { mutableStateOf(LastLookup.strainId) }
    var newName by remember { mutableStateOf(if (LastLookup.strainId == null) LastLookup.name.orEmpty() else "") }
    var current by remember { mutableStateOf<StrainData?>(null) }
    var useType by remember { mutableStateOf(true) }
    var useBreeder by remember { mutableStateOf(true) }
    var useParents by remember { mutableStateOf(true) }
    var useTerpenes by remember { mutableStateOf(true) }

    suspend fun readText(): String? {
        page.text?.let { return it }
        val uri = page.image ?: return null
        val bitmap = decode(context, uri) ?: return null
        return try {
            scan.scanner.read(bitmap).text
        } finally {
            bitmap.recycle()
        }
    }

    // The page as text, read on the phone, then the suggestions.
    val body: @androidx.compose.runtime.Composable () -> Unit = {
        LaunchedEffect(Unit) {
            strains = repo.strains()
            if (strainId != null && strains.none { it.id == strainId }) strainId = null
            val text = readText()
            if (text == null) failed = true else read = parseStrainPage(text)
        }
        LaunchedEffect(strainId) {
            current = strainId?.let { repo.strain(it) }
            // A suggestion that would replace something saved starts unchecked.
            useType = current?.strainType == null
            useBreeder = current?.breeder.isNullOrBlank()
            useParents = current?.lineage.isNullOrBlank()
        }
        val found = read
        val step = tracking.strainDetail
        val showTerpenes = tracking.shows(DataPoint.TERPENES) && step != StrainDetail.SIMPLE
        val showLineage = step != StrainDetail.SIMPLE
        EditorScaffold(
            title = stringResource(R.string.shared_page_title),
            onClose = onDone,
            saveEnabled = found != null && (strainId != null || newName.isNotBlank()),
            onSave = {
                val page = found ?: return@EditorScaffold
                scope.launch {
                    val base = current ?: StrainData(newName.trim(), null, emptyList(), null, emptyList(), emptyList(), emptyList())
                    val terpenes = if (showTerpenes && useTerpenes) {
                        (base.terpenes + page.terpenes.map { TagRefData.Builtin(it) }).distinct()
                    } else {
                        base.terpenes
                    }
                    val dominant = page.dominant?.let { TagRefData.Builtin(it) }
                    val levels = if (showTerpenes && useTerpenes && step == StrainDetail.DETAILED && dominant != null &&
                        base.terpeneLevels.none { it.level == "dominant" }
                    ) {
                        base.terpeneLevels.filter { it.terpene != dominant } + TerpeneLevelData(dominant, "dominant")
                    } else {
                        base.terpeneLevels
                    }
                    val data = base.copy(
                        strainType = if (useType && page.strainType != null) page.strainType else base.strainType,
                        breeder = if (showLineage && useBreeder && page.breeder != null) page.breeder else base.breeder,
                        lineage = if (showLineage && useParents && page.parents.isNotEmpty()) page.parents.joinToString(" × ") else base.lineage,
                        terpenes = terpenes,
                        terpeneLevels = levels,
                    )
                    val id = repo.saveStrain(strainId, data)
                    onOpenStrain(id)
                }
            },
        ) {
            when {
                failed -> Text(stringResource(R.string.shared_page_unreadable))
                found == null -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    CircularProgressIndicator()
                    Text(stringResource(R.string.shared_page_reading))
                }
                else -> {
                    SectionTitle(stringResource(R.string.shared_page_strain))
                    Dropdown(
                        label = stringResource(R.string.shared_page_strain),
                        options = listOf<Pair<String?, String>>(null to stringResource(R.string.shared_page_new_strain)) + strains.map { it.id to it.data.name },
                        selected = strainId,
                        onSelect = { strainId = it },
                    )
                    if (strainId == null) {
                        OutlinedTextField(newName, { newName = it }, label = { Text(stringResource(R.string.strain_name)) }, singleLine = true, modifier = Modifier.fillMaxWidth())
                    }
                    val nothing = found.strainType == null && found.terpenes.isEmpty() &&
                        (!showLineage || (found.breeder == null && found.parents.isEmpty()))
                    if (nothing) {
                        Text(stringResource(R.string.shared_page_nothing))
                    } else {
                        SectionTitle(stringResource(R.string.shared_page_found))
                        found.strainType?.let { t ->
                            Choice(useType, { useType = it }, stringResource(R.string.strain_type) + ": " + label(t))
                        }
                        if (showLineage) {
                            found.breeder?.let { b ->
                                Choice(useBreeder, { useBreeder = it }, stringResource(R.string.strain_breeder) + ": " + b)
                            }
                            if (found.parents.isNotEmpty()) {
                                Choice(useParents, { useParents = it }, stringResource(R.string.strain_lineage) + ": " + found.parents.joinToString(" × "))
                            }
                        }
                        if (showTerpenes && found.terpenes.isNotEmpty()) {
                            Choice(useTerpenes, { useTerpenes = it }, stringResource(R.string.strain_terpenes))
                            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                for (t in found.terpenes) {
                                    val name = label(t)
                                    Text(if (t == found.dominant) stringResource(R.string.shared_page_dominant, name) else name, style = MaterialTheme.typography.bodyMedium)
                                }
                            }
                        }
                    }
                    Text(stringResource(R.string.shared_page_note), style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
    if (page.image != null && page.text == null) {
        TextEngineGate(scan.scanner, stringResource(R.string.shared_page_title), onDone) { body() }
    } else {
        body()
    }
}

@Composable
private fun Choice(checked: Boolean, onChange: (Boolean) -> Unit, text: String) {
    Row(verticalAlignment = Alignment.CenterVertically) {
        Checkbox(checked, onChange)
        Text(text)
    }
}
