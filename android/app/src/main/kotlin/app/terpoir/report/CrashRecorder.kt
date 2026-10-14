// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.report

import android.content.Context
import java.io.File

/**
 * Keeps a short summary of the last crash so the next launch can offer to
 * report it (docs/design-decisions.md, "Problem reports"). The summary is
 * the error's type and the code locations involved (class, method, file
 * and line), for it and its causes. It never holds an error's message,
 * which could name a file or carry other details, and nothing from the
 * journal. It lives in app-private storage, outside backups, until the
 * user sends or dismisses it.
 */
object CrashRecorder {
    private const val FILE = "last-crash.txt"
    private const val MAX_FRAMES = 40

    private fun file(context: Context) = File(context.noBackupFilesDir, FILE)

    /** Records uncaught errors, then lets Android handle them as before. */
    fun install(context: Context) {
        val app = context.applicationContext
        val previous = Thread.getDefaultUncaughtExceptionHandler()
        Thread.setDefaultUncaughtExceptionHandler { thread, error ->
            runCatching { file(app).writeText(summary(error)) }
            previous?.uncaughtException(thread, error)
        }
    }

    /** The last crash's summary, if one is waiting to be reported or dismissed. */
    fun pending(context: Context): String? = file(context).takeIf { it.exists() }?.let { runCatching { it.readText() }.getOrNull() }

    /** Forgets the last crash. */
    fun clear(context: Context) {
        file(context).delete()
    }

    /** The error's types and stack locations, without any message. */
    fun summary(error: Throwable): String = buildString {
        var current: Throwable? = error
        var depth = 0
        var frames = 0
        while (current != null && depth < 5) {
            appendLine(if (depth == 0) current.javaClass.name else "Caused by ${current.javaClass.name}")
            for (frame in current.stackTrace) {
                if (frames++ >= MAX_FRAMES) break
                appendLine("  at ${frame.className}.${frame.methodName}(${frame.fileName ?: "?"}:${frame.lineNumber})")
            }
            current = current.cause?.takeIf { it !== current }
            depth++
        }
    }
}
