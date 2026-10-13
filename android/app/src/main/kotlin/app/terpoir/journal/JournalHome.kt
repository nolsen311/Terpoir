// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.Manifest
import android.annotation.SuppressLint
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.alerts.AlertNeed
import app.terpoir.alerts.CheckIns
import app.terpoir.alerts.SpotAlerts
import app.terpoir.core.BrandData
import app.terpoir.core.NearbySpot
import app.terpoir.core.ProductData
import app.terpoir.core.SessionData
import app.terpoir.core.SnippetData
import app.terpoir.core.SpotData
import app.terpoir.core.StrainData
import app.terpoir.core.TagRefData
import app.terpoir.core.Vocabulary
import app.terpoir.location.CurrentLocation
import app.terpoir.location.LocationAccess
import app.terpoir.privacy.PrivacySettings
import app.terpoir.privacy.PrivacyToggle
import kotlinx.coroutines.launch

/** Where the user is inside the unlocked app. */
private sealed interface Screen {
    data object Home : Screen

    /** [spot]: a new session at that spot ("Log a session here"). */
    data class Session(val id: String?, val spot: String? = null) : Screen

    data class Product(val id: String?) : Screen

    data class Strain(val id: String?) : Screen

    data class Brand(val id: String?) : Screen

    data object Receipt : Screen

    data class Note(val id: String?) : Screen

    data class Spot(val id: String?) : Screen

    /** The follow-up check-in on a session. */
    data class CheckIn(val session: String) : Screen

    data object Insights : Screen

    data object WhatITrack : Screen
}

private enum class Tab { JOURNAL, PRODUCTS, PRIVACY }

private enum class Catalog { PRODUCTS, STRAINS, BRANDS, SPOTS }

/**
 * The unlocked app: journal, products and the Privacy Center, plus the
 * editors. Navigation is a small in-memory stack; the system Back pops it.
 */
