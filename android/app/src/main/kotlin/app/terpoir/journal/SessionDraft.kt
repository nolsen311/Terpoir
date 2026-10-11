// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.LocalTimeData
import app.terpoir.core.ProductLineData
import app.terpoir.core.SessionData
import app.terpoir.core.TagIntensity
import app.terpoir.core.TagRefData

/** One product line as the editor holds it. */
data class LineDraft(
    val lineId: String? = null,
    val product: String? = null,
    val method: String,
    val dose: String = "",
    val doseUnit: String? = null,
)

/** What is wrong with a draft, if anything. */
enum class DraftError { DOSE, TIMELINE }

/**
 * A session being edited. Fields the editor does not show yet (activity,
 * spot, symptoms, attachments) are carried over from the saved session
 * unchanged, so editing never drops them.
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
    val moodAfter: Int? = null,
    val rating: Int? = null,
    val wouldRepeat: Boolean? = null,
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
            ProductLineData(line.lineId, line.product, line.method, dose, line.doseUnit.takeIf { dose != null })
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
                spot = original?.spot,
                symptoms = original?.symptoms.orEmpty(),
                attachments = original?.attachments.orEmpty(),
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
        fun new(nowMs: Long, utcOffsetMin: Short): SessionDraft = SessionDraft(nowMs, utcOffsetMin)

        /** A saved session, for editing. */
        fun of(data: SessionData): SessionDraft = SessionDraft(
            whenMs = data.`when`.unixMs,
            utcOffsetMin = data.`when`.utcOffsetMin,
            lines = data.lines.map {
                LineDraft(it.lineId, it.product, it.method, it.doseMilli?.let(::formatMilli).orEmpty(), it.doseUnit)
            },
            intention = data.intention,
            social = data.social,
            moodBefore = data.moodBefore?.toInt(),
            effects = data.effects.associate { it.tag to it.value.toInt() },
            onset = data.onsetMin?.toString().orEmpty(),
            peak = data.peakMin?.toString().orEmpty(),
            duration = data.durationMin?.toString().orEmpty(),
            moodAfter = data.moodAfter?.toInt(),
            rating = data.rating?.toInt(),
            wouldRepeat = data.wouldRepeat,
            original = data,
        )
    }
}
