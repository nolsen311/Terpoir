// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.storage

import android.content.ContentResolver
import android.net.Uri
import java.io.FilterOutputStream
import java.io.IOException
import java.io.OutputStream

/** A document doesn't hold what was written to it. */
class IncompleteDocument : IOException()

/**
 * Writes to a document that was just created (by the system file picker
 * or `DocumentsContract.createDocument`), then reads it back and checks it
 * holds every byte. Some storage providers, cloud drives among them, fail
 * quietly and leave an empty file; this turns that into an error.
 *
 * A new document is empty, so plain "w" is enough, and it is the mode
 * every provider supports ("wt" is not). To replace an existing file,
 * delete it and create a new one.
 *
 * @throws IncompleteDocument if the document doesn't hold what was written.
 */
fun writeNewDocument(resolver: ContentResolver, uri: Uri, write: (OutputStream) -> Unit) {
    var written = 0L
    val raw = resolver.openOutputStream(uri, "w") ?: throw IOException("no output stream for $uri")
    raw.use {
        write(
            object : FilterOutputStream(it) {
                override fun write(b: Int) {
                    out.write(b)
                    written++
                }

                override fun write(b: ByteArray, off: Int, len: Int) {
                    out.write(b, off, len)
                    written += len
                }
            },
        )
    }
    val stored = (resolver.openInputStream(uri) ?: throw IncompleteDocument()).use { input ->
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val read = input.read(buffer)
            if (read < 0) break
            total += read
        }
        total
    }
    if (written == 0L || stored != written) throw IncompleteDocument()
}

/** [writeNewDocument] for a byte array. */
fun writeNewDocument(resolver: ContentResolver, uri: Uri, bytes: ByteArray) =
    writeNewDocument(resolver, uri) { it.write(bytes) }
