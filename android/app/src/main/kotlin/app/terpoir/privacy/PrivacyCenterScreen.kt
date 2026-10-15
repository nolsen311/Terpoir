// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import kotlinx.coroutines.launch

/**
 * The Privacy Center: a status summary, the presets, "Restore maximum
 * privacy", and one row per section, each saying whether everything in it
 * is private. A section opens on its own page with its toggles, their
 * cards, and the settings that belong with them ([sectionExtras]).
 * Relaxing a protection first shows its disclosure; returning to the
 * private state needs no confirmation.
 *
 * @param onChanged called after any action ran, so the caller can apply
 *   window-level settings at once.
 * @param tour the onboarding tour: an introduction, and closing finishes
 *   setup.
 * @param sectionExtras settings shown at the end of a section's page, such
 *   as the map provider under Location.
 * @param settings when set, this is the Settings tab (maintainer,
 *   2026-10-14, option A): titled Settings, with the privacy part first and
 *   these groups (journal and phone settings) after it, then About with the
 *   privacy policy and [version]. Not used in the tour.
 */
@Composable
fun PrivacyCenterScreen(
    center: PrivacyCenter,
    onChanged: () -> Unit,
    onClose: () -> Unit,
    tour: Boolean = false,
    sectionExtras: Map<PrivacyGroup, @Composable () -> Unit> = emptyMap(),
    settings: (@Composable () -> Unit)? = null,
    version: String = "",
) {
    val asSettings = settings != null && !tour
    val scope = rememberCoroutineScope()
    // Toggle state lives in settings, not Compose; bumped after each action.
    var revision by remember { mutableIntStateOf(0) }
    var confirming by remember { mutableStateOf<PrivacyToggle?>(null) }
    var turningOff by remember { mutableStateOf<PrivacyToggle?>(null) }
    var choosing by remember { mutableStateOf<PrivacyPreset?>(null) }
    // The tour's optional question; never stored.
    var threat by remember { mutableStateOf<Threat?>(null) }
    var readingPolicy by remember { mutableStateOf(false) }
    var section by remember { mutableStateOf<PrivacyGroup?>(null) }
    fun run(action: suspend () -> Unit) {
        scope.launch {
            action()
            revision++
            onChanged()
        }
    }
    fun switch(toggle: PrivacyToggle, wanted: Boolean) {
        if (wanted == toggle.privateState) {
            if (toggle.offChoice != null) turningOff = toggle else run { toggle.set(wanted) }
        } else {
            confirming = toggle
        }
    }
    // During the tour, Back leaves the app as usual instead of skipping it.
    BackHandler(enabled = !tour, onBack = onClose)
    // Declared later, so it wins: Back from a section returns to the list.
    BackHandler(enabled = section != null) { section = null }
    if (readingPolicy) {
        PrivacyPolicyScreen(onClose = { readingPolicy = false })
        return
    }
    // Every section with a toggle or a setting, in display order.
    val toggles = center.groups
    val sections = PrivacyGroup.entries.filter { it in toggles || it in sectionExtras }

    val open = section
    if (open != null) {
        SectionPage(
            group = open,
            toggles = toggles[open].orEmpty(),
            revision = revision,
            onSwitch = ::switch,
            onBack = { section = null },
            extras = sectionExtras[open],
        )
    } else {
        Column(
            modifier = Modifier.verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(stringResource(if (asSettings) R.string.settings_title else R.string.privacy_center), style = MaterialTheme.typography.headlineMedium)
            if (tour) Text(stringResource(R.string.privacy_tour_intro))
            if (asSettings) SettingsGroupTitle(stringResource(R.string.settings_privacy))
            val relaxed = revision.let { center.relaxedCount() }
            val preset = revision.let { center.currentPreset() }
            Surface(
                color = if (relaxed == 0) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.tertiaryContainer,
                shape = MaterialTheme.shapes.medium,
            ) {
                Column(modifier = Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text(
                        if (relaxed == 0) {
                            stringResource(R.string.privacy_all_on)
                        } else {
                            pluralStringResource(R.plurals.privacy_relaxed, relaxed, relaxed)
                        },
                        style = MaterialTheme.typography.titleMedium,
                    )
                    Text(
                        stringResource(R.string.preset_status, preset?.let { stringResource(it.title) } ?: stringResource(R.string.preset_custom)),
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            }
            if (tour) ThreatQuestion(threat, onAnswer = { threat = it }) { suggested -> center.presets.firstOrNull { it.id == suggested }?.let { choosing = it } }
            PresetButtons(center.presets) { choosing = it }
            Surface(color = MaterialTheme.colorScheme.surfaceContainer, shape = MaterialTheme.shapes.medium) {
                Column {
                    sections.forEachIndexed { i, group ->
                        if (i > 0) HorizontalDivider()
                        val inGroup = toggles[group].orEmpty()
                        val on = revision.let { inGroup.count { !it.isPrivate } }
                        SectionRow(group, inGroup.map { stringResource(it.card.title) }, on) { section = group }
                    }
                }
            }
            Button(onClick = { run { center.restoreMaximumPrivacy() } }) {
                Text(stringResource(R.string.privacy_restore))
            }
            if (asSettings) {
                settings?.invoke()
                SettingsGroup(stringResource(R.string.settings_about)) {
                    SettingsRow(stringResource(R.string.privacy_policy), null) { readingPolicy = true }
                    HorizontalDivider()
                    SettingsRow(stringResource(R.string.settings_version, version), null, onClick = null)
                }
            } else {
                OutlinedButton(onClick = { readingPolicy = true }) { Text(stringResource(R.string.privacy_policy)) }
            }
            if (tour) {
                Button(onClick = onClose) { Text(stringResource(R.string.privacy_tour_finish)) }
            } else if (!asSettings) {
                TextButton(onClick = onClose) { Text(stringResource(R.string.privacy_close)) }
            }
        }
    }

    // One combined confirmation for a preset: every toggle it changes, with
    // the card of each one it relaxes.
    choosing?.let { option ->
        val changes = remember(option, revision) { center.changes(option) }
        AlertDialog(
            onDismissRequest = { choosing = null },
            title = { Text(stringResource(R.string.preset_confirm_title, stringResource(option.title))) },
            text = {
                Column(
                    modifier = Modifier.verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Text(stringResource(option.description))
                    if (changes.isEmpty()) Text(stringResource(R.string.preset_nothing))
                    for ((toggle, on) in changes) {
                        Text(
                            stringResource(if (on) R.string.preset_turns_on else R.string.preset_turns_off, stringResource(toggle.card.title)),
                            style = MaterialTheme.typography.titleSmall,
                        )
                        if (on != toggle.privateState) {
                            Text(stringResource(toggle.card.whatLeaves), style = MaterialTheme.typography.bodySmall)
                            Text(stringResource(toggle.card.whoCanSee), style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    choosing = null
                    run { center.apply(option) }
                }) { Text(stringResource(R.string.preset_apply)) }
            },
            dismissButton = {
                TextButton(onClick = { choosing = null }) { Text(stringResource(R.string.cancel)) }
            },
        )
    }

    turningOff?.let { toggle ->
        val choice = toggle.offChoice ?: return@let
        AlertDialog(
            onDismissRequest = { turningOff = null },
            title = { Text(stringResource(toggle.card.title)) },
            text = { Text(stringResource(choice.question)) },
            confirmButton = {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = {
                        turningOff = null
                        run {
                            toggle.set(toggle.privateState)
                            choice.delete()
                        }
                    }) { Text(stringResource(choice.deleteLabel), color = MaterialTheme.colorScheme.error) }
                    TextButton(onClick = {
                        turningOff = null
                        run { toggle.set(toggle.privateState) }
                    }) { Text(stringResource(choice.hideLabel)) }
                }
            },
            dismissButton = {
                TextButton(onClick = { turningOff = null }) { Text(stringResource(R.string.cancel)) }
            },
        )
    }

    confirming?.let { toggle ->
        AlertDialog(
            onDismissRequest = { confirming = null },
            title = { Text(stringResource(toggle.card.title)) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(toggle.card.whatLeaves))
                    Text(stringResource(toggle.card.whoCanSee))
                }
            },
            confirmButton = {
                TextButton(onClick = {
                    confirming = null
                    run { toggle.set(!toggle.privateState) }
                }) { Text(stringResource(R.string.privacy_relax)) }
            },
            dismissButton = {
                TextButton(onClick = { confirming = null }) { Text(stringResource(R.string.privacy_keep)) }
            },
        )
    }
}

