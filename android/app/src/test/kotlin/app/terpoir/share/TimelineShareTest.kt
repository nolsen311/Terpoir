// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.share

import app.terpoir.core.LocalTimeData
import app.terpoir.core.SessionData
import app.terpoir.core.SymptomData
import app.terpoir.core.TagRefData
import app.terpoir.core.TimelineLevelData
import app.terpoir.journal.timelineShareValue
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class TimelineShareTest {
    private fun minutes(m: Int) = if (m < 60) "$m min" else if (m % 60 == 0) "${m / 60} h" else "${m / 60} h ${m % 60}"

    private val plain = SessionData(
        `when` = LocalTimeData(1_760_000_000_000, -420),
        lines = emptyList(),
        activity = null,
        intention = null,
        moodBefore = null,
        social = null,
        effects = emptyList(),
        onsetMin = null,
        peakMin = null,
        durationMin = null,
        moodAfter = null,
        rating = null,
        wouldRepeat = null,
        spot = null,
        symptoms = emptyList(),
        attachments = emptyList(),
    )

    @Test
    fun onlyTheSetPointsShowEarliestFirst() {
        assertEquals(
            "20 min 2/5 · 40 min 4/5 · 1 h 3/5 · 1 h 30 0/5",
            timelineShareValue(mapOf(60 to 3, 20 to 2, 90 to 0, 40 to 4), ::minutes),
        )
        assertEquals("30 min 5/5", timelineShareValue(mapOf(30 to 5), ::minutes))
        assertEquals("", timelineShareValue(emptyMap(), ::minutes))
    }

    @Test
    fun theTimelineStartsExcluded() {
        assertFalse(SESSION_OPTIONS.single { it.id == "timeline" }.default)
    }

    @Test
    fun theTimelineIsOfferedOnlyWithSomethingToShow() {
        fun offered(session: SessionData) = sessionOptions(session, symptomsOn = false).any { it.id == "timeline" }
        assertFalse(offered(plain))
        assertTrue(offered(plain.copy(timelineLevels = listOf(TimelineLevelData(20u, 3u)))))
        // An older session's onset, peak or duration still counts.
        assertTrue(offered(plain.copy(onsetMin = 5u)))
    }

    @Test
    fun symptomsStillFollowTheirSection() {
        val withSymptoms = plain.copy(symptoms = listOf(SymptomData(TagRefData.Builtin("sleep"), "helped")))
        assertFalse(sessionOptions(withSymptoms, symptomsOn = false).any { it.id == "symptoms" })
        assertTrue(sessionOptions(withSymptoms, symptomsOn = true).any { it.id == "symptoms" })
    }
}
