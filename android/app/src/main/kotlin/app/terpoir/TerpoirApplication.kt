// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.app.Application
import app.terpoir.core.init as initCore
import app.terpoir.privacy.PrivacySettings
import app.terpoir.session.AutoLock
import app.terpoir.session.JournalFiles
import app.terpoir.session.JournalSession
import app.terpoir.session.LockSettings

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

    override fun onCreate() {
        super.onCreate()
        // Installs the core's panic hook (never reports panic messages).
        initCore()
        lockSettings = LockSettings(this)
        privacySettings = PrivacySettings(this)
        session = JournalSession(JournalFiles(noBackupFilesDir))
        AutoLock(this, session, lockSettings).install()
    }
}
