// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.report

import java.io.IOException
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Reports say what kind of error happened and where, never its message. */
class ReportContentTest {
    private val secret = "Blue Dream at /storage/emulated/0/Spot 7"
    private val error = IllegalStateException(secret, IOException("also $secret"))

    @Test
    fun crashSummaryHasTypesAndLocationsButNoMessages() {
        val summary = CrashRecorder.summary(error)
        assertTrue(summary.startsWith("java.lang.IllegalStateException"))
        assertTrue(summary.contains("Caused by java.io.IOException"))
        assertTrue(summary.contains("at app.terpoir.report.ReportContentTest."))
        assertFalse(summary.contains("Blue Dream"))
        assertFalse(summary.contains("storage"))
    }

    @Test
    fun errorTypesNameClassesOnly() {
        val types = ProblemReport.errorTypes(error)
        assertTrue(types == "java.lang.IllegalStateException ← java.io.IOException")
    }

    @Test
    fun crashSummaryIsBounded() {
        fun deep(n: Int): Nothing = if (n == 0) throw RuntimeException(secret) else deep(n - 1)
        val summary = CrashRecorder.summary(runCatching { deep(500) }.exceptionOrNull()!!)
        assertTrue(summary.lines().count { it.trimStart().startsWith("at ") } <= 40)
    }
}
