// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.share

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.ProductData
import app.terpoir.core.SessionData
import app.terpoir.journal.DataPoint
import app.terpoir.journal.JournalRepository
import app.terpoir.journal.LocalTracking
import app.terpoir.journal.Tracking
import app.terpoir.journal.durationLabel
import app.terpoir.journal.formatMilli
import app.terpoir.journal.formatWhen
import app.terpoir.journal.label
import app.terpoir.journal.productLabel
import app.terpoir.journal.tagLabel
import app.terpoir.journal.timelineShareValue
import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

/** What the user may add to a session share; the first four start included. */
internal val SESSION_OPTIONS = listOf(
    ShareOption("product", R.string.share_field_product, true),
    ShareOption("strain", R.string.share_field_strain, true),
    ShareOption("ratings", R.string.share_field_ratings, true),
    ShareOption("effects", R.string.share_field_effects, true),
    ShareOption("method", R.string.share_field_method, false),
    ShareOption("intention", R.string.share_field_intention, false),
    ShareOption("timeline", R.string.share_field_timeline, false),
    ShareOption("spot", R.string.share_field_spot, false),
    ShareOption("notes", R.string.share_field_notes, false),
)

/** Share options that show a data point the user can hide ("What I track"). */
private val OPTION_DATA_POINTS = mapOf(
    "strain" to DataPoint.STRAIN,
    "intention" to DataPoint.INTENTION,
    "timeline" to DataPoint.TIMELINE,
    "cannabinoids" to DataPoint.POTENCY,
    "terpenes" to DataPoint.TERPENES,
    "batch" to DataPoint.BATCH,
)

/** [options] without those whose data point is hidden. */
private fun List<ShareOption>.tracked(tracking: Tracking): List<ShareOption> =
    filter { option -> OPTION_DATA_POINTS[option.id]?.let(tracking::shows) ?: true }

/** Offered only while the targeted-symptoms section is on, and off by default. */
private val SYMPTOMS_OPTION = ShareOption("symptoms", R.string.share_field_symptoms, false)

/** How a symptom result reads in a share. */
private val SYMPTOM_RESULTS = mapOf(
    "helped" to R.string.symptom_helped,
    "did_not_help" to R.string.symptom_did_not_help,
    "not_rated" to R.string.symptom_not_rated,
)

/** How much of a session's time to share; none by default. */
private enum class DateShare(val label: Int) {
    NONE(R.string.share_date_none),
    DATE(R.string.share_date_only),
    DATE_TIME(R.string.share_date_time),
}

/** Everything a session share can draw on, loaded once. */
private class SessionSource(
    val session: SessionData,
    val products: List<String>,
    val strains: List<String>,
    val customNames: Map<String, String>,
    val spot: String?,
    val notes: List<String>,
    val symptomNames: Map<String, String>,
)

/**
 * Shares a saved session. Targeted symptoms are offered only while their
 * section is on ([symptomsOn]), and start excluded.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun SessionShare(repo: JournalRepository, id: String, symptomsOn: Boolean, onClose: () -> Unit) {
    val tracking = LocalTracking.current
    var source by remember { mutableStateOf<SessionSource?>(null) }
    var date by rememberSaveable { mutableStateOf(DateShare.NONE) }
    LaunchedEffect(id) {
        val session = repo.session(id) ?: return@LaunchedEffect onClose()
        val labels = repo.productLabels()
        val productIds = session.lines.mapNotNull { it.product }.distinct()
        val strainIds = productIds.flatMap { repo.product(it)?.strains.orEmpty() }.distinct()
        source = SessionSource(
            session = session,
            products = productIds.mapNotNull { labels[it] },
            strains = strainIds.mapNotNull { repo.strain(it)?.name },
            customNames = (repo.tags("effect")).associate { it.id to it.data.name },
            spot = session.spot?.let { repo.spot(it)?.name },
            notes = repo.sessionSnippets(id).map { it.data.text },
            symptomNames = repo.tags("symptom").associate { it.id to it.data.name },
        )
    }
    val loaded = source ?: return
    ShareScreen(
        options = sessionOptions(loaded.session, symptomsOn).tracked(tracking),
        onClose = onClose,
        extra = {
            Text(stringResource(R.string.share_field_date), style = MaterialTheme.typography.labelLarge)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (option in DateShare.entries) {
                    FilterChip(selected = date == option, onClick = { date = option }, label = { Text(stringResource(option.label)) })
                }
            }
        },
    ) { chosen, mark -> sessionShare(loaded, chosen, date, mark, showDose = tracking.shows(DataPoint.DOSE)) }
}

/**
 * The options a session offers: the timeline only when it has something
 * to show (points the user set, or an older onset, peak or duration), and
 * targeted symptoms only while their section is on and the session has some.
 */
