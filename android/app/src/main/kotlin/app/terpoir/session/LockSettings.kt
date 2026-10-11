// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import android.content.Context

/**
 * Lock preferences. Not journal content (a timeout and a mode), so they
 * live outside the encrypted store and are readable before unlock; app
 * data is excluded from backups.
 */
class LockSettings(context: Context) {
    private val prefs = context.getSharedPreferences("lock", Context.MODE_PRIVATE)

    /** The auto-lock timeout. */
    var timeout: LockTimeout
        get() = LockTimeout.fromName(prefs.getString(KEY_TIMEOUT, null))
        set(value) = prefs.edit().putString(KEY_TIMEOUT, value.name).apply()

    /**
     * Strict mode: unlock with the currently enrolled biometrics only. Off
     * by default (biometric or device passcode). Takes effect when the
     * wrapping key is next created.
     */
    var strictBiometrics: Boolean
        get() = prefs.getBoolean(KEY_STRICT, false)
        set(value) = prefs.edit().putBoolean(KEY_STRICT, value).apply()

    private companion object {
        const val KEY_TIMEOUT = "timeout"
        const val KEY_STRICT = "strict_biometrics"
    }
}
