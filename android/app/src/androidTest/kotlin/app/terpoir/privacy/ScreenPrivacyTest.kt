// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.content.Context
import android.os.Build
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import androidx.lifecycle.Lifecycle
import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import app.terpoir.MainActivity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Test
import org.junit.runner.RunWith

/** The window protections from ScreenPrivacy on a real device. */
@RunWith(AndroidJUnit4::class)
class ScreenPrivacyTest {
    private val context = InstrumentationRegistry.getInstrumentation().targetContext
    private val settings = PrivacySettings(context)

    @After
    fun restoreDefault() {
        settings.screenProtection = true
    }

    @Test
    fun screenProtectionIsOnByDefault() {
        context.getSharedPreferences("privacy", Context.MODE_PRIVATE).edit().clear().commit()
        assertTrue(settings.screenProtection)
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity { assertTrue(it.isSecure()) }
        }
    }

    @Test
    fun screenProtectionOffClearsFlagSecure() {
        settings.screenProtection = false
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity { assertFalse(it.isSecure()) }
            // Not in front: Android 11 and 12 can only keep the app out of
            // Recents with FLAG_SECURE, so it comes back while paused.
            scenario.moveToState(Lifecycle.State.STARTED)
            scenario.onActivity { assertEquals(Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU, it.isSecure()) }
            scenario.moveToState(Lifecycle.State.RESUMED)
            scenario.onActivity { assertFalse(it.isSecure()) }
        }
    }

    @Test
    fun autofillIsOffForTheWholeUi() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity {
                assertEquals(
                    View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS,
                    it.window.decorView.importantForAutofill,
                )
                assertFalse(it.composeView().isImportantForAutofill)
            }
        }
    }

    @Test
    fun contentIsAccessibilityDataSensitive() {
        assumeTrue(Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE)
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity { assertTrue(it.composeView().isAccessibilityDataSensitive) }
        }
    }

    @Test
    fun textFieldsAskForNoPersonalizedLearning() {
        val attrs = EditorInfo().apply { imeOptions = EditorInfo.IME_ACTION_DONE }
        applyPrivateImeOptions(attrs)
        assertEquals(
            EditorInfo.IME_ACTION_DONE or EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING,
            attrs.imeOptions,
        )
    }

    private fun MainActivity.isSecure() =
        window.attributes.flags and WindowManager.LayoutParams.FLAG_SECURE != 0

    private fun MainActivity.composeView(): View =
        findViewById<ViewGroup>(android.R.id.content).getChildAt(0)
}
