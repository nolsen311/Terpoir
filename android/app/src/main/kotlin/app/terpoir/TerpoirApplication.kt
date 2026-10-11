// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.app.Application
import android.content.Context
import android.content.pm.ApplicationInfo
import app.terpoir.alerts.CheckIns
import app.terpoir.alerts.SpotAlerts
import app.terpoir.core.init as initCore
import app.terpoir.onboarding.Onboarding
import app.terpoir.privacy.PrivacyCenter
import app.terpoir.privacy.PrivacySettings
import app.terpoir.privacy.buildPrivacyCenter
import app.terpoir.session.AutoLock
import app.terpoir.session.JournalFiles
import app.terpoir.session.JournalSession
import app.terpoir.session.LockSettings
import java.io.OutputStream
import java.io.PrintStream

/** Process-wide state: the journal session and auto-lock. */
class TerpoirApplication : Application() {
    /** The journal session. */
    lateinit var session: JournalSession
        private set

    /** Lock preferences. */
    lateinit var lockSettings: LockSettings
        private set

    /** Privacy toggles readable before unlock. */
    lateinit var privacySettings: PrivacySettings
        private set

    /** "Notify me" geofences and alerts. */
    lateinit var spotAlerts: SpotAlerts
        private set

    /** Follow-up check-in reminders. */
    lateinit var checkIns: CheckIns
        private set

    /** Every privacy toggle, each with its on- and off-action. */
    lateinit var privacyCenter: PrivacyCenter
        private set

    /** First-run progress. */
    lateinit var onboarding: Onboarding
        private set

    override fun attachBaseContext(base: Context) {
        super.attachBaseContext(base)
        Distribution.attachBaseContext(this)
    }

    override fun onCreate() {
        super.onCreate()
        if (applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE == 0) silenceConsole()
        // Installs the core's panic hook (never reports panic messages).
        initCore()
        lockSettings = LockSettings(this)
        privacySettings = PrivacySettings(this)
        spotAlerts = SpotAlerts(this, privacySettings, Distribution.geofencer(this))
        checkIns = CheckIns(this, privacySettings)
        privacyCenter = buildPrivacyCenter(this, privacySettings, spotAlerts, checkIns) {
            (session.state as? JournalSession.State.Unlocked)?.journal
        }
        session = JournalSession(JournalFiles(noBackupFilesDir))
        onboarding = Onboarding(this)
        // The process died after the journal was created but before its
        // recovery key was backed up or skipped. The key lived only in that
        // process, and setup blocks the journal until the backup is
        // resolved, so the journal is still empty: discard it and start
        // setup again rather than keep a journal with no usable recovery key.
        if (onboarding.backupPending) {
            if (session.state != JournalSession.State.NoJournal) session.reset()
            onboarding.discarded()
        }
        AutoLock(this, session, lockSettings).install()
    }

    /**
     * Backstop for release builds, which log nothing: R8 already strips
     * logging and console printing (proguard-rules.pro), and this catches
     * anything that writes to System.out or System.err some other way, which
     * Android would otherwise send to logcat.
     */
    private fun silenceConsole() {
        val nothing = PrintStream(
            object : OutputStream() {
                override fun write(b: Int) = Unit

                override fun write(b: ByteArray, off: Int, len: Int) = Unit
            },
        )
        System.setOut(nothing)
        System.setErr(nothing)
    }
}
