// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.location

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationManager
import android.os.Build
import android.os.CancellationSignal
import androidx.core.content.ContextCompat
import kotlin.coroutines.resume
import kotlin.math.roundToInt
import kotlin.math.roundToLong
import kotlinx.coroutines.suspendCancellableCoroutine

/** A location fix in the core's units. */
data class Fix(val latE7: Int, val lonE7: Int, val accuracyM: Int)

/** Which location permission the app holds. */
enum class LocationAccess { NONE, APPROXIMATE, PRECISE }

/**
 * One current location fix through the platform's LocationManager, which
 * works the same on both builds (no Play services). Used only while the app
 * is open; nothing is registered with the OS. With only the approximate
 * permission the system itself blurs the fix.
 */
object CurrentLocation {
    fun access(context: Context): LocationAccess = when {
        granted(context, Manifest.permission.ACCESS_FINE_LOCATION) -> LocationAccess.PRECISE
        granted(context, Manifest.permission.ACCESS_COARSE_LOCATION) -> LocationAccess.APPROXIMATE
        else -> LocationAccess.NONE
    }

    /** A fresh fix, or `null` when there is no permission, provider or fix. */
    // The permission is checked just above (access), and a revoked one is
    // caught as SecurityException.
    @SuppressLint("MissingPermission")
    suspend fun get(context: Context): Fix? {
        val access = access(context)
        if (access == LocationAccess.NONE) return null
        val manager = context.getSystemService(LocationManager::class.java) ?: return null
        val provider = provider(manager, access) ?: return null
        return suspendCancellableCoroutine { cont ->
            val cancel = CancellationSignal()
            cont.invokeOnCancellation { cancel.cancel() }
            try {
                manager.getCurrentLocation(provider, cancel, ContextCompat.getMainExecutor(context)) { location ->
                    cont.resume(
                        location?.let {
                            Fix(
                                latE7 = (it.latitude * 1e7).roundToLong().toInt(),
                                lonE7 = (it.longitude * 1e7).roundToLong().toInt(),
                                accuracyM = if (it.hasAccuracy()) it.accuracy.roundToInt().coerceAtLeast(0) else DEFAULT_ACCURACY_M,
                            )
                        },
                    )
                }
            } catch (_: SecurityException) {
                cont.resume(null)
            }
        }
    }

    private fun provider(manager: LocationManager, access: LocationAccess): String? {
        val candidates = buildList {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) add(LocationManager.FUSED_PROVIDER)
            add(LocationManager.NETWORK_PROVIDER)
            // Satellite positioning needs the precise permission.
            if (access == LocationAccess.PRECISE) add(LocationManager.GPS_PROVIDER)
        }
        return candidates.firstOrNull { name ->
            runCatching { manager.isProviderEnabled(name) && name in manager.allProviders }.getOrDefault(false)
        }
    }

    private fun granted(context: Context, permission: String) =
        ContextCompat.checkSelfPermission(context, permission) == PackageManager.PERMISSION_GRANTED

    /** Accuracy assumed when a fix reports none. */
    private const val DEFAULT_ACCURACY_M = 2_000
}
