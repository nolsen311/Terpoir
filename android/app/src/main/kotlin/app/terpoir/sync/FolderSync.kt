// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.sync

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.provider.DocumentsContract
import app.terpoir.core.CoreException
import app.terpoir.core.Journal
import app.terpoir.core.syncFolderKeyFile
import java.security.MessageDigest
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext

/**
 * Sync through a folder the user picks in the system file picker
 * (docs/design-decisions.md, "Settled 2026-10-12"): a Syncthing folder, or
 * Google Drive, Nextcloud or Dropbox through their apps' document
 * providers. Terpoir needs no INTERNET permission and holds no credentials;
 * whatever syncs the folder moves the files.
 *
 * Only ciphertext reaches the folder: the core's sync blobs, padded and
 * under random names. Merging happens in the core; this class only moves
 * files. Each sync:
 * 1. imports every blob not seen before (another device's, or a retry);
 * 2. writes this device's full state as new blobs, each under a temporary
 *    name renamed when complete, so no device reads half a file;
 * 3. deletes this device's previous blobs, which the new ones supersede;
 * 4. writes `terpoir.key` (the master key wrapped by the recovery key, from
 *    [masterWrappedByRecovery]) if the folder has none, or replaces the one
 *    this device wrote if its own copy changed. A new device restores from
 *    the folder with the recovery key ([readForRestore]).
 */
class FolderSync(private val context: Context, private val masterWrappedByRecovery: () -> ByteArray?) {
    private val prefs = context.getSharedPreferences("folder_sync", Context.MODE_PRIVATE)
    private val lock = Mutex()

    /** What the last sync did. */
    data class Outcome(val imported: Int, val written: Int, val needsUpdate: Boolean = false)

    /** The folder, if one was chosen and its permission is still held. */
    val folder: Uri?
        get() = prefs.getString(KEY_FOLDER, null)?.let(Uri::parse)?.takeIf { uri ->
            context.contentResolver.persistedUriPermissions.any { it.uri == uri && it.isReadPermission && it.isWritePermission }
        }

    /** When the last sync finished, in ms since the epoch, or 0. */
    val lastSyncMs: Long get() = prefs.getLong(KEY_LAST, 0)

    /** Keeps access to [tree], the folder the user picked, across restarts. */
    fun choose(tree: Uri) {
        val flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        context.contentResolver.takePersistableUriPermission(tree, flags)
        val previous = prefs.getString(KEY_FOLDER, null)
        // A different folder: nothing seen or written there yet.
        if (previous != tree.toString()) prefs.edit().clear().apply()
        prefs.edit().putString(KEY_FOLDER, tree.toString()).apply()
    }

    /**
     * The toggle's off-action: gives up the folder permission and forgets
     * what was seen and written. Files already in the folder stay there
     * (encrypted); the card says so.
     */
    fun forget() {
        prefs.getString(KEY_FOLDER, null)?.let(Uri::parse)?.let { uri ->
            val flags = Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
            runCatching { context.contentResolver.releasePersistableUriPermission(uri, flags) }
        }
        prefs.edit().clear().apply()
    }

    /** Syncs with the folder once; `null` if there is no folder. Throws on I/O failure. */
    suspend fun sync(journal: Journal): Outcome? = lock.withLock {
        val tree = folder ?: return@withLock null
        withContext(Dispatchers.IO) {
            val resolver = context.contentResolver
            val dir = DocumentsContract.buildDocumentUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
            val seen = prefs.getStringSet(KEY_SEEN, emptySet()).orEmpty().toMutableSet()
            val own = prefs.getStringSet(KEY_OWN, emptySet()).orEmpty()

            // 1. Import what is new. A blob that fails (another device still
            //    writing it, or in a newer app's format) is left for the next
            //    sync. Records a newer app wrote inside a readable blob are
            //    kept by the core and applied after updating.
            var imported = 0
            var needsUpdate = false
            for ((name, doc) in blobs(resolver, tree)) {
                if (name in seen || name in own) continue
                val bytes = runCatching { resolver.openInputStream(doc)?.use { it.readBytes() } }.getOrNull() ?: continue
                val result = runCatching { journal.importSyncBlob(bytes, System.currentTimeMillis().toULong()) }
                if (result.exceptionOrNull() is CoreException.NewerVersion) needsUpdate = true
                if (result.isFailure) continue
                seen += name
                imported++
            }

            // 2. Write this device's full state under fresh names.
            val written = mutableSetOf<String>()
            for (blob in journal.exportSyncBlobs()) {
                val name = blob.name + SUFFIX
                val partial = DocumentsContract.createDocument(resolver, dir, MIME, name + PARTIAL) ?: continue
                resolver.openOutputStream(partial, "wt")?.use { it.write(blob.bytes) }
                val renamed = runCatching { DocumentsContract.renameDocument(resolver, partial, name) }.getOrNull()
                if (renamed == null) {
                    // Some providers cannot rename: write under the final
                    // name directly. A device reading it half-written fails
                    // to import it and retries at its next sync.
                    runCatching { DocumentsContract.deleteDocument(resolver, partial) }
                    val direct = DocumentsContract.createDocument(resolver, dir, MIME, name) ?: continue
                    resolver.openOutputStream(direct, "wt")?.use { it.write(blob.bytes) }
                }
                written += name
            }

            // 3. Remove this device's previous blobs (and stray partials).
            for ((name, doc) in blobs(resolver, tree, includePartial = true)) {
                if ((name in own && name !in written) || name.endsWith(PARTIAL)) {
                    runCatching { DocumentsContract.deleteDocument(resolver, doc) }
                }
            }
            writeKeyFile(resolver, tree, dir)

            // Seen names that are gone from the folder need not be kept.
            val present = blobs(resolver, tree).map { it.first }.toSet()
            prefs.edit()
                .putStringSet(KEY_SEEN, seen.filter { it in present }.toSet())
                .putStringSet(KEY_OWN, written)
                .putLong(KEY_LAST, System.currentTimeMillis())
                .apply()
            Outcome(imported, written.size, needsUpdate)
        }
    }

