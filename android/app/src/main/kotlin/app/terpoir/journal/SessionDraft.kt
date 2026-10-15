// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.LocalTimeData
import app.terpoir.core.ProductLineData
import app.terpoir.core.SessionData
import app.terpoir.core.SymptomData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData
import app.terpoir.core.TimelineLevelData

/** One product line as the editor holds it. */
data class LineDraft(
    val lineId: String? = null,
    val product: String? = null,
    val method: String,
    val dose: String = "",
    val doseUnit: String? = null,
    /** The purchase this unit came from; not edited here yet, so carried over. */
    val purchaseId: String? = null,
    val purchaseLineId: String? = null,
    /** The shop it came from, picked when the product is bought at several. */
    val shop: String? = null,
)

/** What is wrong with a draft, if anything. */
enum class DraftError { DOSE, TIMELINE }

/**
 * A session being edited. Fields the editor does not show yet (activity,
 * attachments) are carried over from the saved session unchanged, so
 * editing never drops them. Symptoms are carried the same way while the
 * targeted-symptoms section is hidden.
 */
data class SessionDraft(
    val whenMs: Long,
    val utcOffsetMin: Short,
    val lines: List<LineDraft> = emptyList(),
    val intention: String? = null,
    val social: String? = null,
    val moodBefore: Int? = null,
    /** Effect → intensity 0–5 (0: noted as not felt). Absent: not recorded. */
    val effects: Map<TagRefData, Int> = emptyMap(),
    val onset: String = "",
    val peak: String = "",
    val duration: String = "",
    /** Timeline: minutes after consumption → level 0–5, only the points the user set. */
    val levels: Map<Int, Int> = emptyMap(),
    /** When [levels] last changed, in Unix milliseconds. */
    val levelsUpdatedMs: Long? = null,
    val moodAfter: Int? = null,
    val rating: Int? = null,
    val wouldRepeat: Boolean? = null,
    /** The spot it happened at, if recorded. */
    val spot: String? = null,
    /** Targeted symptom → result slug (`helped`, `did_not_help`, `not_rated`). */
    val symptoms: Map<TagRefData, String> = emptyMap(),
    private val original: SessionData? = null,
) {
    /** The session to save, or what is wrong. */
    fun toData(): Result {
        val lineData = lines.map { line ->
            val dose = when (val parsed = parseMilli(line.dose)) {
                Parsed.Empty -> null
                Parsed.Invalid -> return Result.Invalid(DraftError.DOSE)
                is Parsed.Milli -> parsed.value
            }
            if (dose != null && line.doseUnit == null) return Result.Invalid(DraftError.DOSE)
            ProductLineData(
                line.lineId,
                line.product,
                line.method,
                dose,
                line.doseUnit.takeIf { dose != null },
                line.purchaseId,
                line.purchaseLineId,
                line.shop,
            )
        }
        val minutes = listOf(onset, peak, duration).map {
            when (val parsed = parseMinutes(it)) {
                Parsed.Empty -> null
                Parsed.Invalid -> return Result.Invalid(DraftError.TIMELINE)
                is Parsed.Milli -> parsed.value.toUInt()
            }
        }
        return Result.Valid(
            SessionData(
                `when` = LocalTimeData(whenMs, utcOffsetMin),
                lines = lineData,
                activity = original?.activity,
                intention = intention,
                moodBefore = moodBefore?.toUByte(),
                social = social,
                effects = effects.map { (tag, value) -> TagIntensity(tag, value.toLong()) },
                onsetMin = minutes[0],
                peakMin = minutes[1],
                durationMin = minutes[2],
                moodAfter = moodAfter?.toUByte(),
                rating = rating?.toUByte(),
                wouldRepeat = wouldRepeat,
                spot = spot,
                symptoms = symptoms.map { (tag, result) -> SymptomData(tag, result) },
                attachments = original?.attachments.orEmpty(),
                timelineLevels = levels.toSortedMap().map { (minute, level) -> TimelineLevelData(minute.toUInt(), level.toUByte()) },
                timelineUpdatedMs = levelsUpdatedMs,
            ),
        )
    }

    /** The outcome of [toData]. */
    sealed interface Result {
        data class Valid(val data: SessionData) : Result

        data class Invalid(val error: DraftError) : Result
    }

    companion object {
        /** A new session, now, in the device's time zone. */
        fun new(nowMs: Long, utcOffsetMin: Short, spot: String? = null): SessionDraft =
            SessionDraft(nowMs, utcOffsetMin, spot = spot)

        /** A saved session, for editing. */
        fun of(data: SessionData): SessionDraft = SessionDraft(
            whenMs = data.`when`.unixMs,
            utcOffsetMin = data.`when`.utcOffsetMin,
            lines = data.lines.map {
                LineDraft(
                    it.lineId,
                    it.product,
                    it.method,
                    it.doseMilli?.let(::formatMilli).orEmpty(),
                    it.doseUnit,
                    it.purchaseId,
                    it.purchaseLineId,
                    it.shop,
                )
            },
            intention = data.intention,
            social = data.social,
            moodBefore = data.moodBefore?.toInt(),
            effects = data.effects.associate { it.tag to it.value.toInt() },
            onset = data.onsetMin?.toString().orEmpty(),
            peak = data.peakMin?.toString().orEmpty(),
            duration = data.durationMin?.toString().orEmpty(),
            levels = data.timelineLevels.associate { it.minute.toInt() to it.level.toInt() },
            levelsUpdatedMs = data.timelineUpdatedMs,
            moodAfter = data.moodAfter?.toInt(),
            rating = data.rating?.toInt(),
            wouldRepeat = data.wouldRepeat,
            spot = data.spot,
            symptoms = data.symptoms.associate { it.tag to it.result },
            original = data,
        )
    }
}
