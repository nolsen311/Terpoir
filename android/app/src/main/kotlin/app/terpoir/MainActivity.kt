// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.os.Bundle
import android.security.keystore.KeyPermanentlyInvalidatedException
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import app.terpoir.core.coreVersion
import app.terpoir.core.vocabulary
import app.terpoir.journal.JournalHome
import app.terpoir.journal.JournalRepository
import app.terpoir.journal.ScanServices
import app.terpoir.onboarding.AgeGate
import app.terpoir.onboarding.OnboardingStep
import app.terpoir.onboarding.RecoveryKeyBackup
import app.terpoir.privacy.PrivacyCenterScreen
import app.terpoir.privacy.PrivateTextInput
import app.terpoir.privacy.ScreenPrivacy
import app.terpoir.scan.Scanner
import app.terpoir.security.UserAuthentication
import app.terpoir.session.JournalSession
import app.terpoir.ui.theme.TerpoirTheme
import javax.crypto.Cipher

/** The app's only activity. UI only: data, crypto and sync live in the Rust core. */
class MainActivity : ComponentActivity() {
    private val app get() = application as TerpoirApplication

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
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
                val scan = remember { ScanServices(Scanner(app)) { app.privacySettings.coaLinks } }
                JournalHome(repo, vocab, scan, app.privacySettings, onLock = session::lock) { onClose ->
                    PrivacyCenterScreen(
                        center = app.privacyCenter,
                        onChanged = { ScreenPrivacy.onResume(this, app.privacySettings.screenProtection) },
                        onClose = onClose,
                    )
                }
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
