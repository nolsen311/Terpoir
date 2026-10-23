// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.app.Application
import android.content.ComponentName
import android.graphics.Bitmap
import androidx.activity.ComponentActivity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onRoot
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.captureToImage
import androidx.test.core.app.ApplicationProvider
import app.terpoir.core.Journal
import app.terpoir.core.createKeys
import app.terpoir.core.vocabulary
import app.terpoir.journal.JournalHome
import app.terpoir.journal.JournalRepository
import app.terpoir.journal.ScanServices
import app.terpoir.privacy.PrivacyCenterScreen
import app.terpoir.scan.BundledTextEngine
import app.terpoir.scan.Scanner
import app.terpoir.ui.theme.TerpoirTheme
import java.io.File
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.rules.ExternalResource
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.Shadows.shadowOf
import org.robolectric.annotation.GraphicsMode

/**
 * The store screenshots, drawn from the real screens with a demo journal on
 * the core built for this computer. Runs only when Gradle is given
 * `-Pterpoir.screenshots=<dir>` (app/build.gradle.kts).
 */
@RunWith(ScreenshotRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(sdk = [36], qualifiers = "w411dp-h891dp-420dpi")
class StoreScreenshots {
    /** Registers the activity the screens are drawn in, which the app's manifest doesn't have. */
    @get:Rule(order = 0)
    val hostActivity = object : ExternalResource() {
        override fun before() {
            val context = ApplicationProvider.getApplicationContext<Application>()
            shadowOf(context.packageManager).addActivityIfNotPresent(ComponentName(context, ComponentActivity::class.java))
        }
    }

    @get:Rule(order = 1)
    val compose = createAndroidComposeRule<ComponentActivity>()

    private val out = File(System.getProperty("terpoir.screenshots") ?: error("run with -Pterpoir.screenshots=<dir>"))
    private lateinit var dir: File
    private lateinit var journal: Journal

    @Before
    fun openJournal() {
        dir = compose.activity.cacheDir.resolve("screenshots").apply { deleteRecursively(); mkdirs() }
        val keys = createKeys()
        journal = Journal.open(dir.resolve("journal.db").path, dir.resolve("files").path, keys.masterKey, keys.wrappedDatabaseKey)
        keys.masterKey.fill(0)
    }

    @After
    fun close() {
        if (::journal.isInitialized) {
            journal.lock()
            journal.close()
        }
    }

    private fun save(name: String) {
        compose.waitForIdle()
        val bitmap = compose.onRoot().captureToImage().asAndroidBitmap()
        out.mkdirs()
        out.resolve("$name.png").outputStream().use { bitmap.compress(Bitmap.CompressFormat.PNG, 100, it) }
    }

    /**
     * The unlocked app over a demo journal, as MainActivity shows it, with
     * the Privacy Center toggles in [on] turned on as the user would.
     */
    private fun home(vararg on: String) {
        val app = compose.activity.application as TerpoirApplication
        val repo = JournalRepository(journal)
        runBlocking {
            repo.seedDemo(System.currentTimeMillis())
            on.forEach { id -> app.privacyCenter.toggles.first { it.id == id }.set(true) }
        }
        compose.setContent {
            TerpoirTheme(darkTheme = false) {
                Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.surface)) {
                    JournalHome(
                        repo = repo,
                        vocab = vocabulary(),
                        scan = ScanServices(Scanner { BundledTextEngine(app) }) { false },
                        settings = app.privacySettings,
                        appearance = app.appearance,
                        alerts = app.spotAlerts,
                        checkIns = app.checkIns,
                        mapToggle = null,
                        checkNearby = false,
                        checkInSession = null,
                        onCheckInOpened = {},
                        quickLog = false,
                        onQuickLogOpened = {},
                        newNote = false,
                        onNewNoteOpened = {},
                        newEntry = false,
                        onNewEntryOpened = {},
                        shared = null,
                        onSharedOpened = {},
                        onEdited = {},
                    ) { onClose, settings ->
                        PrivacyCenterScreen(center = app.privacyCenter, onChanged = {}, onClose = onClose, settings = settings)
                    }
                }
            }
        }
        compose.waitForIdle()
    }

    /** Taps the first control showing [text], once the screen (which loads off the main thread) shows it. */
    private fun tap(text: String) {
        shown(text)
        compose.onAllNodesWithText(text, substring = true).onFirst().performClick()
        compose.waitForIdle()
    }

    private fun shown(text: String) =
        compose.waitUntil(10_000) { compose.onAllNodesWithText(text, substring = true).fetchSemanticsNodes().isNotEmpty() }

    @Test
    fun journalPage() {
        home()
        shown("Calm 1:1 Drops")
        save("1_journal")
    }

    @Test
    fun sessionPage() {
        home()
        tap("Granddaddy Purple")
        shown("Sleepy")
        save("2_session")
    }

    @Test
    fun insightsPage() {
        home("insights")
        shown("Your journal")
        compose.onNodeWithContentDescription("Insights").performClick()
        // Worked out when opened, off the main thread.
        shown("averaged")
        save("3_insights")
    }

    @Test
    fun productsPage() {
        home()
        tap("Products")
        shown("Coastal Extracts")
        save("4_products")
    }

    @Test
    fun privacyCenter() {
        home()
        tap("Settings")
        shown("Sync and keys")
        save("5_privacy")
    }
}
