// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import androidx.activity.result.ActivityResultLauncher

/**
 * A system screen the user opened from the journal (the file or folder
 * picker) is in front. That screen is a separate app, so Android reports
 * Terpoir as gone to the background; hiding journal content then would
 * throw away the screen that asked, and with it the picker's answer.
 * While handed off, [AutoLock] keeps content shown, and "Immediately"
 * waits [AutoLockPolicy.HAND_OFF_GRACE_MILLIS] instead. Screen-off and
 * device lock still lock at once. Cleared when the activity resumes.
 */
object HandOff {
    @Volatile
    var active: Boolean = false
        private set

    /** Marks a hand-off, then launches [launcher]. */
    fun <I> launch(launcher: ActivityResultLauncher<I>, input: I) {
        active = true
        try {
            launcher.launch(input)
        } catch (e: RuntimeException) {
            active = false
            throw e
        }
    }

    /** Back in Terpoir (the activity resumed). */
    fun end() {
        active = false
    }
}
