// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.ui.theme

import android.app.UiModeManager
import android.content.Context
import android.os.Build
import androidx.annotation.StringRes
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import app.terpoir.R

/** The app's theme, as the user chose it. */
enum class ThemeChoice(val slug: String, @param:StringRes val title: Int) {
    /** Follow the phone's setting (the default). */
    SYSTEM("system", R.string.theme_system),
    LIGHT("light", R.string.theme_light),
    DARK("dark", R.string.theme_dark),
    ;

    /** Whether this choice is dark, given whether the phone is. */
    fun isDark(systemDark: Boolean): Boolean = when (this) {
        SYSTEM -> systemDark
        LIGHT -> false
        DARK -> true
    }
}

/**
 * The theme choice, kept on this phone only (not in the journal, so it
 * applies before unlock and doesn't sync). From Android 12 it is also
 * handed to the system ([UiModeManager.setApplicationNightMode]), so the
 * launch screen and system bars match it.
 */
class AppearanceSettings(private val context: Context) {
    private val prefs = context.getSharedPreferences("appearance", Context.MODE_PRIVATE)

    /** The current choice. */
    var theme: ThemeChoice by mutableStateOf(
        ThemeChoice.entries.firstOrNull { it.slug == prefs.getString(KEY_THEME, null) } ?: ThemeChoice.SYSTEM,
    )
        private set

    /** Saves [choice] and applies it. */
    fun choose(choice: ThemeChoice) {
        prefs.edit().putString(KEY_THEME, choice.slug).apply()
        theme = choice
        applyToSystem()
    }

    /** Hands the choice to the system, where it can take it (Android 12+). */
    fun applyToSystem() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) return
        val mode = when (theme) {
            ThemeChoice.SYSTEM -> UiModeManager.MODE_NIGHT_AUTO
            ThemeChoice.LIGHT -> UiModeManager.MODE_NIGHT_NO
            ThemeChoice.DARK -> UiModeManager.MODE_NIGHT_YES
        }
        context.getSystemService(UiModeManager::class.java)?.setApplicationNightMode(mode)
    }

    private companion object {
        const val KEY_THEME = "theme"
    }
}
