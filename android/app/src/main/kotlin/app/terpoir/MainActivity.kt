// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.content.Intent
import android.os.Bundle
import android.security.keystore.KeyPermanentlyInvalidatedException
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.alerts.CheckIns
import app.terpoir.alerts.PurchasePrompts
import app.terpoir.alerts.SpotAlerts
import app.terpoir.capture.QuickCapture
import app.terpoir.capture.ShortcutCard
import app.terpoir.core.ExportArchive
import app.terpoir.core.coreVersion
import app.terpoir.core.readExportArchive
import app.terpoir.core.vocabulary
import app.terpoir.journal.DataCard
import app.terpoir.journal.JournalHome
import app.terpoir.journal.JournalRepository
import app.terpoir.journal.ScanServices
import app.terpoir.onboarding.AgeGate
import app.terpoir.onboarding.OnboardingStep
import app.terpoir.onboarding.RecoveryBackup
import app.terpoir.onboarding.RecoveryKeyBackup
import app.terpoir.onboarding.RestoreScreen
import app.terpoir.privacy.ONLINE_MAP
import app.terpoir.sync.FolderSyncCard
import app.terpoir.privacy.PrivacyCenterScreen
import app.terpoir.privacy.PrivateTextInput
import app.terpoir.privacy.ScreenPrivacy
import app.terpoir.scan.Scanner
import app.terpoir.security.UserAuthentication
import app.terpoir.session.AutoLockCard
import app.terpoir.session.JournalSession
import app.terpoir.ui.theme.TerpoirTheme
import javax.crypto.Cipher
import kotlinx.coroutines.launch

/** The app's only activity. UI only: data, crypto and sync live in the Rust core. */
class MainActivity : ComponentActivity() {
    private val app get() = application as TerpoirApplication