    /** What a folder holds for restoring: its key file and every sync blob. */
    class Contents(val keyFile: ByteArray, val blobs: List<ByteArray>)

    /**
     * Reads the folder [tree] the user picked to restore from; `null` if it
     * has no key file. Blobs that can't be read are skipped (the core also
     * leaves out those that don't open).
     */
    suspend fun readForRestore(tree: Uri): Contents? = withContext(Dispatchers.IO) {
        val resolver = context.contentResolver
        val key = child(resolver, tree, KEY_FILE)?.let { doc ->
            runCatching { resolver.openInputStream(doc)?.use { it.readBytes() } }.getOrNull()
        } ?: return@withContext null
        val blobs = blobs(resolver, tree).mapNotNull { (_, doc) ->
            runCatching { resolver.openInputStream(doc)?.use { it.readBytes() } }.getOrNull()
        }
        Contents(key, blobs)
    }

    /**
     * After restoring from [tree]: keeps syncing through it. Every blob is
     * imported again at the first sync, which changes nothing already there.
     */
    fun adopt(tree: Uri) = choose(tree)

    /** Step 4 of [sync]: the folder's key file. */
    private fun writeKeyFile(resolver: ContentResolver, tree: Uri, dir: Uri) {
        val wrapped = masterWrappedByRecovery() ?: return
        val bytes = runCatching { syncFolderKeyFile(wrapped) }.getOrNull() ?: return
        val digest = sha256(bytes)
        val existing = child(resolver, tree, KEY_FILE)
        if (existing != null) {
            // Another device's copy (the same key, wrapped separately) stays;
            // this device replaces only its own, and only when it changed.
            val current = runCatching { resolver.openInputStream(existing)?.use { it.readBytes() } }.getOrNull() ?: return
            val ours = prefs.getString(KEY_KEY_WRITTEN, null)
            if (ours != sha256(current) || ours == digest) return
            resolver.openOutputStream(existing, "wt")?.use { it.write(bytes) } ?: return
        } else {
            val doc = DocumentsContract.createDocument(resolver, dir, MIME, KEY_FILE) ?: return
            resolver.openOutputStream(doc, "wt")?.use { it.write(bytes) } ?: return
        }
        prefs.edit().putString(KEY_KEY_WRITTEN, digest).apply()
    }

    /** The document named [name] in the folder, if any. */
    private fun child(resolver: ContentResolver, tree: Uri, name: String): Uri? {
        val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
        val columns = arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        resolver.query(children, columns, null, null, null)?.use { cursor ->
            while (cursor.moveToNext()) {
                if (cursor.getString(1) == name) {
                    return cursor.getString(0)?.let { DocumentsContract.buildDocumentUriUsingTree(tree, it) }
                }
            }
        }
        return null
    }

    private fun sha256(bytes: ByteArray): String =
        MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it) }

    /** The sync blobs in the folder, as (file name, document URI). */
    private fun blobs(resolver: ContentResolver, tree: Uri, includePartial: Boolean = false): List<Pair<String, Uri>> {
        val children = DocumentsContract.buildChildDocumentsUriUsingTree(tree, DocumentsContract.getTreeDocumentId(tree))
        val columns = arrayOf(DocumentsContract.Document.COLUMN_DOCUMENT_ID, DocumentsContract.Document.COLUMN_DISPLAY_NAME)
        val found = mutableListOf<Pair<String, Uri>>()
        resolver.query(children, columns, null, null, null)?.use { cursor ->
            while (cursor.moveToNext()) {
                val id = cursor.getString(0) ?: continue
                val name = cursor.getString(1) ?: continue
                val isBlob = BLOB.matches(name) || (includePartial && BLOB.matches(name.removeSuffix(PARTIAL)) && name.endsWith(PARTIAL))
                if (isBlob) found += name to DocumentsContract.buildDocumentUriUsingTree(tree, id)
            }
        }
        return found
    }

    private companion object {
        const val KEY_FOLDER = "folder"
        const val KEY_SEEN = "seen"
        const val KEY_OWN = "own"
        const val KEY_LAST = "last"
        const val KEY_KEY_WRITTEN = "key_written"
        const val KEY_FILE = "terpoir.key"
        const val SUFFIX = ".terpoir"
        const val PARTIAL = ".partial"
        const val MIME = "application/octet-stream"
        val BLOB = Regex("[0-9a-f]{32}\\.terpoir")
    }
}
