// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import android.content.Context
import java.io.File
import org.maplibre.android.MapLibre
import org.maplibre.android.log.Logger
import org.maplibre.android.log.LoggerDefinition
import org.maplibre.android.module.http.HttpRequestUtil
import org.maplibre.android.offline.OfflineManager

/**
 * The opt-in online map (docs/design-decisions.md, "Map picker"): MapLibre
 * showing the chosen [MapProvider]'s map, for picking a spot's location.
 * In every build, and only while the "Online map" toggle is on.
 */
object OnlineMap {
    /** MapLibre's tile database and settings file, which [forget] deletes. */
    private const val DATABASE = "mbgl-offline.db"
    private const val PREFERENCES = "MapboxSharedPreferences"

    private var started = false

    /**
     * Sets MapLibre up once, before the first map: logging off, every
     * request through [MapGateway] (which asks [enabled] and [provider] each
     * time), and no tile cache kept, since cached tiles would show which
     * areas were viewed.
     */
    fun start(context: Context, enabled: () -> Boolean, provider: () -> MapProvider) {
        if (started) return
        Logger.setVerbosity(Logger.NONE)
        Logger.setLoggerDefinition(Silent)
        MapLibre.getInstance(context.applicationContext)
        // MapLibre would otherwise read the network state, a permission the
        // foss app does not have; the gateway decides what goes out.
        MapLibre.setConnected(true)
        HttpRequestUtil.setLogEnabled(false)
        HttpRequestUtil.setPrintRequestUrlOnFailure(false)
        HttpRequestUtil.setOkHttpClient(MapGateway.client(enabled, provider))
        OfflineManager.getInstance(context).setMaximumAmbientCacheSize(0, Ignore)
        started = true
    }

    /**
     * The toggle's off-action: deletes anything MapLibre stored (its tile
     * database and its settings file), whether or not a map was shown in
     * this run.
     */
    fun forget(context: Context) {
        if (started) OfflineManager.getInstance(context).resetDatabase(Ignore)
        val dirs = listOfNotNull(context.filesDir, context.cacheDir, context.getExternalFilesDir(null))
        for (dir in dirs) {
            dir.listFiles { file: File -> file.name.startsWith(DATABASE) }?.forEach(File::delete)
        }
        context.deleteSharedPreferences(PREFERENCES)
    }

    private object Ignore : OfflineManager.FileSourceCallback {
        override fun onSuccess() = Unit

        override fun onError(message: String) = Unit
    }

    /** Drops everything MapLibre would log (its native code logs through this too). */
    private object Silent : LoggerDefinition {
        override fun v(tag: String, msg: String) = Unit

        override fun v(tag: String, msg: String, tr: Throwable) = Unit

        override fun d(tag: String, msg: String) = Unit

        override fun d(tag: String, msg: String, tr: Throwable) = Unit

        override fun i(tag: String, msg: String) = Unit

        override fun i(tag: String, msg: String, tr: Throwable) = Unit

        override fun w(tag: String, msg: String) = Unit

        override fun w(tag: String, msg: String, tr: Throwable) = Unit

        override fun e(tag: String, msg: String) = Unit

        override fun e(tag: String, msg: String, tr: Throwable) = Unit
    }
}