internal fun sessionOptions(session: SessionData, symptomsOn: Boolean): List<ShareOption> {
    val timeline = session.timelineLevels.isNotEmpty() ||
        listOf(session.onsetMin, session.peakMin, session.durationMin).any { it != null }
    val options = if (timeline) SESSION_OPTIONS else SESSION_OPTIONS.filter { it.id != "timeline" }
    return if (symptomsOn && session.symptoms.isNotEmpty()) options + SYMPTOMS_OPTION else options
}

@Composable
private fun sessionShare(source: SessionSource, chosen: Set<String>, date: DateShare, mark: String?, showDose: Boolean): ShareText {
    val s = source.session
    val lines = buildList {
        when (date) {
            DateShare.NONE -> Unit
            DateShare.DATE -> add(ShareLine(stringResource(R.string.share_field_date), formatDay(s.`when`.unixMs, s.`when`.utcOffsetMin)))
            DateShare.DATE_TIME -> add(ShareLine(stringResource(R.string.share_field_date), formatWhen(s.`when`.unixMs, s.`when`.utcOffsetMin)))
        }
        if ("product" in chosen && source.products.isNotEmpty()) {
            add(ShareLine(stringResource(R.string.share_field_product), source.products.joinToString(", ")))
        }
        if ("strain" in chosen && source.strains.isNotEmpty()) {
            add(ShareLine(stringResource(R.string.share_field_strain), source.strains.joinToString(", ")))
        }
        if ("method" in chosen && s.lines.isNotEmpty()) {
            val how = s.lines.map { line ->
                val dose = line.doseMilli?.takeIf { showDose }?.let { "${formatMilli(it)} ${line.doseUnit?.let { u -> label(u) }.orEmpty()}".trim() }
                listOfNotNull(label(line.method), dose).joinToString(", ")
            }
            add(ShareLine(stringResource(R.string.share_field_method), how.joinToString("; ")))
        }
        if ("intention" in chosen) s.intention?.let { add(ShareLine(stringResource(R.string.share_field_intention), label(it))) }
        if ("ratings" in chosen) {
            val parts = listOfNotNull(
                s.rating?.let { stringResource(R.string.share_rating, it.toInt()) },
                s.wouldRepeat?.let { stringResource(if (it) R.string.share_would_repeat else R.string.share_would_not_repeat) },
            )
            if (parts.isNotEmpty()) add(ShareLine(stringResource(R.string.share_field_ratings), parts.joinToString(", ")))
        }
        if ("effects" in chosen && s.effects.isNotEmpty()) {
            val effects = s.effects.filter { it.value > 0 }.map { "${tagLabel(it.tag, source.customNames)} ${it.value}/5" }
            if (effects.isNotEmpty()) add(ShareLine(stringResource(R.string.share_field_effects), effects.joinToString(", ")))
        }
        if ("timeline" in chosen) {
            // The set points only; the chart's guesses are never shared.
            val labels = s.timelineLevels.associate { it.minute.toInt() to durationLabel(it.minute.toInt()) }
            val points = s.timelineLevels.associate { it.minute.toInt() to it.level.toInt() }
                .takeIf { it.isNotEmpty() }
                ?.let { levels -> timelineShareValue(levels) { labels.getValue(it) } }
            val older = listOfNotNull(
                s.onsetMin?.let { stringResource(R.string.share_onset, it.toInt()) },
                s.peakMin?.let { stringResource(R.string.share_peak, it.toInt()) },
                s.durationMin?.let { stringResource(R.string.share_duration, it.toInt()) },
            ).takeIf { it.isNotEmpty() }?.joinToString(", ")
            val parts = listOfNotNull(points, older)
            if (parts.isNotEmpty()) add(ShareLine(stringResource(R.string.share_field_timeline), parts.joinToString("; ")))
        }
        if ("spot" in chosen) source.spot?.let { add(ShareLine(stringResource(R.string.share_field_spot), it)) }
        if ("symptoms" in chosen && s.symptoms.isNotEmpty()) {
            val symptoms = s.symptoms.map { symptom ->
                val result = SYMPTOM_RESULTS[symptom.result]?.let { stringResource(it) } ?: symptom.result
                "${tagLabel(symptom.tag, source.symptomNames)}: $result"
            }
            add(ShareLine(stringResource(R.string.share_field_symptoms), symptoms.joinToString(", ")))
        }
        if ("notes" in chosen && source.notes.isNotEmpty()) {
            add(ShareLine(stringResource(R.string.share_field_notes), source.notes.joinToString("\n")))
        }
    }
    return ShareText(stringResource(R.string.share_session_title), lines, mark)
}

