// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.onboarding

import android.content.Context
import com.google.android.play.agesignals.AgeSignalsManagerFactory
import com.google.android.play.agesignals.AgeSignalsRequest
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * Google Play Age Signals, read once at first launch. Only the age range
 * is used, to block under-18s; nothing from the result is stored or sent.
 *
 * Terpoir never calls `requestAgeSignalsAccess`, which would show Google's
 * prompt asking the user to share their age with the app: without a
 * signal, self-attestation decides.
 */
object PlayAgeSignals : AgeSignals {
    override suspend fun category(context: Context): AgeCategory {
        val task = AgeSignalsManagerFactory.create(context)
            .checkAgeSignals(AgeSignalsRequest.builder().build())
        return suspendCancellableCoroutine { continuation ->
            task.addOnCompleteListener { done ->
                val result = if (done.isSuccessful) done.result else null
                continuation.resume(
                    if (result == null) {
                        AgeCategory.UNKNOWN
                    } else {
                        ageCategoryFromRange(result.ageLower(), result.ageUpper())
                    },
                )
            }
        }
    }
}