/** Answers to the tour's optional question, each suggesting a preset by ID. */
private enum class Threat(val label: Int, val suggests: String) {
    PEOPLE(R.string.threat_people, "most_private"),
    LEGAL(R.string.threat_legal, "most_private"),
    COMPANIES(R.string.threat_companies, "most_private"),
    LITTLE(R.string.threat_little, "yolo"),
}

/**
 * "What are you protecting this journal from?": optional, never stored. An
 * answer only suggests a preset; nothing changes until the user reviews and
 * confirms it.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun ThreatQuestion(answer: Threat?, onAnswer: (Threat) -> Unit, onReview: (String) -> Unit) {
    Text(stringResource(R.string.threat_question), style = MaterialTheme.typography.titleSmall)
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (option in Threat.entries) {
            FilterChip(selected = answer == option, onClick = { onAnswer(option) }, label = { Text(stringResource(option.label)) })
        }
    }
    val suggested = answer?.suggests
    when (suggested) {
        null -> Unit
        "most_private" -> Text(stringResource(R.string.threat_suggest_default), style = MaterialTheme.typography.bodySmall)
        else -> Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(stringResource(R.string.threat_suggest_yolo), style = MaterialTheme.typography.bodySmall, modifier = Modifier.weight(1f))
            OutlinedButton(onClick = { onReview(suggested) }) { Text(stringResource(R.string.preset_review)) }
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun PresetButtons(presets: List<PrivacyPreset>, onChoose: (PrivacyPreset) -> Unit) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        for (option in presets) {
            OutlinedButton(onClick = { onChoose(option) }) { Text(stringResource(option.title)) }
        }
    }
}

/** A group label on the Settings page. */
@Composable
fun SettingsGroupTitle(text: String) {
    Text(text.uppercase(), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

/** A titled group of [SettingsRow]s on the Settings page, in one card. */
@Composable
fun SettingsGroup(title: String, rows: @Composable () -> Unit) {
    SettingsGroupTitle(title)
    Surface(color = MaterialTheme.colorScheme.surfaceContainer, shape = MaterialTheme.shapes.medium) {
        Column { rows() }
    }
}

/** One row of a [SettingsGroup]: a title, an optional line under it, and a chevron when it opens something. */
@Composable
fun SettingsRow(title: String, subtitle: String?, onClick: (() -> Unit)?) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .then(if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier)
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMedium)
            if (subtitle != null) Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
        if (onClick != null) Text("›", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

/** A section on the main page: its name, its toggles, and whether any is relaxed. */
@Composable
private fun SectionRow(group: PrivacyGroup, toggleTitles: List<String>, relaxed: Int, onOpen: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().clickable(onClick = onOpen).padding(horizontal = 16.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(stringResource(group.title), style = MaterialTheme.typography.titleMedium)
            if (toggleTitles.isNotEmpty()) {
                Text(toggleTitles.joinToString(", "), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        if (relaxed > 0) {
            StatusPill(pluralStringResource(R.plurals.privacy_section_on, relaxed, relaxed), MaterialTheme.colorScheme.tertiaryContainer)
        } else {
            StatusPill(stringResource(R.string.privacy_section_private), MaterialTheme.colorScheme.secondaryContainer)
        }
        Text("›", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

@Composable
private fun StatusPill(text: String, color: Color) {
    Surface(color = color, shape = MaterialTheme.shapes.small) {
        Text(text, style = MaterialTheme.typography.labelMedium, modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp))
    }
}

/**
 * One section's page: its toggles in one card, each with its switch and
 * what it does, then the section's own settings. A relaxed toggle's
 * details (what leaves, who can see, how to undo) are always open, since
 * they say what is leaving the phone; a private one's open on request.
 */
@Composable
private fun SectionPage(
    group: PrivacyGroup,
    toggles: List<PrivacyToggle>,
    revision: Int,
    onSwitch: (PrivacyToggle, Boolean) -> Unit,
    onBack: () -> Unit,
    extras: (@Composable () -> Unit)?,
) {
    var expanded by remember(group) { mutableStateOf(setOf<String>()) }
    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()).padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        TextButton(onClick = onBack) { Text(stringResource(R.string.privacy_back)) }
        Text(stringResource(group.title), style = MaterialTheme.typography.headlineMedium)
        if (toggles.isNotEmpty()) {
            Surface(color = MaterialTheme.colorScheme.surfaceContainer, shape = MaterialTheme.shapes.large) {
                Column {
                    toggles.forEachIndexed { i, toggle ->
                        if (i > 0) HorizontalDivider()
                        val on = revision.let { toggle.isOn() }
                        val relaxed = on != toggle.privateState
                        val open = relaxed || toggle.id in expanded
                        ToggleRow(toggle, on, open, canCollapse = !relaxed, onSwitch = { onSwitch(toggle, it) }) {
                            expanded = if (toggle.id in expanded) expanded - toggle.id else expanded + toggle.id
                        }
                    }
                }
            }
        }
        extras?.invoke()
        TextButton(onClick = onBack) { Text(stringResource(R.string.privacy_back)) }
    }
}

@Composable
private fun ToggleRow(
    toggle: PrivacyToggle,
    on: Boolean,
    open: Boolean,
    canCollapse: Boolean,
    onSwitch: (Boolean) -> Unit,
    onExpand: () -> Unit,
) {
    Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
                Text(stringResource(toggle.card.title), style = MaterialTheme.typography.titleMedium)
                Text(stringResource(toggle.card.whatItDoes), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = on, onCheckedChange = onSwitch)
        }
        if (open) {
            CardLine(R.string.privacy_card_leaves, toggle.card.whatLeaves)
            CardLine(R.string.privacy_card_who, toggle.card.whoCanSee)
            CardLine(R.string.privacy_card_undo, toggle.card.howToUndo)
        }
        if (canCollapse) {
            Text(
                stringResource(if (open) R.string.privacy_card_hide else R.string.privacy_card_show),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.primary,
                modifier = Modifier.clickable(onClick = onExpand).padding(vertical = 4.dp),
            )
        }
    }
}

/** One part of a toggle's card, with its label in bold. */
@Composable
private fun CardLine(label: Int, text: Int) {
    val heading = stringResource(label)
    val body = stringResource(text)
    Text(
        buildAnnotatedString {
            withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) { append(heading) }
            append(" ")
            append(body)
        },
        style = MaterialTheme.typography.bodySmall,
    )
}

private val PrivacyGroup.title: Int
    get() = when (this) {
        PrivacyGroup.JOURNAL -> R.string.privacy_group_journal
        PrivacyGroup.LOCATION -> R.string.privacy_group_location
        PrivacyGroup.NOTIFICATIONS -> R.string.privacy_group_notifications
        PrivacyGroup.CATALOGS -> R.string.privacy_group_catalogs
        PrivacyGroup.SCANNING -> R.string.privacy_group_scanning
        PrivacyGroup.OS_INTEGRATION -> R.string.privacy_group_os_integration
        PrivacyGroup.KEYBOARD_AND_VOICE -> R.string.privacy_group_keyboard_and_voice
        PrivacyGroup.PHOTOS -> R.string.privacy_group_photos
        PrivacyGroup.SYNC_AND_KEYS -> R.string.privacy_group_sync_and_keys
        PrivacyGroup.DEVICE_SECURITY -> R.string.privacy_group_device_security
    }