/** What the user may add to a product share; batch and package date start out. */
private val PRODUCT_OPTIONS = listOf(
    ShareOption("strain", R.string.share_field_strain, true),
    ShareOption("form", R.string.share_field_form, true),
    ShareOption("cannabinoids", R.string.share_field_cannabinoids, true),
    ShareOption("terpenes", R.string.share_field_terpenes, true),
    ShareOption("batch", R.string.share_field_batch, false),
)

private class ProductSource(val product: ProductData, val title: String, val strains: List<String>, val customNames: Map<String, String>)

/** Shares a saved product. */
@Composable
fun ProductShare(repo: JournalRepository, id: String, onClose: () -> Unit) {
    val tracking = LocalTracking.current
    var source by remember { mutableStateOf<ProductSource?>(null) }
    LaunchedEffect(id) {
        val product = repo.product(id) ?: return@LaunchedEffect onClose()
        val brand = product.brand?.takeIf { tracking.shows(DataPoint.BRAND) }?.let { repo.brand(it)?.name }
        source = ProductSource(
            product = product,
            title = productLabel(product.name, brand),
            strains = product.strains.mapNotNull { repo.strain(it)?.name },
            customNames = repo.tags("terpene").associate { it.id to it.data.name },
        )
    }
    val loaded = source ?: return
    ShareScreen(options = PRODUCT_OPTIONS.tracked(tracking), onClose = onClose) { chosen, mark ->
        val p = loaded.product
        val lines = buildList {
            if ("strain" in chosen && loaded.strains.isNotEmpty()) {
                add(ShareLine(stringResource(R.string.share_field_strain), loaded.strains.joinToString(", ")))
            }
            if ("form" in chosen) p.form?.let { add(ShareLine(stringResource(R.string.share_field_form), label(it))) }
            if ("cannabinoids" in chosen) {
                val amounts = listOfNotNull(
                    p.thcMilli?.let { "THC ${formatMilli(it)} ${p.thcUnit?.let { u -> label(u) }.orEmpty()}".trim() },
                    p.cbdMilli?.let { "CBD ${formatMilli(it)} ${p.cbdUnit?.let { u -> label(u) }.orEmpty()}".trim() },
                )
                if (amounts.isNotEmpty()) add(ShareLine(stringResource(R.string.share_field_cannabinoids), amounts.joinToString(", ")))
            }
            if ("terpenes" in chosen && p.terpenes.isNotEmpty()) {
                val terpenes = p.terpenes.map { "${tagLabel(it.tag, loaded.customNames)} ${formatMilli(it.value)}%" }
                add(ShareLine(stringResource(R.string.share_field_terpenes), terpenes.joinToString(", ")))
            }
            if ("batch" in chosen) p.batch?.let { add(ShareLine(stringResource(R.string.share_field_batch), it)) }
        }
        ShareText(loaded.title, lines, mark)
    }
}

/** "Oct 11, 2026" in the time zone the session was logged in. */
private fun formatDay(unixMs: Long, utcOffsetMin: Short): String =
    DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM)
        .format(Instant.ofEpochMilli(unixMs).atOffset(ZoneOffset.ofTotalSeconds(utcOffsetMin * 60)))
