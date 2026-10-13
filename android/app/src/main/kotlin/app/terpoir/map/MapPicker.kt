// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import app.terpoir.R
import app.terpoir.location.Fix
import kotlin.math.roundToInt
import org.maplibre.android.camera.CameraPosition
import org.maplibre.android.geometry.LatLng
import org.maplibre.android.maps.MapLibreMap
import org.maplibre.android.maps.MapView

/** How close a map starts on a saved point, and on the whole world. */
private const val POINT_ZOOM = 16.0
private const val WORLD_ZOOM = 1.0

/** The crosshair's radius, which is also the picked point's accuracy on the ground. */
private val CROSSHAIR_RADIUS = 12.dp

/** The smallest and largest accuracy a pick records, in metres. */
private const val MIN_ACCURACY_M = 5
private const val MAX_ACCURACY_M = 50_000

/**
 * Picks a point on the online map: the user moves the map under a fixed
 * crosshair and confirms. The accuracy recorded is the ground the crosshair
 * covers at that zoom, so a pick made zoomed out is as vague as it looks.
 * No location permission is needed; the phone's position is never read.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun MapPicker(
    start: Fix?,
    provider: MapProvider,
    styleUrl: String?,
    google: GoogleMapSupport?,
    onPick: (Fix) -> Unit,
    onClose: () -> Unit,
) {
    BackHandler(onBack = onClose)
    // Reads the point under the crosshair, once the map is ready.
    var pick by remember { mutableStateOf<(() -> Fix)?>(null) }
    val radiusPx = with(LocalDensity.current) { CROSSHAIR_RADIUS.toPx() }
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.map_pick_title)) },
                navigationIcon = {
                    IconButton(onClick = onClose) {
                        Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.close))
                    }
                },
            )
        },
    ) { padding ->
        Column(modifier = Modifier.padding(padding).fillMaxSize()) {
            Text(
                stringResource(R.string.map_pick_hint),
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp),
            )
            Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                if (provider == MapProvider.GOOGLE) {
                    google?.PickerMap(start, CROSSHAIR_RADIUS) { pick = it }
                } else if (styleUrl != null) {
                    MapLibreView(start, styleUrl) { map -> pick = { pickedFix(map, radiusPx) } }
                }
                Crosshair()
                // The providers' terms: the credit always shows on the map,
                // not behind an info button. Google draws its own.
                provider.attribution?.let { credit ->
                    Text(
                        stringResource(credit),
                        style = MaterialTheme.typography.labelSmall,
                        modifier = Modifier
                            .align(Alignment.BottomStart)
                            .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.8f))
                            .padding(horizontal = 4.dp, vertical = 2.dp),
                    )
                }
            }
            Button(
                enabled = pick != null,
                onClick = { pick?.let { onPick(it()) } },
                modifier = Modifier.fillMaxWidth().padding(16.dp),
            ) { Text(stringResource(R.string.map_pick_use)) }
        }
    }
}

/** The point under the crosshair, with the ground it covers as accuracy. */
private fun pickedFix(map: MapLibreMap, radiusPx: Float): Fix {
    val target = map.cameraPosition.target ?: LatLng(0.0, 0.0)
    val metresPerPixel = map.projection.getMetersPerPixelAtLatitude(target.latitude)
    val accuracy = (metresPerPixel * radiusPx).roundToInt().coerceIn(MIN_ACCURACY_M, MAX_ACCURACY_M)
    return Fix((target.latitude * 1e7).roundToInt(), (target.longitude * 1e7).roundToInt(), accuracy)
}

/** A MapLibre map view, tied to the screen's lifecycle. */
@Composable
private fun MapLibreView(start: Fix?, styleUrl: String, onReady: (MapLibreMap) -> Unit) {
    val context = LocalContext.current
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    val view = remember {
        MapView(context).apply {
            onCreate(null)
            getMapAsync { map ->
                map.uiSettings.isLogoEnabled = false
                map.uiSettings.isRotateGesturesEnabled = false
                map.uiSettings.isTiltGesturesEnabled = false
                map.cameraPosition = CameraPosition.Builder()
                    .target(start?.let { LatLng(it.latE7 / 1e7, it.lonE7 / 1e7) } ?: LatLng(0.0, 0.0))
                    .zoom(if (start != null) POINT_ZOOM else WORLD_ZOOM)
                    .build()
                map.setStyle(styleUrl)
                onReady(map)
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

/** A ring with a dot, fixed at the map's centre. */
@Composable
private fun Crosshair() {
    val color = MaterialTheme.colorScheme.primary
    Canvas(modifier = Modifier.size(CROSSHAIR_RADIUS * 2)) {
        val centre = Offset(size.width / 2, size.height / 2)
        drawCircle(color, radius = size.minDimension / 2 - 1.dp.toPx(), center = centre, style = Stroke(width = 2.dp.toPx()))
        drawCircle(color, radius = 2.dp.toPx(), center = centre)
    }
}
