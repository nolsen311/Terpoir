// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import android.annotation.SuppressLint
import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/** Where first-run setup stands. */
enum class OnboardingStep {
    /** Age gate: platform signal, then self-attestation. */
    AGE,

    /** Explain the journal and create it (keys, Keystore, database). */
    CREATE,

    /** Show the recovery key, then confirm the backup or skip. */
    BACKUP,

    /** Tour of the Privacy Center, everything at its most private. */
    PRIVACY_TOUR,

    /** Setup finished. */
    DONE,
}

/** How the recovery key was backed up. */
enum class RecoveryBackup {
    /** Not yet (or a journal from before onboarding existed). */
    NONE,

    /** The user typed the key back and it unlocked this journal. */
    CONFIRMED,

    /** The user chose "skip, I accept data loss". */
    SKIPPED,
}

/**
 * The step to show. Pure, so the rules are testable without a device.
 *
 * @param hasJournal a journal exists on this device.
 * @param backupPending a journal was created but its backup was neither
 *   confirmed nor skipped.
 */
fun onboardingStep(
    hasJournal: Boolean,
    ageGatePassed: Boolean,
    backupPending: Boolean,
    complete: Boolean,
): OnboardingStep = when {
    !hasJournal && !ageGatePassed -> OnboardingStep.AGE
    !hasJournal -> OnboardingStep.CREATE
    backupPending -> OnboardingStep.BACKUP
    !complete -> OnboardingStep.PRIVACY_TOUR
    else -> OnboardingStep.DONE
}

/**
 * First-run progress. The flags hold no journal content and are readable
 * before unlock; app data is excluded from backups. The recovery key
 * itself is never written anywhere: it lives in process memory only, from
 * journal creation until the backup is confirmed or skipped.
 */
class Onboarding(context: Context) {
    private val prefs = context.getSharedPreferences("onboarding", Context.MODE_PRIVATE)

    /**
     * The age gate passed: an adult platform signal, or the user attested
     * "I'm of legal age to use cannabis where I live". Only the pass is
     * stored, never the signal's category or a "no".
     */
    var ageGatePassed: Boolean by mutableStateOf(prefs.getBoolean(KEY_AGE, false))
        private set

    /** A journal exists whose recovery-key backup is unresolved. */
    var backupPending: Boolean by mutableStateOf(prefs.getBoolean(KEY_PENDING, false))
        private set

    /** How the recovery key was backed up. */
    var backup: RecoveryBackup by mutableStateOf(
        RecoveryBackup.entries.firstOrNull { it.name == prefs.getString(KEY_BACKUP, null) } ?: RecoveryBackup.NONE,
    )
        private set

    /** Setup finished, Privacy Center tour included. */
    var complete: Boolean by mutableStateOf(prefs.getBoolean(KEY_COMPLETE, false))
        private set

    /**
     * The new journal's recovery key, shown until the backup is resolved.
     * A JVM string cannot be wiped, so it is dropped as soon as possible.
     */
    var recoveryKey: String? by mutableStateOf(null)
        private set

    /** The step to show, given whether a journal exists. */
    fun step(hasJournal: Boolean): OnboardingStep = onboardingStep(hasJournal, ageGatePassed, backupPending, complete)

    /** The age gate passed. */
    fun passAgeGate() {
        prefs.edit().putBoolean(KEY_AGE, true).apply()
        ageGatePassed = true
    }

    /**
     * Call right before creating the journal. Written synchronously, so a
     * journal never exists without this flag until its backup is resolved.
     */
    @SuppressLint("ApplySharedPref") // Must be on disk before the journal is.
    fun creating() {
        prefs.edit().putBoolean(KEY_PENDING, true).commit()
        backupPending = true
    }

    /** The journal exists; keep its recovery key in memory for backup. */
    fun created(recoveryKey: String) {
        this.recoveryKey = recoveryKey
    }

    /**
     * Creation failed, or the app restarted before the backup was resolved
     * and the (still empty) journal was discarded.
     */
    fun discarded() {
        prefs.edit().putBoolean(KEY_PENDING, false).apply()
        backupPending = false
        recoveryKey = null
    }

    /**
     * The backup is confirmed or deliberately skipped. Written
     * synchronously: if the flag were lost in a crash, the next start would
     * discard the journal the user just secured.
     */
    @SuppressLint("ApplySharedPref")
    fun backupResolved(how: RecoveryBackup) {
        prefs.edit().putString(KEY_BACKUP, how.name).putBoolean(KEY_PENDING, false).commit()
        backup = how
        backupPending = false
        recoveryKey = null
    }

    /** The Privacy Center tour is done. */
    fun finish() {
        prefs.edit().putBoolean(KEY_COMPLETE, true).apply()
        complete = true
    }

    private companion object {
        const val KEY_AGE = "age_gate_passed"
        const val KEY_PENDING = "backup_pending"
        const val KEY_BACKUP = "recovery_backup"
        const val KEY_COMPLETE = "complete"
    }
}
