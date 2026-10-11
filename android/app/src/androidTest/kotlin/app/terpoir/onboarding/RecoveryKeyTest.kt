// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.terpoir.core.createKeys
import app.terpoir.session.JournalFiles
import app.terpoir.session.JournalSession
import app.terpoir.session.JournalSession.RecoveryCheck
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Backup confirmation checks the typed key against the journal through the core. */
@RunWith(AndroidJUnit4::class)
class RecoveryKeyTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val dir = context.cacheDir.resolve("recovery-test").apply { deleteRecursively() }
    private val files = JournalFiles(dir)
    private val keys = createKeys().also {
        files.masterWrappedByRecovery.parentFile?.mkdirs()
        files.masterWrappedByRecovery.writeBytes(it.masterWrappedByRecovery)
        it.masterKey.fill(0)
    }
    private val session = JournalSession(files)

    @After
    fun cleanUp() {
        dir.deleteRecursively()
        context.getSharedPreferences("onboarding", android.content.Context.MODE_PRIVATE).edit().clear().commit()
    }

    @Test
    fun theJournalsKeyMatches() {
        assertEquals(RecoveryCheck.MATCHES, session.checkRecoveryKey(keys.recoveryKey))
    }

    @Test
    fun caseAndDashesDoNotMatter() {
        val typed = keys.recoveryKey.replace("-", "").lowercase()
        assertEquals(RecoveryCheck.MATCHES, session.checkRecoveryKey(typed))
    }

    @Test
    fun partialKeyIsIncomplete() {
        assertEquals(RecoveryCheck.INCOMPLETE, session.checkRecoveryKey(keys.recoveryKey.take(20)))
    }

    @Test
    fun typoIsCaught() {
        // A character inside the key (not the leading version bits, not a dash).
        val key = keys.recoveryKey
        val changed = key.substring(0, 10) + (if (key[10] == 'A') 'B' else 'A') + key.substring(11)
        assertEquals(RecoveryCheck.TYPO, session.checkRecoveryKey(changed))
    }

    @Test
    fun anotherJournalsKeyIsWrong() {
        val other = createKeys().also { it.masterKey.fill(0) }
        assertEquals(RecoveryCheck.WRONG_KEY, session.checkRecoveryKey(other.recoveryKey))
    }

    @Test
    fun onboardingKeepsTheKeyOnlyUntilTheBackupIsResolved() {
        val onboarding = Onboarding(context)
        onboarding.creating()
        onboarding.created(keys.recoveryKey)
        assertTrue(Onboarding(context).backupPending)
        onboarding.backupResolved(RecoveryBackup.CONFIRMED)
        assertNull(onboarding.recoveryKey)
        val reloaded = Onboarding(context)
        assertFalse(reloaded.backupPending)
        assertEquals(RecoveryBackup.CONFIRMED, reloaded.backup)
        assertNull(reloaded.recoveryKey)
    }
}
