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
import org.junit.Assert.assertEquals
import org.junit.Test

class SessionDraftTest {
    private val relaxed = TagRefData.Builtin("relaxed")

    private val saved = SessionData(
        `when` = LocalTimeData(1_760_000_000_000, -420),
        lines = listOf(ProductLineData("ab".repeat(16), "cd".repeat(16), "smoked", 2000, "hits", "ef".repeat(16), "01".repeat(16))),
        activity = "walk",
        intention = "relax",
        moodBefore = 3u,
        social = "solo",
        effects = listOf(TagIntensity(relaxed, 4), TagIntensity(TagRefData.Builtin("dry_mouth"), 0)),
        onsetMin = 5u,
        peakMin = null,
        durationMin = 120u,
        moodAfter = 4u,
        rating = 5u,
        wouldRepeat = true,
        spot = "ef".repeat(16),
        symptoms = listOf(SymptomData(TagRefData.Builtin("sleep"), "helped")),
        attachments = listOf("01".repeat(16)),
    )

    @Test
    fun anUnchangedEditSavesTheSameSession() {
        val result = SessionDraft.of(saved).toData()
        assertEquals(SessionDraft.Result.Valid(saved), result)
    }

    @Test
    fun fieldsTheEditorDoesNotShowAreKept() {
        val edited = SessionDraft.of(saved).copy(rating = 3).toData() as SessionDraft.Result.Valid
        assertEquals("walk", edited.data.activity)
        assertEquals(saved.spot, edited.data.spot)
        assertEquals(saved.symptoms, edited.data.symptoms)
        assertEquals(saved.attachments, edited.data.attachments)
        assertEquals(3.toUByte(), edited.data.rating)
    }

    @Test
    fun aBadDoseOrTimelineIsReported() {
        val draft = SessionDraft.new(0, 0)
        val badDose = draft.copy(lines = listOf(LineDraft(method = "smoked", dose = "two", doseUnit = "hits")))
        assertEquals(SessionDraft.Result.Invalid(DraftError.DOSE), badDose.toData())
        val noUnit = draft.copy(lines = listOf(LineDraft(method = "smoked", dose = "2", doseUnit = null)))
        assertEquals(SessionDraft.Result.Invalid(DraftError.DOSE), noUnit.toData())
        assertEquals(SessionDraft.Result.Invalid(DraftError.TIMELINE), draft.copy(onset = "soon").toData())
    }

    @Test
    fun anEmptyDoseDropsTheUnit() {
        val draft = SessionDraft.new(0, 0).copy(lines = listOf(LineDraft(method = "edible", dose = "", doseUnit = "mg")))
        val line = (draft.toData() as SessionDraft.Result.Valid).data.lines.single()
        assertEquals(null, line.doseMilli)
        assertEquals(null, line.doseUnit)
    }
}