@Composable
fun JournalHome(
    repo: JournalRepository,
    vocab: Vocabulary,
    scan: ScanServices,
    settings: PrivacySettings,
    alerts: SpotAlerts,
    checkIns: CheckIns,
    mapToggle: PrivacyToggle?,
    checkNearby: Boolean,
    checkInSession: String?,
    onCheckInOpened: () -> Unit,
    quickLog: Boolean,
    onQuickLogOpened: () -> Unit,
    newNote: Boolean,
    onNewNoteOpened: () -> Unit,
    onEdited: () -> Unit,
    onLock: () -> Unit,
    privacyCenter: @Composable (onClose: () -> Unit) -> Unit,
) {
    val stack = remember { mutableStateListOf<Screen>(Screen.Home) }
    var tab by remember { mutableStateOf(Tab.JOURNAL) }
    // Bumped whenever an editor closes, so lists reload.
    var revision by remember { mutableIntStateOf(0) }
    val pop: () -> Unit = {
        if (stack.size > 1) stack.removeAt(stack.lastIndex)
        revision++
    }
    // "What I track", for every screen below (LocalTracking); reloaded when
    // an editor or the What I track screen closes.
    var tracking by remember { mutableStateOf(Tracking.EVERYTHING) }
    LaunchedEffect(revision) { tracking = repo.tracking() }
    // Opened from a "Quick log?" purchase prompt: receipt scan first.
    LaunchedEffect(quickLog) {
        if (quickLog) {
            // Purchases hidden: there is nothing to log.
            if (repo.tracking().shows(DataPoint.PURCHASES)) stack.add(Screen.Receipt)
            onQuickLogOpened()
        }
    }
    // Opened from a quick-capture shortcut, tile or widget: a new note, in
    // the user's default mode (the note screen applies voice-first).
    LaunchedEffect(newNote) {
        if (newNote) {
            stack.add(Screen.Note(null))
            onNewNoteOpened()
        }
    }
    // Opened from a "Quick check-in?" reminder.
    LaunchedEffect(checkInSession) {
        if (checkInSession != null) {
            stack.add(Screen.CheckIn(checkInSession))
            onCheckInOpened()
        }
    }
    // Spots may have changed: "Notify me" re-registers its geofences.
    LaunchedEffect(revision) { if (revision > 0) onEdited() }

    CompositionLocalProvider(LocalTracking provides tracking) {
        when (val screen = stack.last()) {
            is Screen.Session -> SessionEditor(repo, vocab, settings, checkIns, screen.id, pop, spotForNew = screen.spot)
            is Screen.CheckIn -> CheckInScreen(repo, vocab, screen.session, pop)
            is Screen.Spot -> SpotEditor(repo, screen.id, mapToggle, settings, settings.purchasePrompts, pop)
            is Screen.Product -> ProductEditor(
                repo,
                vocab,
                scan,
                screen.id,
                onOpenProduct = { other -> stack[stack.lastIndex] = Screen.Product(other) },
                onDone = pop,
            )
            Screen.Receipt -> ReceiptScreen(repo, scan, pop)
            Screen.Insights -> InsightsScreen(repo, onOpenSession = { stack.add(Screen.Session(it)) }, onClose = pop)
            Screen.WhatITrack -> WhatITrackScreen(repo, onChanged = { revision++ }, onClose = pop)
            is Screen.Note -> NoteScreen(repo, settings, vocab, screen.id, pop)
            is Screen.Strain -> StrainEditor(repo, vocab, screen.id, pop)
            is Screen.Brand -> BrandEditor(repo, screen.id, pop)
            Screen.Home -> Scaffold(
                bottomBar = {
                    NavigationBar {
                        NavItem(tab == Tab.JOURNAL, R.drawable.ic_journal, R.string.tab_journal) { tab = Tab.JOURNAL }
                        NavItem(tab == Tab.PRODUCTS, R.drawable.ic_products, R.string.tab_products) { tab = Tab.PRODUCTS }
                        NavItem(tab == Tab.PRIVACY, R.drawable.ic_privacy, R.string.tab_privacy) { tab = Tab.PRIVACY }
                    }
                },
                floatingActionButton = {
                    if (tab == Tab.JOURNAL) {
                        Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                            SmallFloatingActionButton(onClick = { stack.add(Screen.Session(null)) }) {
                                Icon(painterResource(R.drawable.ic_add), contentDescription = stringResource(R.string.session_new))
                            }
                            // The new-note button: voice first unless the user chose typing.
                            ExtendedFloatingActionButton(
                                onClick = { stack.add(Screen.Note(null)) },
                                icon = { Icon(painterResource(R.drawable.ic_mic), contentDescription = null) },
                                text = { Text(stringResource(R.string.note_new)) },
                            )
                        }
                    }
                },
            ) { padding ->
                when (tab) {
                    Tab.JOURNAL -> JournalList(
                        repo,
                        settings,
                        alerts,
                        checkNearby,
                        onEdited,
                        revision,
                        padding,
                        onLock,
                        onOpenNote = { stack.add(Screen.Note(it)) },
                        onSessionAt = { stack.add(Screen.Session(null, spot = it)) },
                        onInsights = { stack.add(Screen.Insights) },
                        onWhatITrack = { stack.add(Screen.WhatITrack) },
                        onPurchase = { stack.add(Screen.Receipt) },
                    ) {
                        stack.add(Screen.Session(it))
                    }
                    Tab.PRODUCTS -> CatalogLists(repo, vocab, revision, padding) { stack.add(it) }
                    Tab.PRIVACY -> Column(Modifier.padding(padding)) { privacyCenter { tab = Tab.JOURNAL } }
                }
            }
        }
    }
}

@Composable
private fun androidx.compose.foundation.layout.RowScope.NavItem(selected: Boolean, icon: Int, label: Int, onClick: () -> Unit) {
    NavigationBarItem(
        selected = selected,
        onClick = onClick,
        icon = { Icon(painterResource(icon), contentDescription = null) },
        label = { Text(stringResource(label)) },
    )
}

