// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.IntentSenderRequest
import androidx.annotation.StringRes
import kotlinx.coroutines.flow.Flow

/** Where a text recognition engine stands on this device. */
sealed interface EngineStatus {
    /** Installed and ready to read. */
    data object Ready : EngineStatus

    /** Needs a one-time download before first use, which the user must agree to. */
    data object NotInstalled : EngineStatus

    /** Downloading; [fraction] is null while the size is unknown. */
    data class Downloading(val fraction: Float?) : EngineStatus

    /**
     * The app store asks the user to confirm the download (a large one, or
     * one over mobile data): [confirm] shows its dialog.
     */
    class NeedsConfirmation(val confirm: (ActivityResultLauncher<IntentSenderRequest>) -> Unit) : EngineStatus

    /**
     * The download failed, or the app store is not available. [error]
     * names why (an error code from the store, never anything personal),
     * so a problem report can say.
     */
    data class Failed(val error: Throwable? = null) : EngineStatus
}

/**
 * A text recognition engine (docs/design-decisions.md, "Rapid product
 * entry"). The scanner sees only this interface, so which engine reads a
 * photo is one switch: the bundled Tesseract (foss), Tesseract from Play's
 * on-demand module, or Google's ML Kit when the user turns it on (Play).
 * Every engine runs on the device.
 */
interface TextEngine {
    /** What the download prompt tells the user, for an engine that needs one. */
    @get:StringRes
    val downloadPrompt: Int

    /** Whether the engine can read now. */
    suspend fun status(): EngineStatus

    /**
     * Downloads the engine, only after the user agreed, reporting progress.
     * The flow ends after [EngineStatus.Ready] or [EngineStatus.Failed].
     */
    fun install(): Flow<EngineStatus>

    /** Starts a recognizer, or returns `null` if the engine is not ready or would not start. */
    suspend fun open(): TextRecognizer?
}
