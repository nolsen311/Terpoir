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
import androidx.compose.foundation.layout.Box
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
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
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
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberSwipeToDismissBoxState
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
import androidx.compose.ui.semantics.CustomAccessibilityAction
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.customActions
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
import app.terpoir.privacy.SettingsGroup
import app.terpoir.privacy.SettingsRow
import app.terpoir.ui.theme.AppearanceSettings
import app.terpoir.ui.theme.CardTones
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
    data object Theme : Screen
}

private enum class Tab { JOURNAL, PRODUCTS, SETTINGS }

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
    appearance: AppearanceSettings,
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
    newEntry: Boolean,
    onNewEntryOpened: () -> Unit,
    onEdited: () -> Unit,
    privacyCenter: @Composable (onClose: () -> Unit, settings: @Composable () -> Unit) -> Unit,
) {
    val stack = remember { mutableStateListOf<Screen>(Screen.Home) }
    var tab by remember { mutableStateOf(Tab.JOURNAL) }
    // The Products tab's list, here so its "New" button can be the FAB.
    var catalog by remember { mutableStateOf(Catalog.PRODUCTS) }
    // Location cards swiped away on the journal page. Kept in memory only,
    // so they come back after the next unlock.
    val dismissed = remember { mutableStateListOf<String>() }
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
    // Opened from a "New entry" shortcut, tile or widget.
    LaunchedEffect(newEntry) {
        if (newEntry) {
            stack.add(Screen.Session(null))
            onNewEntryOpened()
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
            Screen.Theme -> ThemeScreen(appearance, onClose = pop)
            is Screen.Note -> NoteScreen(repo, settings, vocab, screen.id, pop)
            is Screen.Strain -> StrainEditor(repo, vocab, screen.id, pop)
            is Screen.Brand -> BrandEditor(repo, screen.id, pop)
            Screen.Home -> Scaffold(
                bottomBar = {
                    NavigationBar {
                        NavItem(tab == Tab.JOURNAL, R.drawable.ic_journal, R.string.tab_journal) { tab = Tab.JOURNAL }
                        NavItem(tab == Tab.PRODUCTS, R.drawable.ic_products, R.string.tab_products) { tab = Tab.PRODUCTS }
                        NavItem(tab == Tab.SETTINGS, R.drawable.ic_tune, R.string.tab_settings) { tab = Tab.SETTINGS }
                    }
                },
                floatingActionButton = {
                    // Products: "New" for whichever list is showing.
                    if (tab == Tab.PRODUCTS) {
                        Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                            // Scanning a receipt is the products list's second action
                            // (maintainer, 2026-10-15), while purchases are tracked.
                            if (catalog == Catalog.PRODUCTS && tracking.shows(DataPoint.PURCHASES)) {
                                SmallFloatingActionButton(onClick = { stack.add(Screen.Receipt) }) {
                                    Icon(painterResource(R.drawable.ic_scan), contentDescription = stringResource(R.string.scan_receipt))
                                }
                            }
                            ExtendedFloatingActionButton(
                                onClick = {
                                    stack.add(
                                        when (catalog) {
                                            Catalog.PRODUCTS -> Screen.Product(null)
                                            Catalog.STRAINS -> Screen.Strain(null)
                                            Catalog.BRANDS -> Screen.Brand(null)
                                            Catalog.SPOTS -> Screen.Spot(null)
                                        },
                                    )
                                },
                                icon = { Icon(painterResource(R.drawable.ic_add), contentDescription = null) },
                                text = {
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
                                },
                            )
                        }
                    }
                    if (tab == Tab.JOURNAL) {
                        Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(12.dp)) {
                            // A note is the secondary action (maintainer, 2026-10-15);
                            // it still opens by voice unless the user chose typing.
                            SmallFloatingActionButton(onClick = { stack.add(Screen.Note(null)) }) {
                                Icon(painterResource(R.drawable.ic_note_add), contentDescription = stringResource(R.string.note_new))
                            }
                            // A new session is the main action.
                            ExtendedFloatingActionButton(
                                onClick = { stack.add(Screen.Session(null)) },
                                icon = { Icon(painterResource(R.drawable.ic_add), contentDescription = null) },
                                text = { Text(stringResource(R.string.session_new)) },
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
                        dismissed,
                        onEdited,
                        revision,
                        padding,
                        onOpenNote = { stack.add(Screen.Note(it)) },
                        onSessionAt = { stack.add(Screen.Session(null, spot = it)) },
                        onInsights = { stack.add(Screen.Insights) },
                        onPurchase = { stack.add(Screen.Receipt) },
                    ) {
                        stack.add(Screen.Session(it))
                    }
                    Tab.PRODUCTS -> CatalogLists(repo, revision, padding, catalog, { catalog = it }) { stack.add(it) }
                    Tab.SETTINGS -> Column(Modifier.padding(padding)) {
                        privacyCenter({ tab = Tab.JOURNAL }) {
                            SettingsGroup(stringResource(R.string.settings_journal)) {
                                SettingsRow(stringResource(R.string.track_title), stringResource(R.string.settings_track_hint)) { stack.add(Screen.WhatITrack) }
                            }
                            SettingsGroup(stringResource(R.string.settings_phone)) {
                                SettingsRow(stringResource(R.string.theme_title), stringResource(appearance.theme.title)) { stack.add(Screen.Theme) }
                            }
                        }
                    }
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
    dismissed: MutableList<String>,
    onPermissions: () -> Unit,
    revision: Int,
    padding: PaddingValues,
    onOpenNote: (String) -> Unit,
    onSessionAt: (String) -> Unit,
    onInsights: () -> Unit,
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
                // Insights are computed only when opened, and only while turned on.
                if (settings.insights) {
                    IconButton(onClick = onInsights) {
                        Icon(painterResource(R.drawable.ic_lightbulb), contentDescription = stringResource(R.string.insights_open))
                    }
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
        // Each location card can be swiped away (maintainer, 2026-10-15).
        if ((settings.notifyNearSpots || settings.purchasePrompts) && NOTIFY_SETUP !in dismissed) {
            item(key = NOTIFY_SETUP) { Dismissible({ dismissed += NOTIFY_SETUP }) { NotifySetupCard(alerts, onPermissions) } }
        }
        // "Show me", or the tap on a "Notify me" alert: which spot is near.
        if (settings.showNearbySpots || checkNearby) {
            item { NearbyBanner(repo, dismissed, onSessionAt, onPurchase.takeIf { purchases }) }
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
        colors = CardDefaults.cardColors(containerColor = CardTones.raised),
        border = CardTones.border,
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
        colors = CardDefaults.cardColors(containerColor = CardTones.quiet),
        border = CardTones.border,
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
    revision: Int,
    padding: PaddingValues,
    catalog: Catalog,
    onCatalog: (Catalog) -> Unit,
    onOpen: (Screen) -> Unit,
) {
    // "What I track": hidden strains or brands lose their lists.
    val tracking = LocalTracking.current
    val catalogs = Catalog.entries.filter {
        when (it) {
            Catalog.STRAINS -> tracking.shows(DataPoint.STRAIN)
            Catalog.BRANDS -> tracking.shows(DataPoint.BRAND)
            else -> true
        }
    }
    if (catalog !in catalogs) onCatalog(Catalog.PRODUCTS)
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var strains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var brands by remember { mutableStateOf<List<Entry<BrandData>>>(emptyList()) }
    var spots by remember { mutableStateOf<List<Entry<SpotData>>>(emptyList()) }
    var subtypeNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    LaunchedEffect(revision) {
        products = repo.products()
        strains = repo.strains()
        brands = repo.brands()
        spots = repo.spots()
        subtypeNames = repo.tags(SUBTYPE_CATEGORY).associate { it.id to it.data.name }
    }
    // A product shows without its brand when brands are hidden.
    val brandNames = if (tracking.shows(DataPoint.BRAND)) brands.associate { it.id to it.data.name } else emptyMap()
    val strainNames = if (tracking.shows(DataPoint.STRAIN)) strains.associate { it.id to it.data.name } else emptyMap()
    val showPotency = tracking.shows(DataPoint.POTENCY)
    @Composable
    fun amount(milli: Long, unit: String?): String =
        if (unit == "percent") "${formatMilli(milli)}%" else listOfNotNull(formatMilli(milli), unit?.let { label(it) }).joinToString(" ")
    // Each product titled by its strains, with brand, type, name and potency below.
    val productRows = products.map { p ->
        val d = p.data
        val potency = if (!showPotency) {
            emptyList()
        } else {
            listOfNotNull(
                d.thcMilli?.let { stringResource(R.string.catalog_thc, amount(it, d.thcUnit)) },
                d.cbdMilli?.let { stringResource(R.string.catalog_cbd, amount(it, d.cbdUnit)) },
            )
        }
        // Its kind ("Bubble hash") says more than its type ("Concentrate").
        val kind = when (val st = d.subtype) {
            is TagRefData.Builtin -> label(st.slug)
            is TagRefData.Custom -> subtypeNames[st.id]
            null -> null
        } ?: d.form?.let { label(it) }
        p.id to productTitle(d.name, d.strains.mapNotNull(strainNames::get), d.brand?.let(brandNames::get), kind, potency)
    }.sortedBy { it.second.title.lowercase() }
    // The lists swipe sideways (maintainer, 2026-10-15). The buttons above
    // follow the swipe, and tapping one scrolls to its list.
    val pager = rememberPagerState(initialPage = catalogs.indexOf(catalog).coerceAtLeast(0)) { catalogs.size }
    val scope = rememberCoroutineScope()
    LaunchedEffect(pager.currentPage, catalogs) {
        catalogs.getOrNull(pager.currentPage)?.let { if (it != catalog) onCatalog(it) }
    }
    Column(modifier = Modifier.fillMaxSize().padding(padding)) {
        Box(modifier = Modifier.padding(start = 16.dp, top = 16.dp, end = 16.dp, bottom = 8.dp)) {
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                catalogs.forEachIndexed { index, c ->
                    SegmentedButton(
                        selected = catalog == c,
                        onClick = { scope.launch { pager.animateScrollToPage(index) } },
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
        HorizontalPager(state = pager, modifier = Modifier.fillMaxSize(), key = { catalogs[it] }) { page ->
            LazyColumn(
                modifier = Modifier.fillMaxSize(),
                // Room at the bottom so the floating buttons don't cover the last row.
                contentPadding = PaddingValues(start = 16.dp, top = 8.dp, end = 16.dp, bottom = 160.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                when (catalogs[page]) {
                    Catalog.PRODUCTS -> items(productRows, key = { it.first }) { (id, row) ->
                        CatalogRow(row.title, row.subtitle) { onOpen(Screen.Product(id)) }
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
    }
}

/**
 * "Show me": a location fix taken now, while the journal is open, compared
 * with saved spots inside the core. Nothing is registered with the OS and
 * the fix is not kept.
 */
@Composable
private fun NearbyBanner(
    repo: JournalRepository,
    dismissed: MutableList<String>,
    onSessionAt: (String) -> Unit,
    onPurchase: (() -> Unit)?,
) {
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
        if (NEARBY_ACCESS in dismissed) return
        Dismissible({ dismissed += NEARBY_ACCESS }) {
            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(R.string.show_nearby_needs_location))
                    androidx.compose.material3.OutlinedButton(onClick = { ask.launch(arrayOf(Manifest.permission.ACCESS_COARSE_LOCATION)) }) {
                        Text(stringResource(R.string.show_nearby_allow))
                    }
                }
            }
        }
        return
    }
    // Swiping a spot away hides that spot only; another nearby one shows.
    val spot = near.firstOrNull { NEARBY_SPOT + it.id !in dismissed } ?: return
    Dismissible({ dismissed += NEARBY_SPOT + spot.id }) { NearbySpotCard(spot, onSessionAt, onPurchase) }
}

@Composable
private fun NearbySpotCard(spot: NearbySpot, onSessionAt: (String) -> Unit, onPurchase: (() -> Unit)?) {
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

/** Keys for the location cards swiped away (see JournalHome). */
private const val NOTIFY_SETUP = "notify-setup"
private const val NEARBY_ACCESS = "nearby-access"
private const val NEARBY_SPOT = "nearby-spot:"

/**
 * A card that can be swiped away in either direction. Screen readers get a
 * "Dismiss" action instead of the swipe.
 */
@Composable
private fun Dismissible(onDismiss: () -> Unit, content: @Composable () -> Unit) {
    val dismissLabel = stringResource(R.string.dismiss)
    SwipeToDismissBox(
        state = rememberSwipeToDismissBoxState(),
        backgroundContent = {},
        modifier = Modifier.semantics {
            customActions = listOf(
                CustomAccessibilityAction(dismissLabel) {
                    onDismiss()
                    true
                },
            )
        },
        onDismiss = { onDismiss() },
    ) { content() }
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
        colors = CardDefaults.cardColors(containerColor = CardTones.raised),
        border = CardTones.border,
    ) {
        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            if (detail.isNotEmpty()) Text(detail, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** Tags shown on a note card; the note itself lists them all. */
private const val MAX_CARD_TAGS = 4
