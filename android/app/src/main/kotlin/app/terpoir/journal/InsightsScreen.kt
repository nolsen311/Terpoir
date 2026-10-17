// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedCard
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.InsightData
import app.terpoir.core.InsightFeedback
import app.terpoir.core.SessionData
import app.terpoir.core.ShopInsightData
import app.terpoir.core.TimelineInsightData
import java.util.Locale
import kotlinx.coroutines.launch

/**
 * Insights (docs/design-decisions.md, "Insights engine"): plain averages of
 * the user's own ratings, computed in the core only while insights are on.
 * Each shows its evidence and likely confounders, opens its sessions, and
 * can be dismissed, marked wrong, or muted by category. Never advice.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun InsightsScreen(repo: JournalRepository, onOpenSession: (String) -> Unit, onClose: () -> Unit) {
    val scope = rememberCoroutineScope()
    var insights by remember { mutableStateOf<List<InsightData>?>(null) }
    var shops by remember { mutableStateOf<List<ShopInsightData>>(emptyList()) }
    var timelines by remember { mutableStateOf<List<TimelineInsightData>>(emptyList()) }
    var sessions by remember { mutableStateOf<Map<String, SessionData>>(emptyMap()) }
    var expanded by remember { mutableStateOf<String?>(null) }
    var revision by remember { mutableIntStateOf(0) }
    LaunchedEffect(revision) {
        insights = repo.insights()
        shops = repo.shopInsights()
        timelines = repo.timelineInsights()
        sessions = repo.sessions().associate { it.id to it.data }
    }
    fun feedback(key: String, kind: InsightFeedback) {
        scope.launch {
            repo.setInsightFeedback(key, kind)
            revision++
        }
    }

    EditorScaffold(title = stringResource(R.string.insights_title), onClose = onClose, onSave = null) {
        Text(stringResource(R.string.insights_explainer), style = MaterialTheme.typography.bodySmall)
        Text(stringResource(R.string.health_reminder), style = MaterialTheme.typography.bodySmall)
        val list = insights ?: return@EditorScaffold
        if (list.isEmpty()) Text(stringResource(R.string.insights_none), style = MaterialTheme.typography.bodyMedium)
        for (insight in list) {
            OutlinedCard(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(stringResource(R.string.insights_suggest), style = MaterialTheme.typography.labelMedium)
                    Text(
                        stringResource(
                            R.string.insights_finding,
                            subjectPhrase(insight),
                            stars(insight.withAvgMilli),
                            stars(insight.withoutAvgMilli),
                        ),
                        style = MaterialTheme.typography.bodyLarge,
                    )
                    Text(
                        pluralStringResource(
                            R.plurals.insights_evidence,
                            insight.withSessions.size,
                            insight.withSessions.size,
                            insight.withoutCount.toInt(),
                        ),
                        style = MaterialTheme.typography.bodySmall,
                    )
                    if (insight.fromStrainProfiles > 0u) {
                        Text(
                            pluralStringResource(
                                R.plurals.insights_from_strain_profiles,
                                insight.fromStrainProfiles.toInt(),
                                insight.fromStrainProfiles.toInt(),
                            ),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.tertiary,
                        )
                    }
                    if (insight.confounders.isNotEmpty()) {
                        val also = insight.confounders.map { stringResource(confounderLabel(it)) }.joinToString(", ")
                        Text(stringResource(R.string.insights_confounders, also), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.tertiary)
                    }
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        TextButton(onClick = { expanded = if (expanded == insight.key) null else insight.key }) {
                            Text(stringResource(R.string.insights_sessions))
                        }
                        TextButton(onClick = { feedback(insight.key, InsightFeedback.DISMISSED) }) { Text(stringResource(R.string.insights_dismiss)) }
                        TextButton(onClick = { feedback(insight.key, InsightFeedback.WRONG) }) { Text(stringResource(R.string.insights_wrong)) }
                        TextButton(onClick = { feedback(insight.category, InsightFeedback.MUTED) }) {
                            Text(stringResource(R.string.insights_mute, stringResource(categoryLabel(insight.category))))
                        }
                    }
                    if (expanded == insight.key) {
                        for (id in insight.withSessions) {
                            val s = sessions[id] ?: continue
                            TextButton(onClick = { onOpenSession(id) }) {
                                Text(listOfNotNull(formatWhen(s.`when`.unixMs, s.`when`.utcOffsetMin), s.rating?.let { "${it}★" }).joinToString(" · "))
                            }
                        }
                    }
                }
            }
        }
        if (shops.isNotEmpty()) {
            SectionTitle(stringResource(R.string.insights_shops))
            Text(stringResource(R.string.insights_shops_explainer), style = MaterialTheme.typography.bodySmall)
        }
        for (shop in shops) {
            OutlinedCard(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(shop.name, style = MaterialTheme.typography.titleSmall)
                    val parts = listOfNotNull(
                        shop.avgRatingMilli?.let { stringResource(R.string.insights_shop_rating, stars(it)) },
                        shop.hitRatePercent?.let { stringResource(R.string.insights_shop_hits, it.toInt()) },
                    )
                    Text(
                        (parts + pluralStringResource(R.plurals.insights_shop_sessions, shop.sessions.size, shop.sessions.size)).joinToString(" · "),
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    shop.avgAgeDays?.let {
                        Text(pluralStringResource(R.plurals.insights_shop_age, it.toInt(), it.toInt()), style = MaterialTheme.typography.bodySmall)
                    }
                    for (effect in shop.effects) {
                        val subject = when (effect.level) {
                            "brand" -> R.string.insights_shop_level_brand
                            "strain" -> R.string.insights_shop_level_strain
                            "lineage" -> R.string.insights_shop_level_lineage
                            "terpene" -> R.string.insights_shop_level_terpene
                            else -> R.string.insights_shop_level_product
                        }
                        // A built-in terpene comes as its slug; a custom one by name.
                        val name = if (effect.level == "terpene" && !effect.subject.startsWith("custom:")) label(effect.label) else effect.label
                        Text(
                            stringResource(
                                R.string.insights_shop_effect,
                                stringResource(subject, name),
                                stars(effect.hereAvgMilli),
                                stars(effect.elsewhereAvgMilli),
                                pluralStringResource(
                                    R.plurals.insights_shop_effect_evidence,
                                    effect.hereSessions.toInt(),
                                    effect.hereSessions.toInt(),
                                    effect.elsewhereSessions.toInt(),
                                ),
                            ),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.tertiary,
                        )
                    }
                    TextButton(onClick = { expanded = if (expanded == shop.spot) null else shop.spot }) {
                        Text(stringResource(R.string.insights_sessions))
                    }
                    if (expanded == shop.spot) {
                        for (id in shop.sessions) {
                            val s = sessions[id] ?: continue
                            TextButton(onClick = { onOpenSession(id) }) {
                                Text(listOfNotNull(formatWhen(s.`when`.unixMs, s.`when`.utcOffsetMin), s.rating?.let { "${it}★" }).joinToString(" · "))
                            }
                        }
                    }
                }
            }
        }
        // "How it usually goes" (the session timeline's insights): typical
        // timelines from the points the user set. The core returns none
        // while the timeline is hidden in What I track.
        if (timelines.isNotEmpty()) {
            SectionTitle(stringResource(R.string.insights_timelines))
            Text(stringResource(R.string.insights_timelines_explainer), style = MaterialTheme.typography.bodySmall)
        }
        val levelNames = stringArrayResource(R.array.timeline_levels)
        for (t in timelines) {
            val key = "timeline:${t.category}:${t.subject}"
            OutlinedCard(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(if (t.category == "method") label(t.name) else t.name, style = MaterialTheme.typography.titleSmall)
                    val parts = listOfNotNull(
                        t.onsetMin?.let { stringResource(R.string.insights_timeline_onset, durationLabel(it.toInt())) },
                        stringResource(
                            R.string.insights_timeline_peak,
                            durationLabel(t.peakMin.toInt()),
                            levelNames.getOrElse(t.peakLevel.toInt()) { "" },
                        ),
                        t.easesMin?.let { stringResource(R.string.insights_timeline_eases, durationLabel(it.toInt())) },
                    )
                    Text(parts.joinToString(", "), style = MaterialTheme.typography.bodyLarge)
                    Text(
                        pluralStringResource(R.plurals.insights_timeline_sessions, t.sessions.size, t.sessions.size),
                        style = MaterialTheme.typography.bodySmall,
                    )
                    TextButton(onClick = { expanded = if (expanded == key) null else key }) {
                        Text(stringResource(R.string.insights_sessions))
                    }
                    if (expanded == key) {
                        for (id in t.sessions) {
                            val s = sessions[id] ?: continue
                            TextButton(onClick = { onOpenSession(id) }) {
                                Text(listOfNotNull(formatWhen(s.`when`.unixMs, s.`when`.utcOffsetMin), s.rating?.let { "${it}★" }).joinToString(" · "))
                            }
                        }
                    }
                }
            }
        }
    }
}

/** What was compared, as the start of a sentence. */
@Composable
private fun subjectPhrase(insight: InsightData): String {
    val name = if (insight.custom) insight.subject else label(insight.subject)
    return when (insight.category) {
        "terpene" -> stringResource(R.string.insights_subject_terpene, name)
        "method" -> stringResource(R.string.insights_subject_method, name)
        "form" -> stringResource(R.string.insights_subject_form, name)
        "strain_type" -> stringResource(R.string.insights_subject_strain_type, name)
        // The shared parent's name, as the user wrote it.
        "lineage" -> stringResource(R.string.insights_subject_lineage, insight.subject)
        "time" -> stringResource(R.string.insights_subject_time, stringResource(timeLabel(insight.subject)))
        else -> name
    }
}

/** Thousandths of a star as "3.4". */
private fun stars(milli: Long): String = String.format(Locale.getDefault(), "%.1f", milli / 1000.0)

private fun timeLabel(slug: String): Int = when (slug) {
    "morning" -> R.string.insights_time_morning
    "afternoon" -> R.string.insights_time_afternoon
    "evening" -> R.string.insights_time_evening
    else -> R.string.insights_time_night
}

private fun confounderLabel(slug: String): Int = when (slug) {
    "method" -> R.string.insights_category_method
    "time" -> R.string.insights_category_time
    else -> R.string.insights_confounder_dose
}

private fun categoryLabel(slug: String): Int = when (slug) {
    "terpene" -> R.string.insights_category_terpene
    "method" -> R.string.insights_category_method
    "form" -> R.string.insights_category_form
    "strain_type" -> R.string.insights_category_strain_type
    "lineage" -> R.string.insights_category_lineage
    else -> R.string.insights_category_time
}