@Composable
private fun JournalList(
    repo: JournalRepository,
    settings: PrivacySettings,
    alerts: SpotAlerts,
    checkNearby: Boolean,
    onPermissions: () -> Unit,
    revision: Int,
    padding: PaddingValues,
    onLock: () -> Unit,
    onOpenNote: (String) -> Unit,
    onSessionAt: (String) -> Unit,
    onInsights: () -> Unit,
    onWhatITrack: () -> Unit,
    onPurchase: () -> Unit,
    onOpen: (String) -> Unit,
) {
    var sessions by remember { mutableStateOf<List<Entry<SessionData>>?>(null) }
    var notes by remember { mutableStateOf<List<Entry<SnippetData>>>(emptyList()) }
    var productNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    var effectNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    // Purchases hidden ("What I track"): no "log a purchase" here.
    val purchases = LocalTracking.current.shows(DataPoint.PURCHASES)
    var tagNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    // Bumped when a note added while locked is kept or discarded.
    var reviewed by remember { mutableIntStateOf(0) }
    // Records from a newer app version, kept until this one can read them.
    var newer by remember { mutableLongStateOf(0L) }
    var discarding by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(revision, reviewed) {
        productNames = repo.productLabels()
        effectNames = repo.tags(EFFECT_CATEGORY).associate { it.id to it.data.name }
        tagNames = (repo.tags(EFFECT_CATEGORY) + repo.tags("flavor") + repo.tags("mood") + repo.tags("terpene"))
            .associate { it.id to it.data.name }
        notes = repo.standaloneSnippets()
        newer = runCatching { repo.newerRecordCount() }.getOrDefault(0L)
        sessions = repo.sessions()
    }
    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(padding),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        item {
            Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.journal_title), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.weight(1f))
                IconButton(onClick = onWhatITrack) {
                    Icon(painterResource(R.drawable.ic_tune), contentDescription = stringResource(R.string.track_title))
                }
                IconButton(onClick = onLock) {
                    Icon(painterResource(R.drawable.ic_lock), contentDescription = stringResource(R.string.lock_now))
                }
            }
        }
        // Notes written while the journal was locked: anyone holding the
        // phone could have added them, so the user keeps or discards each.
        val toReview = notes.count { it.data.addedWhileLocked }
        if (toReview > 0) {
            item {
                Text(
                    pluralStringResource(R.plurals.capture_review, toReview, toReview),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
        if (newer > 0) {
            item {
                Text(
                    pluralStringResource(R.plurals.newer_records_kept, newer.toInt(), newer.toInt()),
                    style = MaterialTheme.typography.bodyMedium,
                )
            }
        }
        // Purchase prompts use the same geofencing, so they need the same permissions.
        if (settings.notifyNearSpots || settings.purchasePrompts) item { NotifySetupCard(alerts, onPermissions) }
        // "Show me", or the tap on a "Notify me" alert: which spot is near.
        if (settings.showNearbySpots || checkNearby) item { NearbyBanner(repo, onSessionAt, onPurchase.takeIf { purchases }) }
        // Insights are computed only when opened, and only while turned on.
        if (settings.insights) {
            item {
                androidx.compose.material3.OutlinedButton(onClick = onInsights, modifier = Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.insights_open))
                }
            }
        }
        val list = sessions
        // One timeline: sessions and standalone notes, newest first.
        val timeline: List<Pair<Long, Entry<*>>> = (
            list.orEmpty().map { it.data.`when`.unixMs to it } + notes.map { it.data.at.unixMs to it }
            ).sortedByDescending { it.first }
        if (list != null && timeline.isEmpty()) {
            item { Text(stringResource(R.string.journal_empty), style = MaterialTheme.typography.bodyLarge) }
        }
        items(timeline, key = { it.second.id }) { (_, entry) ->
            when (val data = entry.data) {
                is SessionData -> SessionCard(data, productNames, effectNames) { onOpen(entry.id) }
                is SnippetData -> NoteCard(
                    data,
                    tagNames,
                    onKeep = {
                        scope.launch {
                            repo.keepCapture(entry.id, data)
                            reviewed++
                        }
                    },
                    onDiscard = { discarding = entry.id },
                ) { onOpenNote(entry.id) }
            }
        }
    }
    discarding?.let { id ->
        AlertDialog(
            onDismissRequest = { discarding = null },
            title = { Text(stringResource(R.string.capture_discard_title)) },
            text = { Text(stringResource(R.string.note_delete_body)) },
            confirmButton = {
                TextButton(onClick = {
                    discarding = null
                    scope.launch {
                        repo.delete(id)
                        reviewed++
                    }
                }) { Text(stringResource(R.string.capture_discard)) }
            },
            dismissButton = { TextButton(onClick = { discarding = null }) { Text(stringResource(android.R.string.cancel)) } },
        )
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun SessionCard(
    session: SessionData,
    productNames: Map<String, String>,
    effectNames: Map<String, String>,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLowest),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    formatWhen(session.`when`.unixMs, session.`when`.utcOffsetMin),
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.weight(1f),
                )
                session.rating?.let { Stars(it.toInt()) }
            }
            val names = session.lines.mapNotNull { line -> line.product?.let(productNames::get) }
            Text(
                names.joinToString(" · ").ifEmpty { stringResource(R.string.journal_untitled) },
                style = MaterialTheme.typography.titleLarge,
            )
            val methods = session.lines.map { it.method }.distinct()
            if (methods.isNotEmpty()) {
                Text(methods.map { label(it) }.joinToString(" · "), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            val top = session.effects.filter { it.value > 0 }.sortedByDescending { it.value }.take(3)
            if (top.isNotEmpty()) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    for (effect in top) AssistChip(onClick = onClick, label = { Text(tagLabel(effect.tag, effectNames)) })
                }
            }
        }
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class)
private fun NoteCard(
    note: SnippetData,
    customNames: Map<String, String>,
    onKeep: () -> Unit,
    onDiscard: () -> Unit,
    onClick: () -> Unit,
) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(
                formatWhen(note.at.unixMs, note.at.utcOffsetMin),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(note.text, style = MaterialTheme.typography.bodyLarge, maxLines = 4, overflow = TextOverflow.Ellipsis)
            if (note.tags.isNotEmpty()) {
                FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    for (t in note.tags.take(MAX_CARD_TAGS)) AssistChip(onClick = onClick, label = { Text(tagLabel(t.tag, customNames)) })
                }
            }
            if (note.addedWhileLocked) {
                Text(
                    stringResource(R.string.capture_added_while_locked),
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.tertiary,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = onKeep) { Text(stringResource(R.string.capture_keep)) }
                    TextButton(onClick = onDiscard) { Text(stringResource(R.string.capture_discard)) }
                }
            }
        }
    }
}

