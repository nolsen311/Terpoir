// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Matrix
import android.util.Size
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.compose.CameraXViewfinder
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.SurfaceRequest
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.lifecycle.compose.LocalLifecycleOwner
import app.terpoir.R
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

/** The largest photo taken: 12 megapixels, 4:3. */
private val PHOTO_SIZE = Size(4000, 3000)

/** What the camera is for. */
enum class CaptureMode {
    /** One still photo (labels, receipts), kept in memory only. */
    PHOTO,

    /** Live decoding of barcodes and QR codes; nothing is captured. */
    CODE,
}

/**
 * The in-app camera. Nothing it sees is written to storage or the gallery:
 * a photo is handed to [onPhoto] as an in-memory bitmap, which the caller
 * recycles once it has been read.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CameraScreen(
    mode: CaptureMode,
    title: String,
    hint: String,
    onClose: () -> Unit,
    onPhoto: (Bitmap) -> Unit = {},
    onCode: (DecodedCode) -> Unit = {},
) {
    val context = LocalContext.current
    var granted by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED,
        )
    }
    var asked by remember { mutableStateOf(false) }
    val request = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) {
        granted = it
        asked = true
    }
    LaunchedEffect(Unit) { if (!granted) request.launch(Manifest.permission.CAMERA) }
    BackHandler(onBack = onClose)

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title) },
                navigationIcon = {
                    IconButton(onClick = onClose) {
                        Icon(painterResource(R.drawable.ic_close), contentDescription = stringResource(R.string.close))
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.padding(padding).padding(16.dp).fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(hint, style = MaterialTheme.typography.bodyMedium)
            if (granted) {
                Viewfinder(mode, onPhoto, onCode, Modifier.weight(1f).fillMaxWidth())
            } else if (asked) {
                Text(stringResource(R.string.scan_camera_denied))
                Button(onClick = { request.launch(Manifest.permission.CAMERA) }) {
                    Text(stringResource(R.string.scan_camera_allow))
                }
            }
        }
    }
}

@Composable
private fun Viewfinder(
    mode: CaptureMode,
    onPhoto: (Bitmap) -> Unit,
    onCode: (DecodedCode) -> Unit,
    modifier: Modifier,
) {
    val context = LocalContext.current
    val lifecycleOwner = LocalLifecycleOwner.current
    var surface by remember { mutableStateOf<SurfaceRequest?>(null) }
    var busy by remember { mutableStateOf(false) }
    val capture = remember {
        // Small print needs a sharp photo; the size cap keeps 50-megapixel
        // sensors from filling memory.
        ImageCapture.Builder()
            .setCaptureMode(ImageCapture.CAPTURE_MODE_MAXIMIZE_QUALITY)
            .setResolutionSelector(
                ResolutionSelector.Builder()
                    .setResolutionStrategy(ResolutionStrategy(PHOTO_SIZE, ResolutionStrategy.FALLBACK_RULE_CLOSEST_LOWER_THEN_HIGHER))
                    .build(),
            )
            .build()
    }
    val analysisExecutor = remember { Executors.newSingleThreadExecutor() }
    val done = remember { AtomicBoolean(false) }

    DisposableEffect(lifecycleOwner, mode) {
        val preview = Preview.Builder().build().apply { setSurfaceProvider { surface = it } }
        val useCase = when (mode) {
            CaptureMode.PHOTO -> capture
            CaptureMode.CODE -> ImageAnalysis.Builder()
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .build()
                .apply {
                    val decoder = CodeDecoder()
                    setAnalyzer(analysisExecutor) { image ->
                        image.use {
                            if (done.get()) return@use
                            val code = decodeFrame(decoder, it) ?: return@use
                            if (done.compareAndSet(false, true)) {
                                ContextCompat.getMainExecutor(context).execute { onCode(code) }
                            }
                        }
                    }
                }
        }
        var provider: ProcessCameraProvider? = null
        val future = ProcessCameraProvider.getInstance(context)
        future.addListener(
            {
                provider = future.get().also {
                    it.unbindAll()
                    it.bindToLifecycle(lifecycleOwner, CameraSelector.DEFAULT_BACK_CAMERA, preview, useCase)
                }
            },
            ContextCompat.getMainExecutor(context),
        )
        onDispose {
            provider?.unbindAll()
        }
    }
    DisposableEffect(Unit) { onDispose { analysisExecutor.shutdown() } }

    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(modifier = Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
            surface?.let { CameraXViewfinder(surfaceRequest = it, modifier = Modifier.fillMaxSize()) }
            if (busy) CircularProgressIndicator()
        }
        if (mode == CaptureMode.PHOTO) {
            Button(
                enabled = !busy,
                onClick = {
                    busy = true
                    capture.takePicture(
                        ContextCompat.getMainExecutor(context),
                        object : ImageCapture.OnImageCapturedCallback() {
                            override fun onCaptureSuccess(image: ImageProxy) {
                                val bitmap = image.use { upright(it) }
                                busy = false
                                onPhoto(bitmap)
                            }

                            override fun onError(exception: ImageCaptureException) {
                                busy = false
                            }
                        },
                    )
                },
            ) { Text(stringResource(R.string.scan_take_photo)) }
        }
    }
}

/** The captured image, turned upright, in memory only. */
private fun upright(image: ImageProxy): Bitmap {
    val bitmap = image.toBitmap()
    val degrees = image.imageInfo.rotationDegrees
    if (degrees == 0) return bitmap
    val turned = Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, Matrix().apply { postRotate(degrees.toFloat()) }, true)
    if (turned !== bitmap) bitmap.recycle()
    return turned
}

/** Decodes the luminance plane of a camera frame. */
private fun decodeFrame(decoder: CodeDecoder, image: ImageProxy): DecodedCode? {
    val plane = image.planes.firstOrNull() ?: return null
    if (plane.pixelStride != 1) return null
    val buffer = plane.buffer
    val bytes = ByteArray(buffer.remaining()).also { buffer.get(it) }
    if (bytes.size < plane.rowStride * (image.height - 1) + image.width) return null
    return decoder.decode(bytes, image.width, image.height, plane.rowStride)
}
