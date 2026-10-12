// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.share

import android.content.ContentProvider
import android.content.ContentValues
import android.database.Cursor
import android.database.MatrixCursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import android.provider.OpenableColumns
import java.io.FileNotFoundException
import kotlin.concurrent.thread

/**
 * Hands a share card to the app the user picked, straight from memory: the
 * image is never written to storage, so there is no file to delete and
 * nothing reaches the photo gallery (docs/design-decisions.md, "Sharing and
 * export"). Not exported; the share sheet grants a read permission for the
 * one URI.
 */
class ShareProvider : ContentProvider() {
    override fun onCreate() = true

    override fun getType(uri: Uri): String? = current(uri)?.let { MIME }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        if (mode != "r") throw FileNotFoundException()
        val bytes = current(uri) ?: throw FileNotFoundException()
        val (read, write) = ParcelFileDescriptor.createReliablePipe()
        thread(name = "share-card") {
            ParcelFileDescriptor.AutoCloseOutputStream(write).use { it.write(bytes) }
        }
        return read
    }

    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, args: Array<out String>?, order: String?): Cursor? {
        val bytes = current(uri) ?: return null
        val columns = projection ?: arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE)
        return MatrixCursor(columns, 1).apply {
            addRow(
                columns.map {
                    when (it) {
                        OpenableColumns.DISPLAY_NAME -> NAME
                        OpenableColumns.SIZE -> bytes.size.toLong()
                        else -> null
                    }
                },
            )
        }
    }

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun update(uri: Uri, values: ContentValues?, selection: String?, args: Array<out String>?) = 0

    override fun delete(uri: Uri, selection: String?, args: Array<out String>?) = 0

    companion object {
        private const val MIME = "image/png"
        private const val NAME = "terpoir.png"

        @Volatile
        private var card: Pair<String, ByteArray>? = null

        /** Offers [png] for sharing, replacing any earlier card, and returns its URI. */
        fun offer(authority: String, png: ByteArray): Uri {
            val token = java.util.UUID.randomUUID().toString()
            card = token to png
            return Uri.Builder().scheme("content").authority(authority).appendPath(token).appendPath(NAME).build()
        }

        /** Drops the card, so its URI stops working. */
        fun clear() {
            card = null
        }

        private fun current(uri: Uri): ByteArray? = card?.takeIf { it.first == uri.pathSegments.firstOrNull() }?.second
    }
}