@Composable
private fun Stars(rating: Int) {
    val description = pluralStringResource(R.plurals.rated, rating, rating)
    Text(
        "★".repeat(rating) + "☆".repeat(5 - rating),
        color = MaterialTheme.colorScheme.tertiary,
        modifier = Modifier.semantics { contentDescription = description },
    )
}

@Composable
private fun CatalogLists(
    repo: JournalRepository,
    vocab: Vocabulary,
    revision: Int,
    padding: PaddingValues,
    onOpen: (Screen) -> Unit,
) {
    var catalog by remember { mutableStateOf(Catalog.PRODUCTS) }
    // "What I track": hidden strains or brands lose their lists.
    val tracking = LocalTracking.current
    val catalogs = Catalog.entries.filter {
        when (it) {
            Catalog.STRAINS -> tracking.shows(DataPoint.STRAIN)
            Catalog.BRANDS -> tracking.shows(DataPoint.BRAND)
            else -> true
        }
    }
    if (catalog !in catalogs) catalog = Catalog.PRODUCTS
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var strains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var brands by remember { mutableStateOf<List<Entry<BrandData>>>(emptyList()) }
    var spots by remember { mutableStateOf<List<Entry<SpotData>>>(emptyList()) }
    LaunchedEffect(revision) {
        products = repo.products()
        strains = repo.strains()
        brands = repo.brands()
        spots = repo.spots()
    }
    // A product shows without its brand when brands are hidden.
    val brandNames = if (tracking.shows(DataPoint.BRAND)) brands.associate { it.id to it.data.name } else emptyMap()
    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(padding),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item {
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                catalogs.forEachIndexed { index, c ->
                    SegmentedButton(
                        selected = catalog == c,
                        onClick = { catalog = c },
                        shape = SegmentedButtonDefaults.itemShape(index, catalogs.size),
                        // No checkmark: the fill already marks the selection, and
                        // the icon squeezed "Products" onto two lines.
                        icon = {},
                        label = {
                            Text(
                                stringResource(
                                    when (c) {
                                        Catalog.PRODUCTS -> R.string.tab_products
                                        Catalog.STRAINS -> R.string.catalog_strains
                                        Catalog.BRANDS -> R.string.catalog_brands
                                        Catalog.SPOTS -> R.string.catalog_spots
                                    },
                                ),
                                maxLines = 1,
                                softWrap = false,
                                overflow = TextOverflow.Ellipsis,
                                style = MaterialTheme.typography.labelMedium,
                            )
                        },
                    )
                }
            }
        }
        item {
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            androidx.compose.material3.OutlinedButton(onClick = {
                onOpen(
                    when (catalog) {
                        Catalog.PRODUCTS -> Screen.Product(null)
                        Catalog.STRAINS -> Screen.Strain(null)
                        Catalog.BRANDS -> Screen.Brand(null)
                        Catalog.SPOTS -> Screen.Spot(null)
                    },
                )
            }) {
                Text(
                    stringResource(
                        when (catalog) {
                            Catalog.PRODUCTS -> R.string.product_new
                            Catalog.STRAINS -> R.string.strain_new
                            Catalog.BRANDS -> R.string.brand_new
                            Catalog.SPOTS -> R.string.spot_new
                        },
                    ),
                )
            }
            if (catalog == Catalog.PRODUCTS && tracking.shows(DataPoint.PURCHASES)) {
                androidx.compose.material3.OutlinedButton(onClick = { onOpen(Screen.Receipt) }) {
                    Text(stringResource(R.string.scan_receipt))
                }
            }
            }
        }
        when (catalog) {
            Catalog.PRODUCTS -> items(
                products.sortedBy { productLabel(it.data.name, it.data.brand?.let(brandNames::get)).lowercase() },
                key = { it.id },
            ) { p ->
                val detail = p.data.form?.let { label(it) }.orEmpty()
                CatalogRow(productLabel(p.data.name, p.data.brand?.let(brandNames::get)), detail) { onOpen(Screen.Product(p.id)) }
            }
            Catalog.STRAINS -> items(strains, key = { it.id }) { s ->
                CatalogRow(s.data.name, s.data.strainType?.let { label(it) }.orEmpty()) { onOpen(Screen.Strain(s.id)) }
            }
            Catalog.BRANDS -> items(brands, key = { it.id }) { b ->
                CatalogRow(b.data.name, "") { onOpen(Screen.Brand(b.id)) }
            }
            Catalog.SPOTS -> items(spots, key = { it.id }) { sp ->
                val purposes = sp.data.purposes.mapNotNull { (it as? TagRefData.Builtin)?.slug }.map { label(it) }
                CatalogRow(sp.data.name, purposes.joinToString(" · ")) { onOpen(Screen.Spot(sp.id)) }
            }
        }
    }
}

