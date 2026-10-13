// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

import android.content.Context
import app.terpoir.alerts.AospGeofencer
import app.terpoir.alerts.Geofencer
import app.terpoir.map.GoogleMapSupport
import app.terpoir.onboarding.AgeSignals
import app.terpoir.privacy.PrivacySettings
import app.terpoir.scan.BundledTextEngine
import app.terpoir.scan.TextEngine

/**
 * The FOSS distribution (F-Droid, GitHub Releases): no Google Play services,
 * no proprietary libraries, and every feature available under the GPL.
 */
object Distribution {
    /** Short name, for the about screen. */
    const val NAME = "foss"

    /** No Google-dependent features. */
    val capabilities: Set<Capability> = emptySet()

    /** No platform age signal: self-attestation is the gate. */
    val ageSignals: AgeSignals = AgeSignals.NONE

    /** Nothing is delivered on demand. */
    @Suppress("UNUSED_PARAMETER")
    fun attachBaseContext(context: Context) = Unit

    /** Text recognition: the bundled Tesseract. */
    @Suppress("UNUSED_PARAMETER")
    fun textEngine(context: Context, settings: PrivacySettings): TextEngine = BundledTextEngine(context)

    /** "Notify me" geofencing through the platform's LocationManager. */
    fun geofencer(context: Context): Geofencer = AospGeofencer(context)

    /** No Google Maps in this build. */
    @Suppress("UNUSED_PARAMETER")
    fun googleMaps(context: Context): GoogleMapSupport? = null

    /** No Google text recognition here, so nothing to release. */
    @Suppress("UNUSED_PARAMETER", "RedundantSuspendModifier")
    suspend fun releaseGoogleTextRecognition(context: Context) = Unit
}
