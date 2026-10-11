// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

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
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.BrandData
import app.terpoir.core.ProductData
import app.terpoir.core.SessionData
import app.terpoir.core.StrainData
import app.terpoir.core.Vocabulary

/** Where the user is inside the unlocked app. */
private sealed interface Screen {
    data object Home : Screen

    data class Session(val id: String?) : Screen

    data class Product(val id: String?) : Screen

    data class Strain(val id: String?) : Screen

    data class Brand(val id: String?) : Screen
}

private enum class Tab { JOURNAL, PRODUCTS, PRIVACY }

private enum class Catalog { PRODUCTS, STRAINS, BRANDS }

/**
 * The unlocked app: journal, products and the Privacy Center, plus the
 * editors. Navigation is a small in-memory stack; the system Back pops it.
 */
@Composable
fun JournalHome(
    repo: JournalRepository,
    vocab: Vocabulary,
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

    when (val screen = stack.last()) {
        is Screen.Session -> SessionEditor(repo, vocab, screen.id, pop)
        is Screen.Product -> ProductEditor(repo, vocab, screen.id, pop)
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
                    ExtendedFloatingActionButton(
                        onClick = { stack.add(Screen.Session(null)) },
                        icon = { Icon(painterResource(R.drawable.ic_add), contentDescription = null) },
                        text = { Text(stringResource(R.string.session_new)) },
                    )
                }
            },
        ) { padding ->
            when (tab) {
                Tab.JOURNAL -> JournalList(repo, revision, padding, onLock) { stack.add(Screen.Session(it)) }
                Tab.PRODUCTS -> CatalogLists(repo, vocab, revision, padding) { stack.add(it) }
                Tab.PRIVACY -> Column(Modifier.padding(padding)) { privacyCenter { tab = Tab.JOURNAL } }
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
    revision: Int,
    padding: PaddingValues,
    onLock: () -> Unit,
    onOpen: (String) -> Unit,
) {
    var sessions by remember { mutableStateOf<List<Entry<SessionData>>?>(null) }
    var productNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    var effectNames by remember { mutableStateOf<Map<String, String>>(emptyMap()) }
    LaunchedEffect(revision) {
        productNames = repo.products().associate { it.id to it.data.name }
        effectNames = repo.tags(EFFECT_CATEGORY).associate { it.id to it.data.name }
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
                IconButton(onClick = onLock) {
                    Icon(painterResource(R.drawable.ic_lock), contentDescription = stringResource(R.string.lock_now))
                }
            }
        }
        val list = sessions
        if (list != null && list.isEmpty()) {
            item { Text(stringResource(R.string.journal_empty), style = MaterialTheme.typography.bodyLarge) }
        }
        items(list.orEmpty(), key = { it.id }) { entry ->
            SessionCard(entry.data, productNames, effectNames) { onOpen(entry.id) }
        }
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
    var products by remember { mutableStateOf<List<Entry<ProductData>>>(emptyList()) }
    var strains by remember { mutableStateOf<List<Entry<StrainData>>>(emptyList()) }
    var brands by remember { mutableStateOf<List<Entry<BrandData>>>(emptyList()) }
    LaunchedEffect(revision) {
        products = repo.products()
        strains = repo.strains()
        brands = repo.brands()
    }
    val brandNames = brands.associate { it.id to it.data.name }
    LazyColumn(
        modifier = Modifier.fillMaxSize().padding(padding),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        item {
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                Catalog.entries.forEachIndexed { index, c ->
                    SegmentedButton(
                        selected = catalog == c,
                        onClick = { catalog = c },
                        shape = SegmentedButtonDefaults.itemShape(index, Catalog.entries.size),
                        label = {
                            Text(
                                stringResource(
                                    when (c) {
                                        Catalog.PRODUCTS -> R.string.tab_products
                                        Catalog.STRAINS -> R.string.catalog_strains
                                        Catalog.BRANDS -> R.string.catalog_brands
                                    },
                                ),
                            )
                        },
                    )
                }
            }
        }
        item {
            androidx.compose.material3.OutlinedButton(onClick = {
                onOpen(
                    when (catalog) {
                        Catalog.PRODUCTS -> Screen.Product(null)
                        Catalog.STRAINS -> Screen.Strain(null)
                        Catalog.BRANDS -> Screen.Brand(null)
                    },
                )
            }) {
                Text(
                    stringResource(
                        when (catalog) {
                            Catalog.PRODUCTS -> R.string.product_new
                            Catalog.STRAINS -> R.string.strain_new
                            Catalog.BRANDS -> R.string.brand_new
                        },
                    ),
                )
            }
        }
        when (catalog) {
            Catalog.PRODUCTS -> items(products, key = { it.id }) { p ->
                val detail = listOfNotNull(p.data.form?.let { label(it) }, p.data.brand?.let(brandNames::get)).joinToString(" · ")
                CatalogRow(p.data.name, detail) { onOpen(Screen.Product(p.id)) }
            }
            Catalog.STRAINS -> items(strains, key = { it.id }) { s ->
                CatalogRow(s.data.name, s.data.strainType?.let { label(it) }.orEmpty()) { onOpen(Screen.Strain(s.id)) }
            }
            Catalog.BRANDS -> items(brands, key = { it.id }) { b ->
                CatalogRow(b.data.name, "") { onOpen(Screen.Brand(b.id)) }
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