/**
 * "Show me": a location fix taken now, while the journal is open, compared
 * with saved spots inside the core. Nothing is registered with the OS and
 * the fix is not kept.
 */
@Composable
private fun NearbyBanner(repo: JournalRepository, onSessionAt: (String) -> Unit, onPurchase: (() -> Unit)?) {
    val context = LocalContext.current
    var access by remember { mutableStateOf(CurrentLocation.access(context)) }
    var near by remember { mutableStateOf<List<NearbySpot>>(emptyList()) }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        access = CurrentLocation.access(context)
    }
    LaunchedEffect(access) {
        if (access != LocationAccess.NONE) {
            near = CurrentLocation.get(context)?.let { repo.spotsNear(it.latE7, it.lonE7, it.accuracyM) }.orEmpty()
        }
    }
    if (access == LocationAccess.NONE) {
        Card(modifier = Modifier.fillMaxWidth()) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(stringResource(R.string.show_nearby_needs_location))
                androidx.compose.material3.OutlinedButton(onClick = { ask.launch(arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION)) }) {
                    Text(stringResource(R.string.show_nearby_allow))
                }
            }
        }
        return
    }
    val spot = near.firstOrNull() ?: return
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.show_nearby_at, spot.name), style = MaterialTheme.typography.titleMedium)
            androidx.compose.material3.OutlinedButton(onClick = { onSessionAt(spot.id) }) {
                Text(stringResource(R.string.show_nearby_log_here))
            }
            // "Show me" stands in for purchase prompts at a Re-up Spot.
            if (spot.reUp && onPurchase != null) {
                androidx.compose.material3.OutlinedButton(onClick = onPurchase) {
                    Text(stringResource(R.string.show_nearby_log_purchase))
                }
            }
        }
    }
}

/**
 * What "Notify me" still needs, one permission at a time: precise location,
 * then location "all the time" (Android opens its settings for that), then
 * notifications. Nothing shows once all are granted.
 */
// POST_NOTIFICATIONS is asked only on Android 13+, where needs() lists it.
@SuppressLint("InlinedApi")
@Composable
private fun NotifySetupCard(alerts: SpotAlerts, onGranted: () -> Unit) {
    var needs by remember { mutableStateOf(alerts.needs()) }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        needs = alerts.needs()
        onGranted()
    }
    val next = needs.firstOrNull() ?: return
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.notify_setup_title), style = MaterialTheme.typography.titleMedium)
            Text(
                stringResource(
                    when (next) {
                        AlertNeed.PRECISE_LOCATION -> R.string.notify_setup_precise
                        AlertNeed.BACKGROUND_LOCATION -> R.string.notify_setup_background
                        AlertNeed.NOTIFICATIONS -> R.string.notify_setup_notifications
                    },
                ),
            )
            androidx.compose.material3.OutlinedButton(
                onClick = {
                    ask.launch(
                        when (next) {
                            AlertNeed.PRECISE_LOCATION -> arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION)
                            AlertNeed.BACKGROUND_LOCATION -> arrayOf(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
                            AlertNeed.NOTIFICATIONS -> arrayOf(Manifest.permission.POST_NOTIFICATIONS)
                        },
                    )
                },
            ) {
                Text(stringResource(R.string.notify_setup_allow))
            }
        }
    }
}

@Composable
private fun CatalogRow(title: String, detail: String, onClick: () -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLowest),
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            if (detail.isNotEmpty()) Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** Tags shown on a note card; the note itself lists them all. */
private const val MAX_CARD_TAGS = 4
