// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.scan

import android.content.Context
import app.terpoir.R
import com.google.android.play.core.assetpacks.AssetPackManagerFactory
import com.google.android.play.core.assetpacks.AssetPackStateUpdateListener
import com.google.android.play.core.assetpacks.model.AssetPackStatus
import com.google.android.play.core.splitinstall.SplitInstallManagerFactory
import com.google.android.play.core.splitinstall.SplitInstallRequest
import com.google.android.play.core.splitinstall.SplitInstallStateUpdatedListener
import com.google.android.play.core.splitinstall.model.SplitInstallSessionStatus
import java.io.File
import kotlinx.coroutines.channels.awaitClose
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.callbackFlow

/**
 * The Play build's default engine: Tesseract, from the on-demand `:ocr`
 * module (Play Feature Delivery) with its model in the `ocr_model` asset
 * pack (Play Asset Delivery). Neither is in the base app; the Play Store
 * downloads both when the user agrees, so the app needs no INTERNET
 * permission.
 */
class PlayTesseractEngine(private val context: Context) : TextEngine {
    private val splits = SplitInstallManagerFactory.create(context)
    private val packs = AssetPackManagerFactory.getInstance(context)

    override val downloadPrompt: Int = R.string.ocr_download_tesseract

    private fun moduleInstalled() = MODULE in splits.installedModules

    private fun modelDir(): File? = packs.getPackLocation(PACK)?.assetsPath()?.let(::File)

    override suspend fun status(): EngineStatus =
        if (moduleInstalled() && modelDir() != null) EngineStatus.Ready else EngineStatus.NotInstalled

    override fun install(): Flow<EngineStatus> = callbackFlow {
        // Bytes done and expected per part; a finished part counts as done.
        val module = Part(done = moduleInstalled())
        val model = Part(done = modelDir() != null)

        fun report() {
            when {
                module.failed || model.failed -> {
                    trySend(EngineStatus.Failed)
                    close()
                }
                module.done && model.done -> {
                    trySend(EngineStatus.Ready)
                    close()
                }
                else -> trySend(EngineStatus.Downloading(module.fraction(model)))
            }
        }

        val splitListener = SplitInstallStateUpdatedListener { state ->
            if (MODULE !in state.moduleNames()) return@SplitInstallStateUpdatedListener
            when (state.status()) {
                SplitInstallSessionStatus.INSTALLED -> module.done = true
                SplitInstallSessionStatus.FAILED, SplitInstallSessionStatus.CANCELED -> module.failed = true
                SplitInstallSessionStatus.REQUIRES_USER_CONFIRMATION -> {
                    trySend(EngineStatus.NeedsConfirmation { splits.startConfirmationDialogForResult(state, it) })
                    return@SplitInstallStateUpdatedListener
                }
                else -> {
                    module.bytes = state.bytesDownloaded()
                    module.total = state.totalBytesToDownload()
                }
            }
            report()
        }
        val packListener = AssetPackStateUpdateListener { state ->
            if (state.name() != PACK) return@AssetPackStateUpdateListener
            val status = state.status()
            // Outside the when: the library's @IntDef omits this status.
            if (status == AssetPackStatus.REQUIRES_USER_CONFIRMATION || status == AssetPackStatus.WAITING_FOR_WIFI) {
                trySend(EngineStatus.NeedsConfirmation { packs.showConfirmationDialog(it) })
                return@AssetPackStateUpdateListener
            }
            when (status) {
                AssetPackStatus.COMPLETED -> model.done = true
                AssetPackStatus.FAILED, AssetPackStatus.CANCELED -> model.failed = true
                else -> {
                    model.bytes = state.bytesDownloaded()
                    model.total = state.totalBytesToDownload()
                }
            }
            report()
        }
        splits.registerListener(splitListener)
        packs.registerListener(packListener)
        if (!module.done) {
            splits.startInstall(SplitInstallRequest.newBuilder().addModule(MODULE).build())
                .addOnFailureListener {
                    module.failed = true
                    report()
                }
        }
        if (!model.done) {
            packs.fetch(listOf(PACK)).addOnFailureListener {
                model.failed = true
                report()
            }
        }
        report()
        awaitClose {
            splits.unregisterListener(splitListener)
            packs.unregisterListener(packListener)
        }
    }

    override suspend fun open(): TextRecognizer? {
        val dir = modelDir() ?: return null
        if (!moduleInstalled()) return null
        val module = runCatching {
            Class.forName(ENTRY_POINT, true, context.classLoader).getDeclaredConstructor().newInstance() as TextEngineModule
        }.getOrNull() ?: return null
        return module.open(dir)
    }

    /** Download progress of the module or the model. */
    private class Part(var done: Boolean) {
        var failed = false
        var bytes = 0L
        var total = 0L

        /** Combined progress of this part and [other], or null while a size is unknown. */
        fun fraction(other: Part): Float? {
            val parts = listOf(this, other)
            if (parts.any { !it.done && it.total <= 0 }) return null
            // A part that was already installed has no size and drops out.
            val counted = parts.filter { it.total > 0 }
            return counted.sumOf { if (it.done) it.total else it.bytes }.toFloat() / counted.sumOf { it.total }
        }
    }

    private companion object {
        const val MODULE = "ocr"
        const val PACK = "ocr_model"
        const val ENTRY_POINT = "app.terpoir.ocr.TesseractModule"
    }
}
