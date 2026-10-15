// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.content.Context
import android.graphics.Bitmap
import app.terpoir.R
import com.google.android.gms.common.moduleinstall.InstallStatusListener
import com.google.android.gms.common.moduleinstall.ModuleInstall
import com.google.android.gms.common.moduleinstall.ModuleInstallRequest
import com.google.android.gms.common.moduleinstall.ModuleInstallStatusUpdate.InstallState
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow
import com.google.mlkit.vision.text.TextRecognizer as MlKitRecognizer

/**
 * Google's ML Kit text recognition through Google Play services, used only
 * while the user has "Google text recognition" on (Play build, off by
 * default). Play services downloads and updates the model; recognition runs
 * on the device. See docs/design-decisions.md, "Rapid product entry".
 */
class GoogleTextEngine(context: Context) : TextEngine {
    private val modules = ModuleInstall.getClient(context)

    override val downloadPrompt: Int = R.string.ocr_download_google

    override suspend fun status(): EngineStatus {
        // ML Kit can fail to make a client at all (seen as a NullPointerException
        // inside TextRecognition.getClient, 0.5.2).
        val client = try {
            newClient()
        } catch (e: RuntimeException) {
            return EngineStatus.Failed(e)
        }
        return try {
            if (modules.areModulesAvailable(client).await().areModulesAvailable()) {
                EngineStatus.Ready
            } else {
                EngineStatus.NotInstalled
            }
        } catch (e: Exception) {
            // No usable Play services on this device.
            EngineStatus.Failed(e)
        } finally {
            client.close()
        }
    }

    override fun install(): Flow<EngineStatus> = callbackFlow {
        val client = newClient()
        val listener = InstallStatusListener { update ->
            when (update.installState) {
                InstallState.STATE_COMPLETED -> {
                    trySend(EngineStatus.Ready)
                    close()
                }
                InstallState.STATE_FAILED, InstallState.STATE_CANCELED -> {
                    trySend(EngineStatus.Failed(IllegalStateException("ML Kit model: state ${update.installState}, error ${update.errorCode}")))
                    close()
                }
                else -> {
                    val progress = update.progressInfo
                    val fraction = progress?.takeIf { it.totalBytesToDownload > 0 }
                        ?.let { it.bytesDownloaded.toFloat() / it.totalBytesToDownload }
                    trySend(EngineStatus.Downloading(fraction))
                }
            }
        }
        val request = ModuleInstallRequest.newBuilder().addApi(client).setListener(listener).build()
        modules.installModules(request)
            .addOnSuccessListener {
                if (it.areModulesAlreadyInstalled()) {
                    trySend(EngineStatus.Ready)
                    close()
                }
            }
            .addOnFailureListener {
                trySend(EngineStatus.Failed(it))
                close()
            }
        trySend(EngineStatus.Downloading(null))
        awaitClose {
            modules.unregisterListener(listener)
            client.close()
        }
    }

    override suspend fun open(): TextRecognizer {
        val client = newClient()
        return object : TextRecognizer {
            // ML Kit has no layout setting; the scanner regroups its lines into rows.
            override suspend fun recognize(image: Bitmap, layout: TextLayout): List<TextLine> =
                client.process(InputImage.fromBitmap(image, 0)).await().textBlocks
                    .flatMap { it.lines }
                    .mapNotNull { line -> line.boundingBox?.let { TextLine(line.text, it) } }

            override fun close() = client.close()
        }
    }

    companion object {
        private fun newClient(): MlKitRecognizer = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)

        /**
         * Tells Play services the app no longer needs the model, so it can
         * remove it (the toggle's off-action). Best effort: does nothing
         * without Play services, or when ML Kit can't make a client, as
         * when the model was never used. "Restore maximum privacy" runs
         * this on every recovery, so it must never throw (a crash after
         * the recovery key was entered, 0.5.2).
         */
        suspend fun release(context: Context) {
            val client = try {
                newClient()
            } catch (_: RuntimeException) {
                return
            }
            try {
                ModuleInstall.getClient(context).releaseModules(client).await()
            } catch (_: Exception) {
                // Nothing to release.
            } finally {
                runCatching { client.close() }
            }
        }
    }
}
