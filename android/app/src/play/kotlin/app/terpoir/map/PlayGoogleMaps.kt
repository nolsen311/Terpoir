// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import android.content.Context
import android.content.pm.PackageManager
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import app.terpoir.location.Fix
import com.google.android.gms.maps.CameraUpdateFactory
import com.google.android.gms.maps.GoogleMap
import com.google.android.gms.maps.MapView
import com.google.android.gms.maps.model.LatLng
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.pow
import kotlin.math.roundToInt

/** How close a map starts on a saved point, and on the whole world. */
private const val POINT_ZOOM = 16f
private const val WORLD_ZOOM = 1f

/** Ground metres per dp at the equator at zoom 0 (a 256 dp wide world). */
private const val EQUATOR_METRES_PER_DP = 40_075_016.686 / 256

/** The smallest and largest accuracy a pick records, in metres. */
private const val MIN_ACCURACY_M = 5
private const val MAX_ACCURACY_M = 50_000

private const val KEY_NAME = "com.google.android.geo.API_KEY"

/**
 * Google Maps through Play services (docs/design-decisions.md, "Map
 * picker"). A basic map (no map ID, so no styling or billing), with the
 * user's location layer, buildings, indoor maps and the map toolbar off.
 */
object PlayGoogleMaps : GoogleMapSupport {
    /** Whether the build put a Google Maps key in the manifest. */
    fun hasKey(context: Context): Boolean {
        val info = context.packageManager.getApplicationInfo(context.packageName, PackageManager.GET_META_DATA)
        return !info.metaData?.getString(KEY_NAME).isNullOrBlank()
    }

    @Composable
    override fun PickerMap(start: Fix?, crosshairRadius: Dp, onReady: (pick: () -> Fix) -> Unit) {
        val context = LocalContext.current
        val lifecycle = LocalLifecycleOwner.current.lifecycle
        val view = remember {
            MapView(context).apply {
                onCreate(null)
                getMapAsync { map ->
                    map.mapType = GoogleMap.MAP_TYPE_NORMAL
                    map.isBuildingsEnabled = false
                    map.isIndoorEnabled = false
                    map.uiSettings.isMapToolbarEnabled = false
                    map.uiSettings.isRotateGesturesEnabled = false
                    map.uiSettings.isTiltGesturesEnabled = false
                    map.uiSettings.isCompassEnabled = false
                    map.moveCamera(
                        CameraUpdateFactory.newLatLngZoom(
                            start?.let { LatLng(it.latE7 / 1e7, it.lonE7 / 1e7) } ?: LatLng(0.0, 0.0),
                            if (start != null) POINT_ZOOM else WORLD_ZOOM,
                        ),
                    )
                    onReady { pickedFix(map, crosshairRadius) }
                }
            }
        }
        DisposableEffect(lifecycle, view) {
            val observer = LifecycleEventObserver { _, event ->
                when (event) {
                    Lifecycle.Event.ON_START -> view.onStart()
                    Lifecycle.Event.ON_RESUME -> view.onResume()
                    Lifecycle.Event.ON_PAUSE -> view.onPause()
                    Lifecycle.Event.ON_STOP -> view.onStop()
                    else -> Unit
                }
            }
            lifecycle.addObserver(observer)
            onDispose {
                lifecycle.removeObserver(observer)
                if (lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)) view.onPause()
                if (lifecycle.currentState.isAtLeast(Lifecycle.State.STARTED)) view.onStop()
                view.onDestroy()
            }
        }
        AndroidView(factory = { view }, modifier = Modifier.fillMaxSize())
    }

    /** The point under the centre, with the ground the crosshair covers as accuracy. */
    private fun pickedFix(map: GoogleMap, crosshairRadius: Dp): Fix {
        val camera = map.cameraPosition
        val target = camera.target
        val metresPerDp = EQUATOR_METRES_PER_DP * cos(target.latitude * PI / 180) / 2.0.pow(camera.zoom.toDouble())
        val accuracy = (metresPerDp * crosshairRadius.value).roundToInt().coerceIn(MIN_ACCURACY_M, MAX_ACCURACY_M)
        return Fix((target.latitude * 1e7).roundToInt(), (target.longitude * 1e7).roundToInt(), accuracy)
    }
}
