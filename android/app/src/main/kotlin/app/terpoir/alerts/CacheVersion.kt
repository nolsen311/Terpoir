// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.alerts

import java.io.File

/**
 * Which version of the rules built what alerts keep outside the journal
 * (docs/design-decisions.md, "Schema upgrades"). Kept in [file], in the
 * no-backup directory next to the notification cache; no file means
 * version 0 (before versions were kept).
 */
class CacheVersion(private val file: File, private val current: Int) {
    /** Whether the cache and fences were built by this version's rules. */
    val isCurrent: Boolean
        get() = stored() == current

    /** After a rebuild by this version's rules. */
    fun markCurrent() {
        if (isCurrent) return
        file.parentFile?.mkdirs()
        val partial = File(file.path + ".partial")
        partial.writeText(current.toString())
        if (!partial.renameTo(file)) partial.delete()
    }

    private fun stored(): Int =
        if (!file.exists()) 0 else runCatching { file.readText().trim().toInt() }.getOrDefault(0)
}
