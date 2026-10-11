// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.terpoir.core.Journal
import app.terpoir.core.createKeys
import app.terpoir.security.MasterKeyVault
import java.io.File
import javax.crypto.Cipher

/** Where the journal lives: app-private, no-backup storage. */
class JournalFiles(noBackupDir: File) {
    private val root = noBackupDir.resolve("journal")

    /** The SQLCipher database. */
    val database: File = root.resolve("journal.db")

    /** Encrypted attachments. */
    val attachments: File = root.resolve("files")

    /** The master key wrapped by the Keystore key. */
    val wrappedMasterKey: File = root.resolve("keys/master.keystore")

    /** The master key wrapped by the recovery key. */
    val masterWrappedByRecovery: File = root.resolve("keys/master.recovery")

    /** The database key wrapped by the master key. */
    val wrappedDatabaseKey: File = root.resolve("keys/database")
}

/**
 * Whether the journal is open, and the open [Journal] if so. The master key
 * is never held here: it goes straight into the core and the array is
 * wiped.
 */
class JournalSession(private val files: JournalFiles) {
    /** Lock state. */
    sealed interface State {
        /** No journal on this device yet. */
        data object NoJournal : State

        /** A journal exists; unlocking needs the user. */
        data object Locked : State

        /** Open. */
        class Unlocked(val journal: Journal) : State
    }

    /** The vault holding the wrapped master key. */
    val vault = MasterKeyVault(files.wrappedMasterKey)

    /** Current state, observable from Compose. */
    var state: State by mutableStateOf(if (vault.exists()) State.Locked else State.NoJournal)
        private set

    /** False while the app is in the background: journal content is hidden. */
    var contentVisible: Boolean by mutableStateOf(true)
        private set

    /**
     * Creates a new journal and unlocks it, wrapping the master key with an
     * [MasterKeyVault.encryptCipher] the user has just authenticated.
     *
     * Interim: the recovery key is not shown or kept yet. Onboarding
     * (roadmap Phase 2) adds recovery-key backup and confirmation; until
     * then a journal created here cannot be restored on another device.
     */
    fun create(authenticated: Cipher) {
        val keys = createKeys()
        try {
            files.masterWrappedByRecovery.parentFile?.mkdirs()
            files.masterWrappedByRecovery.writeBytes(keys.masterWrappedByRecovery)
            files.wrappedDatabaseKey.writeBytes(keys.wrappedDatabaseKey)
            vault.store(authenticated, keys.masterKey)
            open(keys.masterKey.copyOf())
        } finally {
            keys.masterKey.fill(0)
        }
    }

    /** Unlocks with a [MasterKeyVault.decryptCipher] the user has just authenticated. */
    fun unlock(authenticated: Cipher) {
        open(vault.unwrap(authenticated))
    }

    /** Opens the journal; always wipes [masterKey]. */
    private fun open(masterKey: ByteArray) {
        try {
            val journal = Journal.open(
                files.database.path,
                files.attachments.path,
                masterKey,
                files.wrappedDatabaseKey.readBytes(),
            )
            state = State.Unlocked(journal)
            contentVisible = true
        } finally {
            masterKey.fill(0)
        }
    }

    /** Wipes the master key from memory and closes the database. */
    fun lock() {
        val current = state
        if (current is State.Unlocked) {
            current.journal.lock()
            current.journal.close()
            state = State.Locked
        }
    }

    /** Hides journal content (app left the foreground). */
    fun hideContent() {
        contentVisible = false
    }

    /** Shows journal content again (app back in the foreground, still unlocked). */
    fun showContent() {
        contentVisible = true
    }
}
