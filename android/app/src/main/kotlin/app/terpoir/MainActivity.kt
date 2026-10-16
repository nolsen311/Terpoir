// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.content.Intent
import android.content.res.Configuration
import android.net.Uri
import android.os.Bundle
import android.os.SystemClock
import android.security.keystore.KeyPermanentlyInvalidatedException
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.lifecycle.withResumed
import app.terpoir.alerts.CheckIns
import app.terpoir.alerts.PurchasePrompts
import app.terpoir.alerts.SpotAlerts
import app.terpoir.capture.LockedCaptureScreen
import app.terpoir.capture.QuickCapture
import app.terpoir.capture.ShortcutCard
import app.terpoir.core.CoreException
import app.terpoir.core.ExportArchive
import app.terpoir.core.checkExportArchive
import app.terpoir.core.readExportArchive
import app.terpoir.core.readableBlobs
import app.terpoir.core.vocabulary
import app.terpoir.journal.DataCard
import app.terpoir.journal.JournalHome
import app.terpoir.journal.JournalRepository
import app.terpoir.journal.ScanServices
import app.terpoir.map.MapProvider
import app.terpoir.map.MapProviderCard
import app.terpoir.onboarding.AgeGate
import app.terpoir.onboarding.BrandFrame
import app.terpoir.onboarding.BrandGreen
import app.terpoir.onboarding.ExistingJournal
import app.terpoir.onboarding.GoogleMapChoice
import app.terpoir.onboarding.JournalChoice
import app.terpoir.onboarding.OnboardingStep
import app.terpoir.onboarding.RecoveryBackup
import app.terpoir.onboarding.RecoveryKeyBackup
import app.terpoir.onboarding.RestoreScreen
import app.terpoir.privacy.FOLDER_SYNC
import app.terpoir.privacy.ONLINE_MAP
import app.terpoir.privacy.PrivacyCenterScreen
import app.terpoir.privacy.PrivacyGroup
import app.terpoir.privacy.PrivateTextInput
import app.terpoir.privacy.ScreenPrivacy
import app.terpoir.report.CrashRecorder
import app.terpoir.report.ProblemReport
import app.terpoir.report.ReportButton
import app.terpoir.scan.Scanner
import app.terpoir.security.UserAuthentication
import app.terpoir.session.AutoLockCard
import app.terpoir.session.HandOff
import app.terpoir.session.JournalSession
import app.terpoir.sync.FolderSyncCard
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

    /**
     * Opened from a quick-capture shortcut, tile or widget: a new note,
     * written while locked if the journal is locked, else after unlock.
     */
    private var newNoteRequested by mutableStateOf(false)

    /** Opened from a "New entry" shortcut, tile or widget: after unlock, a new entry. */
    private var newEntryRequested by mutableStateOf(false)

    /** A note was just written while locked: say so on the lock screen. */
    private var captureSaved by mutableStateOf(false)

    /**
     * Counts the times the app came into view (launch, or back from the
     * background). A locked journal asks for the unlock prompt by itself
     * once per count, so there's no "Unlock" tap first.
     */
    private var appearances by mutableIntStateOf(0)

    /** The appearance the unlock prompt was last shown for on its own. */
    private var promptedFor = 0

    /** An authentication prompt is showing. */
    private var prompting = false

    /** When the last prompt closed ([SystemClock.elapsedRealtime]). */
    private var promptClosedAt = 0L

    private fun fromAlert(intent: Intent?) {
        when (intent?.action) {
            SpotAlerts.ACTION_NEAR_SPOT -> nearbyRequested = true
            CheckIns.ACTION_CHECK_IN -> checkInRequested = true
            PurchasePrompts.ACTION_QUICK_LOG -> quickLogRequested = true
            QuickCapture.ACTION_NEW_ENTRY -> newEntryRequested = true
            QuickCapture.ACTION_NEW_NOTE -> {
                newNoteRequested = true
                captureSaved = false
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        fromAlert(intent)
        ScreenPrivacy.install(this)
        // System bars follow the theme: light icons in dark mode and back.
        systemBars(app.appearance.theme.isDark(systemDark()))
        setContent {
            val dark = app.appearance.theme.isDark(isSystemInDarkTheme())
            // The unlock screen is all brand green, under the system bars
            // too, so their icons are light there in either theme.
            val green = app.session.state == JournalSession.State.Locked
            LaunchedEffect(dark, green) { systemBars(dark || green) }
            PrivateTextInput {
                TerpoirTheme(darkTheme = dark) {
                    val page = if (green) BrandGreen else MaterialTheme.colorScheme.surface
                    Box(Modifier.fillMaxSize().background(page)) {
                        Surface(modifier = Modifier.fillMaxSize().safeDrawingPadding(), color = page) {
                            Screen(app.session)
                            CrashPrompt()
                        }
                    }
                }
            }
        }
    }

    /** Whether the phone (or, from Android 12, the app's night mode) is dark. */
    private fun systemDark(): Boolean =
        resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES

    /**
     * Edge to edge, with system bar icons that suit [dark]. Only through
     * androidx.activity: no bar colors, cutout modes or decor-fitting of our
     * own. The library still calls the bar-color setters deprecated in
     * Android 15 (no-ops there; it draws the scrim itself), which Play
     * reports (docs/design-decisions.md, "Edge to edge").
     */
    private fun systemBars(dark: Boolean) {
        val transparent = android.graphics.Color.TRANSPARENT
        // The scrims enableEdgeToEdge uses by default for the navigation bar.
        val lightScrim = android.graphics.Color.argb(0xe6, 0xFF, 0xFF, 0xFF)
        val darkScrim = android.graphics.Color.argb(0x80, 0x1b, 0x1b, 0x1b)
        enableEdgeToEdge(
            statusBarStyle = if (dark) SystemBarStyle.dark(transparent) else SystemBarStyle.light(transparent, transparent),
            navigationBarStyle = if (dark) SystemBarStyle.dark(darkScrim) else SystemBarStyle.light(lightScrim, darkScrim),
        )
    }

    override fun onStart() {
        super.onStart()
        // The prompt's own PIN screen can stop and restart this activity;
        // that isn't the user coming back, and counting it would show the
        // prompt again right after they cancelled it.
        if (!prompting && SystemClock.elapsedRealtime() - promptClosedAt > PROMPT_SETTLE_MILLIS) appearances++
    }

    override fun onResume() {
        super.onResume()
        HandOff.end()
        ScreenPrivacy.onResume(this, app.privacySettings.screenProtection)
    }

    override fun onPause() {
        ScreenPrivacy.onPause(this)
        super.onPause()
    }

    /**
     * After a crash: offers to report it by email, once. Sending or
     * dismissing forgets it (docs/design-decisions.md, "Problem reports").
     */
    @Composable
    private fun CrashPrompt() {
        var crash by remember { mutableStateOf(CrashRecorder.pending(this)) }
        val summary = crash ?: return
        fun done() {
            CrashRecorder.clear(this)
            crash = null
        }
        AlertDialog(
            onDismissRequest = ::done,
            title = { Text(stringResource(R.string.crash_title)) },
            text = { Text(stringResource(R.string.crash_body)) },
            confirmButton = {
                TextButton(onClick = {
                    val body = ProblemReport.body(this, "crash", details = summary)
                    ProblemReport.send(this, getString(R.string.report_subject, "crash"), body)
                    done()
                }) { Text(stringResource(R.string.crash_send)) }
            },
            dismissButton = { TextButton(onClick = ::done) { Text(stringResource(R.string.crash_dismiss)) } },
        )
    }

    @Composable
    private fun Screen(session: JournalSession) {
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
        // A sync folder chosen for restoring; its contents are in [restoring].
        var restoringFolder by remember { mutableStateOf<Uri?>(null) }
        // After the age gate: "I have a journal" was chosen.
        var existing by rememberSaveable { mutableStateOf(false) }
        val screenScope = rememberCoroutineScope()
        val pickFolder = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocumentTree()) { uri ->
            if (uri != null) {
                screenScope.launch {
                    val contents = runCatching { app.folderSync.readForRestore(uri) }.getOrNull()
                    val key = contents?.let { runCatching { readExportArchive(it.keyFile) }.getOrNull() }
                    if (contents == null || key == null) {
                        message = R.string.restore_not_folder
                    } else {
                        restoring = ExportArchive(key.masterWrappedByRecovery, contents.blobs)
                        restoringFolder = uri
                    }
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
                BrandFrame {
                    AgeGate(Distribution.ageSignals, onAdult = onboarding::passAgeGate)
                }
                return
            }
            showContent && step == OnboardingStep.BACKUP && recoveryKey != null -> {
                BrandFrame {
                    RecoveryKeyBackup(recoveryKey, session::checkRecoveryKey, onboarding::backupResolved)
                }
                return
            }
            showContent && step == OnboardingStep.MAP_CHOICE -> {
                val scope = rememberCoroutineScope()
                BrandFrame {
                    GoogleMapChoice(
                        onUseGoogle = {
                            scope.launch {
                                app.privacyCenter.toggles.firstOrNull { it.id == ONLINE_MAP }?.set(true)
                                app.privacySettings.mapProvider = MapProvider.GOOGLE
                                onboarding.mapChosen()
                            }
                        },
                        onNotNow = onboarding::mapChosen,
                    )
                }
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
                    captureSaved = false
                    app.spotAlerts.refresh(state.journal)
                    // Folder sync, on unlock: it runs only while the app is open.
                    if (app.privacySettings.folderSync) runCatching { app.folderSync.sync(state.journal) }
                }
                JournalHome(
                    repo,
                    vocab,
                    scan,
                    app.privacySettings,
                    app.appearance,
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
                    newEntry = newEntryRequested,
                    onNewEntryOpened = { newEntryRequested = false },
                    onEdited = {
                        scope.launch {
                            app.spotAlerts.refresh(state.journal)
                            if (app.privacySettings.folderSync) runCatching { app.folderSync.sync(state.journal) }
                        }
                    },
                ) { onClose, settings ->
                    PrivacyCenterScreen(
                        center = app.privacyCenter,
                        onChanged = { ScreenPrivacy.onResume(this, app.privacySettings.screenProtection) },
                        onClose = onClose,
                        settings = settings,
                        version = remember { runCatching { packageManager.getPackageInfo(packageName, 0).versionName }.getOrNull().orEmpty() },
                        sectionExtras = mapOf<PrivacyGroup, @Composable () -> Unit>(
                            PrivacyGroup.LOCATION to { MapProviderCard(repo, app.privacySettings) },
                            PrivacyGroup.OS_INTEGRATION to { ShortcutCard(app.privacySettings) },
                            PrivacyGroup.SYNC_AND_KEYS to {
                                DataCard(repo, session::masterWrappedByRecovery)
                                if (app.privacySettings.folderSync) FolderSyncCard(app.folderSync, state.journal)
                            },
                            PrivacyGroup.DEVICE_SECURITY to { AutoLockCard(app.lockSettings) },
                        ),
                    )
                }
                return
            }
        }
        // A quick-capture entry point while locked: write the note without
        // unlocking (write-only capture). It is sealed at once and joins
        // the journal, for review, at the next unlock.
        if (newNoteRequested && state == JournalSession.State.Locked && step == OnboardingStep.DONE && session.captureReady()) {
            LockedCaptureScreen(
                settings = app.privacySettings,
                onSave = { text ->
                    session.sealCapture(text).also { saved ->
                        if (saved) {
                            newNoteRequested = false
                            captureSaved = true
                        }
                    }
                },
                onUnlock = { message = unlock { message = it } },
                onClose = { newNoteRequested = false },
            )
            return
        }
        restoring?.let { archive ->
            if (state == JournalSession.State.NoJournal) {
                val folder = restoringFolder
                BrandFrame {
                    RestoreScreen(
                        explainer = if (folder == null) R.string.restore_explainer else R.string.restore_folder_explainer,
                        check = { typed -> session.checkRecoveryKey(typed, archive.masterWrappedByRecovery) },
                        onRestore = { typed ->
                            // From a folder: leave out blobs that don't open (one
                            // still being written, say); folder sync retries them.
                            val usable = if (folder == null) archive else runCatching { readableBlobs(typed, archive) }.getOrNull()
                            // Checked before anything changes: an export a newer
                            // app wrote is refused whole, never restored partly.
                            val check = usable?.let { runCatching { checkExportArchive(typed, it) }.getOrNull() }
                            message = when {
                                usable == null || check == null -> R.string.restore_failed
                                folder != null && usable.blobs.isEmpty() && archive.blobs.isNotEmpty() -> R.string.restore_folder_out_of_date
                                check.newerFormatBlobs > 0uL -> R.string.newer_version_needed
                                else -> null
                            }
                            if (usable == null || message != null) {
                                restoring = null
                                restoringFolder = null
                            } else {
                                message = restoreJournal(
                                    typed,
                                    usable,
                                    onRestored = {
                                        // Keep syncing through the folder the user restored from.
                                        if (folder != null) {
                                            app.folderSync.adopt(folder)
                                            screenScope.launch { app.privacyCenter.toggles.firstOrNull { it.id == FOLDER_SYNC }?.set(true) }
                                        }
                                    },
                                ) {
                                    message = it
                                    restoring = null
                                    restoringFolder = null
                                }
                            }
                        },
                        onCancel = {
                            restoring = null
                            restoringFolder = null
                        },
                    )
                }
                return
            }
        }
        Branded(state == JournalSession.State.NoJournal || state == JournalSession.State.Locked, filled = state == JournalSession.State.Locked) {
            Column(
                modifier = Modifier.fillMaxSize().padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                when (state) {
                    JournalSession.State.NoJournal -> if (!existing) {
                        JournalChoice(
                            onNew = { message = createJournal { message = it } },
                            onExisting = {
                                message = null
                                existing = true
                            },
                        )
                    } else {
                        ExistingJournal(
                            onExport = { pickExport.launch(arrayOf("*/*")) },
                            onFolder = { pickFolder.launch(null) },
                            onBack = {
                                message = null
                                existing = false
                            },
                        )
                    }
                    JournalSession.State.Locked -> {
                        // Straight to the unlock prompt, once each time the app
                        // comes into view. Cancelling leaves this screen, whose
                        // button asks again.
                        LaunchedEffect(appearances) {
                            if (appearances != promptedFor && !prompting) {
                                promptedFor = appearances
                                lifecycle.withResumed { message = unlock { message = it } }
                            }
                        }
                        Text(stringResource(R.string.locked))
                        Button(onClick = { message = unlock { message = it } }) { Text(stringResource(R.string.unlock)) }
                        if (captureSaved) Text(stringResource(R.string.capture_locked_saved))
                    }
                    // Unlocked but in the background: journal content stays hidden.
                    is JournalSession.State.Unlocked -> Unit
                }
                message?.let {
                    Text(stringResource(it), color = MaterialTheme.colorScheme.error)
                    // Failures with no step for the user to take; the others
                    // (wrong file, no screen lock, an update needed) say what to do.
                    // A Keystore failure's report names the exception types
                    // (nothing from the journal), to tell its causes apart.
                    if (it in REPORTABLE) ReportButton(it, keystoreError.takeIf { _ -> it == R.string.error_keystore })
                }
                Text(stringResource(R.string.settings_version, appVersion()), style = MaterialTheme.typography.bodySmall)
            }
        }
    }

    /** The last Keystore failure behind [R.string.error_keystore], for its problem report. */
    private var keystoreError: Throwable? = null

    /** Onboarding's first-run screens and the unlock screen sit under the brand header ([BrandFrame]). */
    @Composable
    private fun Branded(on: Boolean, filled: Boolean = false, content: @Composable () -> Unit) =
        if (on) BrandFrame(filled, content) else content()

    /** The app's version name ("0.5.1"), as on the Settings page. */
    private fun appVersion(): String =
        runCatching { packageManager.getPackageInfo(packageName, 0).versionName }.getOrNull().orEmpty()

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
        } catch (e: Exception) {
            keystoreError = e
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
    private fun restoreJournal(typed: String, archive: ExportArchive, onRestored: () -> Unit, onError: (Int) -> Unit): Int? {
        val strict = app.lockSettings.strictBiometrics
        if (!UserAuthentication.isAvailable(this, strict)) return R.string.error_no_lock_screen
        val cipher = try {
            app.session.vault.createWrappingKey(strict)
            app.session.vault.encryptCipher()
        } catch (e: Exception) {
            keystoreError = e
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
                return@authenticate
            }
            onRestored()
        }
        return null
    }

    /**
     * Starts unlocking; returns an error message to show, if any. [onError]
     * gets one if opening fails after authentication. The journal stays
     * locked and untouched: a journal a newer app saved is refused with a
     * message to update, never reset (docs/design-decisions.md, "Schema
     * upgrades").
     */
    private fun unlock(onError: (Int) -> Unit): Int? {
        val cipher = try {
            app.session.vault.decryptCipher()
        } catch (_: KeyPermanentlyInvalidatedException) {
            return R.string.error_key_invalidated
        } catch (e: Exception) {
            keystoreError = e
            return R.string.error_keystore
        }
        authenticate(cipher, getString(R.string.unlock_prompt)) { authorized ->
            try {
                app.session.unlock(authorized)
            } catch (_: CoreException.NewerVersion) {
                onError(R.string.newer_journal)
            } catch (_: Exception) {
                onError(R.string.error_unlock_failed)
            }
        }
        return null
    }

    private fun authenticate(cipher: Cipher, title: String, then: (Cipher) -> Unit) {
        prompting = true
        fun closed() {
            prompting = false
            promptClosedAt = SystemClock.elapsedRealtime()
        }
        UserAuthentication.authenticate(
            activity = this,
            cipher = cipher,
            title = title,
            cancel = getString(android.R.string.cancel),
            strict = app.lockSettings.strictBiometrics,
            onSuccess = {
                closed()
                then(it)
            },
            onFailure = { closed() },
        )
    }
}

/**
 * How long after a prompt closes a start of the activity still belongs to
 * that prompt (its PIN screen returning), not to the user coming back.
 */
private const val PROMPT_SETTLE_MILLIS = 1500L

/** Messages on the unlock and setup screens that offer a problem report. */
private val REPORTABLE = setOf(R.string.restore_failed, R.string.error_keystore, R.string.error_unlock_failed)
