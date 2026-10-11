// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.app.Activity
import android.os.Build
import android.view.View
import android.view.WindowManager

/**
 * Window-level protections for the app's screens. See
 * docs/design-decisions.md, "Accessibility and screen capture" and
 * "Keyboards and voice input".
 *
 * Always on, because they cost the user nothing:
 * - Recents shows no snapshot of the app (app-switcher blur).
 * - Android 14+: content is accessibility-data-sensitive, so only services
 *   declared as accessibility tools (TalkBack, Switch Access, Voice Access)
 *   can read it, not every app holding the accessibility permission.
 * - Autofill is off for every field.
 *
 * Toggle, default on: screen protection (FLAG_SECURE).
 */
object ScreenPrivacy {
    /** Applies the always-on protections. Call once from `onCreate`. */
    fun install(activity: Activity) {
        val root = activity.window.decorView
        root.importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            root.setAccessibilityDataSensitive(View.ACCESSIBILITY_DATA_SENSITIVE_YES)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.setRecentsScreenshotEnabled(false)
        }
    }

    /** Screen protection's on/off state. Call from `onResume`. */
    fun onResume(activity: Activity, screenProtection: Boolean) {
        setSecure(activity, screenProtection)
    }

    /**
     * Android 11 and 12 have no way to hide the app from Recents except
     * FLAG_SECURE, so it is set while the app is not in front even when
     * screen protection is off. Call from `onPause`.
     */
    fun onPause(activity: Activity) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) setSecure(activity, true)
    }

    private fun setSecure(activity: Activity, secure: Boolean) {
        if (secure) {
            activity.window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        } else {
            activity.window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
    }
}
