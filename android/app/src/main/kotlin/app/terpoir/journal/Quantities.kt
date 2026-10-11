// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

/*
 * Converting what the user types to the core's fixed-point amounts
 * (thousandths of a unit) and back. Pure functions, tested on the JVM.
 */

/** The result of parsing a typed amount. */
sealed interface Parsed {
    /** Nothing typed. */
    data object Empty : Parsed

    /** Not a non-negative number with at most three decimals. */
    data object Invalid : Parsed

    /** The amount in thousandths. */
    data class Milli(val value: Long) : Parsed
}

private val AMOUNT = Regex("""(\d{1,9})(?:[.,](\d{1,3}))?""")

/** Parses "1.5" or "1,5" to 1500 thousandths. */
fun parseMilli(text: String): Parsed {
    val trimmed = text.trim()
    if (trimmed.isEmpty()) return Parsed.Empty
    val match = AMOUNT.matchEntire(trimmed) ?: return Parsed.Invalid
    val whole = match.groupValues[1].toLong()
    val fraction = match.groupValues[2].padEnd(3, '0').toLong()
    return Parsed.Milli(whole * 1000 + fraction)
}

/** Formats 1500 thousandths as "1.5", 2000 as "2". */
fun formatMilli(milli: Long): String {
    val whole = milli / 1000
    val fraction = milli % 1000
    if (fraction == 0L) return whole.toString()
    return "$whole." + fraction.toString().padStart(3, '0').trimEnd('0')
}

/** Parses a whole number of minutes; null when empty. */
fun parseMinutes(text: String): Parsed {
    val trimmed = text.trim()
    if (trimmed.isEmpty()) return Parsed.Empty
    val minutes = trimmed.toIntOrNull()?.takeIf { it in 0..MAX_MINUTES } ?: return Parsed.Invalid
    return Parsed.Milli(minutes.toLong())
}

/** A week: longer timelines are typos. */
private const val MAX_MINUTES = 7 * 24 * 60
