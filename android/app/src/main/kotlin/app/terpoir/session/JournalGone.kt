// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.session

import app.terpoir.core.CoreException
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * Whether this means the journal locked while a call was on its way: the
 * core refuses a locked journal, and a closed one (locking closes it) is
 * refused by the bindings with an [IllegalStateException].
 */
fun Throwable.isJournalGone(): Boolean =
    this is CoreException.Locked ||
        (this is IllegalStateException && message?.endsWith("has already been destroyed") == true)

/**
 * Runs [block] on the IO dispatcher. If the journal locks before or while
 * it runs, the caller is cancelled instead of failing: locking replaces the
 * whole UI, so the result has nowhere to go.
 */
suspend fun <T> onJournal(block: () -> T): T = withContext(Dispatchers.IO) {
    try {
        block()
    } catch (e: Exception) {
        if (e.isJournalGone()) throw CancellationException("The journal locked").apply { initCause(e) }
        throw e
    }
}
