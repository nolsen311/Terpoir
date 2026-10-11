// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.content.Context
import app.terpoir.alerts.Geofencer
import app.terpoir.alerts.PlayGeofencer
import app.terpoir.onboarding.AgeSignals
import app.terpoir.onboarding.PlayAgeSignals
import app.terpoir.privacy.PrivacySettings
import app.terpoir.scan.GoogleTextEngine
import app.terpoir.scan.PlayTesseractEngine
import app.terpoir.scan.TextEngine
import com.google.android.play.core.splitcompat.SplitCompat

/** The Google Play distribution. */
object Distribution {
    /** Short name, for the about screen. */
    const val NAME = "play"

    /** Google-dependent features this build may offer (each still opt-in). */
    val capabilities: Set<Capability> = Capability.entries.toSet()

    /** Google Play Age Signals. */
    val ageSignals: AgeSignals = PlayAgeSignals

    /**
     * From the application's attachBaseContext: lets the app load the
     * on-demand text recognition module as soon as Play installs it,
     * without a restart.
     */
    fun attachBaseContext(context: Context) {
        SplitCompat.install(context)
    }

    /**
     * Text recognition: Google's ML Kit while the user has it on, otherwise
     * Tesseract from the on-demand module.
     */
    fun textEngine(context: Context, settings: PrivacySettings): TextEngine =
        if (settings.googleTextRecognition) GoogleTextEngine(context) else PlayTesseractEngine(context)

    /** "Notify me" geofencing: Play services, or the platform without them. */
    fun geofencer(context: Context): Geofencer = PlayGeofencer(context)

    /** Lets Play services remove ML Kit's model (the toggle's off-action). */
    suspend fun releaseGoogleTextRecognition(context: Context) = GoogleTextEngine.release(context)
}