    /** Opened from a "Notify me" alert: after unlock, show which spot is near. */
    private var nearbyRequested by mutableStateOf(false)

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        fromAlert(intent)
    }

    /** Opened from a "Quick check-in?" reminder: after unlock, open the check-in. */
    private var checkInRequested by mutableStateOf(false)

    /** Opened from a "Quick log?" purchase prompt: after unlock, open receipt scan. */
    private var quickLogRequested by mutableStateOf(false)

    /** Opened from a quick-capture shortcut, tile or widget: after unlock, a new note. */
    private var newNoteRequested by mutableStateOf(false)

    private fun fromAlert(intent: Intent?) {
        when (intent?.action) {
            SpotAlerts.ACTION_NEAR_SPOT -> nearbyRequested = true
            CheckIns.ACTION_CHECK_IN -> checkInRequested = true
            PurchasePrompts.ACTION_QUICK_LOG -> quickLogRequested = true
            QuickCapture.ACTION_NEW_NOTE -> newNoteRequested = true
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        fromAlert(intent)
        ScreenPrivacy.install(this)
        val core = coreVersion()
        // System bars follow the theme: light icons in dark mode and back.
        enableEdgeToEdge()
        setContent {
            PrivateTextInput {
                TerpoirTheme {
                    Surface(modifier = Modifier.fillMaxSize().safeDrawingPadding()) {
                        Screen(app.session, core)
                    }
                }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        ScreenPrivacy.onResume(this, app.privacySettings.screenProtection)
    }

    override fun onPause() {
        ScreenPrivacy.onPause(this)
        super.onPause()
    }

    @Composable
    private fun Screen(session: JournalSession, core: String) {
        var message by remember { mutableStateOf<Int?>(null) }
        // An export chosen for restoring, waiting for its recovery key.
        var restoring by remember { mutableStateOf<ExportArchive?>(null) }
        val pickExport = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
            if (uri != null) {
                restoring = try {
                    readExportArchive(checkNotNull(contentResolver.openInputStream(uri)).use { it.readBytes() })
                } catch (_: Exception) {
                    message = R.string.restore_not_export
                    null
                }
            }
        }
        val onboarding = app.onboarding
        val state = session.state
        val unlocked = state is JournalSession.State.Unlocked
        // Journal content, the recovery key included, is drawn only while
        // unlocked and in the foreground.
        val showContent = unlocked && session.contentVisible
        val step = onboarding.step(hasJournal = session.state != JournalSession.State.NoJournal)
        val recoveryKey = onboarding.recoveryKey
        when {
            step == OnboardingStep.AGE -> {
                AgeGate(Distribution.ageSignals, onAdult = onboarding::passAgeGate)
                return
            }
            showContent && step == OnboardingStep.BACKUP && recoveryKey != null -> {
                RecoveryKeyBackup(recoveryKey, session::checkRecoveryKey, onboarding::backupResolved)
                return
            }
            showContent && step == OnboardingStep.PRIVACY_TOUR -> {
                PrivacyCenterScreen(
                    center = app.privacyCenter,
                    onChanged = { ScreenPrivacy.onResume(this, app.privacySettings.screenProtection) },
                    onClose = onboarding::finish,
                    tour = true,
                )
                return
            }
            // Locking replaces the whole journal UI, editors included, so
            // nothing opened while unlocked survives a lock.
            showContent && step == OnboardingStep.DONE && state is JournalSession.State.Unlocked -> {
                val repo = remember(state.journal) { JournalRepository(state.journal) }
                val vocab = remember { vocabulary() }
                val scan = remember { ScanServices(Scanner { Distribution.textEngine(app, app.privacySettings) }) { app.privacySettings.coaLinks } }
                val scope = rememberCoroutineScope()
                // Each unlock re-registers "Notify me" geofences: they need the
                // spots' coordinates, which only the open journal has.
                LaunchedEffect(state.journal) {
                    app.spotAlerts.refresh(state.journal)
                    // Folder sync, on unlock: it runs only while the app is open.
                    if (app.privacySettings.folderSync) runCatching { app.folderSync.sync(state.journal) }
                }
                JournalHome(
                    repo,
                    vocab,
                    scan,
                    app.privacySettings,
                    app.spotAlerts,
                    app.checkIns,
                    mapToggle = app.privacyCenter.toggles.firstOrNull { it.id == ONLINE_MAP },
                    checkNearby = nearbyRequested,
                    checkInSession = if (checkInRequested) app.checkIns.pending() else null,
                    onCheckInOpened = {
                        checkInRequested = false
                        app.checkIns.done()
                    },
                    quickLog = quickLogRequested,
                    onQuickLogOpened = { quickLogRequested = false },
                    newNote = newNoteRequested,
                    onNewNoteOpened = { newNoteRequested = false },
                    onEdited = {
                        scope.launch {
                            app.spotAlerts.refresh(state.journal)
                            if (app.privacySettings.folderSync) runCatching { app.folderSync.sync(state.journal) }
                        }
                    },
                    onLock = session::lock,
                ) { onClose ->
                    PrivacyCenterScreen(
                        center = app.privacyCenter,
                        onChanged = { ScreenPrivacy.onResume(this, app.privacySettings.screenProtection) },
                        onClose = onClose,
                    ) {
                        AutoLockCard(app.lockSettings)
                        ShortcutCard(app.privacySettings)
                        DataCard(repo, session::masterWrappedByRecovery)
                        if (app.privacySettings.folderSync) FolderSyncCard(app.folderSync, state.journal)
                    }
                }
                return
            }
        }
        restoring?.let { archive ->
            if (state == JournalSession.State.NoJournal) {
                RestoreScreen(
                    check = { typed -> session.checkRecoveryKey(typed, archive.masterWrappedByRecovery) },
                    onRestore = { typed ->
                        message = restoreJournal(typed, archive) {
                            message = it
                            restoring = null
                        }
                    },
                    onCancel = { restoring = null },
                )
                return
            }
        }
        Column(
            modifier = Modifier.padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            when (state) {
                JournalSession.State.NoJournal -> {
                    Text(stringResource(R.string.setup_explainer))
                    Button(onClick = { message = createJournal { message = it } }) {
                        Text(stringResource(R.string.setup))
                    }
                    TextButton(onClick = { pickExport.launch(arrayOf("*/*")) }) {
                        Text(stringResource(R.string.restore_start))
                    }
                }
                JournalSession.State.Locked -> {
                    Text(stringResource(R.string.locked))
                    Button(onClick = { message = unlock() }) { Text(stringResource(R.string.unlock)) }
                }
                // Unlocked but in the background: journal content stays hidden.
                is JournalSession.State.Unlocked -> Unit
            }
            message?.let { Text(stringResource(it), color = MaterialTheme.colorScheme.error) }
            Text(stringResource(R.string.core_version, core), style = MaterialTheme.typography.bodySmall)
        }
    }

    /**
     * Starts journal creation; returns an error message to show, if any.
     * [onError] gets one if creation fails after authentication.
     */
    private fun createJournal(onError: (Int) -> Unit): Int? {
        val strict = app.lockSettings.strictBiometrics
        if (!UserAuthentication.isAvailable(this, strict)) return R.string.error_no_lock_screen
        val cipher = try {
            app.session.vault.createWrappingKey(strict)
            app.session.vault.encryptCipher()
        } catch (_: Exception) {
            return R.string.error_keystore
        }
        authenticate(cipher, getString(R.string.setup_prompt)) { authorized ->
            app.onboarding.creating()
            try {
                app.onboarding.created(app.session.create(authorized))
            } catch (_: Exception) {
                app.session.reset()
                app.onboarding.discarded()
                onError(R.string.error_keystore)
            }
        }
        return null
    }

    /**
     * Restores a journal from [archive] with the recovery key [typed] (already
     * checked against it), after the user authenticates; returns an error
     * message to show, if any. [onError] gets one if restoring fails after
     * authentication, which leaves no journal behind.
     */
    private fun restoreJournal(typed: String, archive: ExportArchive, onError: (Int) -> Unit): Int? {
        val strict = app.lockSettings.strictBiometrics
        if (!UserAuthentication.isAvailable(this, strict)) return R.string.error_no_lock_screen
        val cipher = try {
            app.session.vault.createWrappingKey(strict)
            app.session.vault.encryptCipher()
        } catch (_: Exception) {
            return R.string.error_keystore
        }
        authenticate(cipher, getString(R.string.setup_prompt)) { authorized ->
            try {
                app.session.restore(authorized, typed, archive)
                // The user just proved they hold the recovery key.
                app.onboarding.backupResolved(RecoveryBackup.CONFIRMED)
            } catch (_: Exception) {
                app.session.reset()
                onError(R.string.restore_failed)
            }
        }
        return null
    }

    /** Starts unlocking; returns an error message to show, if any. */
    private fun unlock(): Int? {
        val cipher = try {
            app.session.vault.decryptCipher()
        } catch (_: KeyPermanentlyInvalidatedException) {
            return R.string.error_key_invalidated
        } catch (_: Exception) {
            return R.string.error_keystore
        }
        authenticate(cipher, getString(R.string.unlock_prompt)) { app.session.unlock(it) }
        return null
    }

    private fun authenticate(cipher: Cipher, title: String, then: (Cipher) -> Unit) {
        UserAuthentication.authenticate(
            activity = this,
            cipher = cipher,
            title = title,
            cancel = getString(android.R.string.cancel),
            strict = app.lockSettings.strictBiometrics,
            onSuccess = then,
            onFailure = {},
        )
    }
}
