// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir

/**
 * Features that depend on proprietary Google services. Only the `play`
 * distribution may offer them; `foss` provides open alternatives (AOSP
 * location) or does without (docs/design-decisions.md, "Distribution
 * channels").
 */
enum class Capability {
    /** Geofencing through Google Play services (AOSP fallback otherwise). */
    PLAY_SERVICES_LOCATION,

    /** On-device Gemini Nano tag suggestions. */
    GEMINI_NANO,

    /** Google Play Age Signals. */
    PLAY_AGE_SIGNALS,

    /** Google Play Billing for the one-time unlock. */
    PLAY_BILLING,

    /** ML Kit text recognition through Google Play services. */
    GOOGLE_TEXT_RECOGNITION,
}
