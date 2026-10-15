// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import kotlin.math.abs
import kotlin.math.pow
import kotlin.math.roundToInt
import kotlin.math.sign

/** The strongest level on the timeline (0 is nothing). */
const val MAX_LEVEL = 5

/** A hold this long or longer means it's hitting as hard as it gets. */
const val FULL_HOLD_MS = 2_000L

/** An upward flick this fast (dp per second) counts as the steepest climb. */
private const val FULL_FLICK_DP_S = 1_600f

/** Slower than this share of a full flick counts as letting go: steady. */
private const val STEADY_SHARE = 0.12f

/** The biggest change a flick makes, in levels. */
private const val MAX_STEP = 3

/** How strong it is now, from how long the indicator was held (1 to 5). */
fun holdLevel(heldMs: Long): Int =
    (1 + 4 * (heldMs.coerceIn(0, FULL_HOLD_MS).toFloat() / FULL_HOLD_MS)).roundToInt().coerceIn(1, MAX_LEVEL)

/**
 * Which way it's heading, from the flick's upward speed: up (positive) if
 * it's still climbing, down if it's easing off, 0 when let go without one.
 */
fun flickStep(upDpPerSecond: Float): Int {
    val share = (upDpPerSecond / FULL_FLICK_DP_S).coerceIn(-1f, 1f)
    if (abs(share) < STEADY_SHARE) return 0
    return share.sign.toInt() * (abs(share) * MAX_STEP).roundToInt().coerceAtLeast(1)
}

/**
 * The levels the first gesture sets: the first point from the hold, the
 * second (when the plan has the gesture set two) from the flick.
 */
fun gestureLevels(pointMinutes: List<Int>, gesturePoints: Int, heldMs: Long, upDpPerSecond: Float): Map<Int, Int> {
    val first = holdLevel(heldMs)
    val levels = listOf(first, (first + flickStep(upDpPerSecond)).coerceIn(0, MAX_LEVEL))
    return pointMinutes.take(gesturePoints.coerceIn(1, 2)).zip(levels).toMap()
}

/** A point to draw: one the user set, or a guess (never saved). */
data class TimelinePoint(val minute: Int, val level: Int, val set: Boolean)

/**
 * Every point to show: the plan's and any set elsewhere (a session whose
 * methods changed). Set points are as saved; the rest are guessed:
 * between set points, a straight line; before the first, rising from 0;
 * after the last, easing off toward the end. [trend] is the flick's step
 * from this visit, if any: rising, the next point is one higher.
 */
fun timelinePoints(pointMinutes: List<Int>, levels: Map<Int, Int>, trend: Int = 0): List<TimelinePoint> {
    val minutes = (pointMinutes + levels.keys).distinct().sorted()
    if (levels.isEmpty()) return minutes.map { TimelinePoint(it, 0, set = false) }
    val setMinutes = levels.keys.sorted()
    val last = setMinutes.last()
    val lastLevel = levels.getValue(last)
    val before = setMinutes.dropLast(1).lastOrNull()?.let { levels.getValue(it) }
    val rising = trend > 0 || (trend == 0 && before != null && lastLevel > before)
    val after = minutes.filter { it > last }
    val top = if (rising) (lastLevel + 1).coerceAtMost(MAX_LEVEL) else lastLevel
    return minutes.map { m ->
        levels[m]?.let { return@map TimelinePoint(m, it, set = true) }
        val level = if (m < last) {
            val lo = setMinutes.lastOrNull { it < m }
            val hi = setMinutes.first { it > m }
            val (fromMin, fromLevel) = if (lo == null) 0 to 0 else lo to levels.getValue(lo)
            val share = (m - fromMin).toFloat() / (hi - fromMin)
            (fromLevel + (levels.getValue(hi) - fromLevel) * share).roundToInt()
        } else {
            val j = after.indexOf(m) + 1
            if (j == 1 && rising) {
                top
            } else {
                val f = j.toFloat() / (after.size + 1)
                (top * (1 - f.pow(1.4f)) + 0.3f).roundToInt()
            }
        }
        TimelinePoint(m, level.coerceIn(0, MAX_LEVEL), set = false)
    }
}

/**
 * A timeline as a share shows it, from the points the user set and never
 * the guesses: "20 min 2/5 · 40 min 4/5 · 1 h 3/5", earliest first.
 * [minuteLabel] names a time after consumption ("20 min", "1 h").
 */
fun timelineShareValue(levels: Map<Int, Int>, minuteLabel: (Int) -> String): String =
    levels.toSortedMap().entries.joinToString(" · ") { (minute, level) ->
        "${minuteLabel(minute)} ${level.coerceIn(0, MAX_LEVEL)}/$MAX_LEVEL"
    }
