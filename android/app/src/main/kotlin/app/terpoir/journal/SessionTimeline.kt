// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import android.content.Context
import android.provider.Settings
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.layout.positionInRoot
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringArrayResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.ProgressBarRangeInfo
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.progressBarRangeInfo
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.setProgress
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import app.terpoir.R
import app.terpoir.core.TimelinePlanData
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlinx.coroutines.delay

/** The indicator's size. */
private val BudSize = 112.dp

/** The help's text over the dimmed screen, in either theme. */
private val ScrimInk = Color(0xFFF3EFE7)

/** How much of the finger's drag the indicator follows. */
private const val DRAG_FOLLOW = 0.6f

/** How far the indicator flies after release, in seconds of its throw. */
private const val FLIGHT_SECONDS = 0.25f

/**
 * The press-and-flick on the timeline's indicator, shared with the
 * screen's [TimelineHost], which dims the screen while it lasts.
 */
@Stable
class TimelineGesture {
    /** The finger is down on the indicator. */
    var holding by mutableStateOf(false)
        internal set

    /** The finger has started moving; the hold is over. */
    var moving by mutableStateOf(false)
        internal set

    /** How much of a full hold has passed, 0 to 1. */
    var heldFraction by mutableFloatStateOf(0f)
        internal set

    /** The indicator's centre in the root. */
    internal var centre by mutableStateOf(Offset.Zero)

    /** How far the indicator follows the finger. */
    internal var drag by mutableStateOf(Offset.Zero)

    /** After release: the throw (px per second) while the indicator flies off. */
    internal var flight by mutableStateOf<Offset?>(null)

    /** Applies the gesture's result once the indicator has flown off. */
    internal var onLanded: (() -> Unit)? = null
}

/** A [TimelineGesture] for this screen. */
@Composable
fun rememberTimelineGesture(): TimelineGesture = remember { TimelineGesture() }

/**
 * A screen with a timeline. While its indicator is held, the screen dims
 * and the help shows over it, with the indicator on top; after release,
 * the indicator flies off the way it was thrown.
 */
@Composable
fun TimelineHost(gesture: TimelineGesture, content: @Composable () -> Unit) {
    var origin by remember { mutableStateOf(Offset.Zero) }
    Box(Modifier.fillMaxSize().onGloballyPositioned { origin = it.positionInRoot() }) {
        content()
        if (gesture.holding || gesture.flight != null) Overlay(gesture, origin)
    }
}

@Composable
private fun Overlay(gesture: TimelineGesture, origin: Offset) {
    val context = LocalContext.current
    val scrim by animateFloatAsState(if (gesture.holding) 0.62f else 0f, label = "scrim")
    val fly = remember { Animatable(0f) }
    val flight = gesture.flight
    LaunchedEffect(flight) {
        if (flight == null) return@LaunchedEffect
        fly.snapTo(0f)
        if (animationsOn(context)) fly.animateTo(1f, tween(380, easing = FastOutSlowInEasing))
        gesture.flight = null
        gesture.onLanded?.invoke()
        gesture.onLanded = null
    }
    Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = scrim)))
    if (gesture.holding) {
        val levels = stringArrayResource(R.array.timeline_levels)
        Column(
            Modifier.fillMaxWidth().statusBarsPadding().padding(horizontal = 24.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(stringResource(R.string.timeline_help_title), style = MaterialTheme.typography.headlineSmall, color = ScrimInk)
            Text(stringResource(R.string.timeline_help_body), style = MaterialTheme.typography.bodyLarge, color = ScrimInk)
            val now = holdLevel((gesture.heldFraction * FULL_HOLD_MS).toLong())
            Text(stringResource(R.string.timeline_now, levels[now]), style = MaterialTheme.typography.labelLarge, color = ScrimInk)
        }
    }
    val sizePx = with(LocalDensity.current) { BudSize.toPx() }
    val thrown = (flight ?: Offset.Zero) * FLIGHT_SECONDS * fly.value
    val at = gesture.centre - origin + gesture.drag + thrown - Offset(sizePx / 2, sizePx / 2)
    Bud(
        heldFraction = gesture.heldFraction,
        modifier = Modifier
            .offset { IntOffset(at.x.roundToInt(), at.y.roundToInt()) }
            .graphicsLayer {
                alpha = 1f - fly.value
                scaleX = 1f - 0.4f * fly.value
                scaleY = 1f - 0.4f * fly.value
                rotationZ = (flight?.x ?: 0f) / 40f * fly.value
            },
    )
}

/**
 * The session's timeline (docs/design-decisions.md, "Session timeline"):
 * how strong it felt at points after consumption, 0 to 5. With nothing set
 * yet, an indicator to hold and flick sets the first points; then a chart
 * whose points can be dragged, the unset ones shown as a guess.
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
fun TimelineSection(
    plan: TimelinePlanData,
    levels: Map<Int, Int>,
    updatedMs: Long?,
    gesture: TimelineGesture,
    onChange: (Map<Int, Int>) -> Unit,
) {
    var byHand by remember { mutableStateOf(false) }
    // The flick's direction, for this visit's guess only; never saved.
    var trend by remember { mutableIntStateOf(0) }
    val minutes = plan.pointMinutes.map { it.toInt() }
    if (levels.isEmpty() && !byHand) {
        Indicator(
            gesture = gesture,
            onRelease = { held, up ->
                trend = flickStep(up)
                onChange(gestureLevels(minutes, plan.gesturePoints.toInt(), held, up))
            },
            onByHand = { byHand = true },
        )
        return
    }
    if (updatedMs != null) UpdatedAgo(updatedMs)
    val points = timelinePoints(minutes, levels, trend)
    TimelineChart(points) { minute, level -> if (levels[minute] != level) onChange(levels + (minute to level)) }
    val names = stringArrayResource(R.array.timeline_levels)
    FlowRow(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        for (p in points) {
            Text(
                "${durationLabel(p.minute)}: ${if (p.set) "" else "~"}${names[p.level]}",
                style = MaterialTheme.typography.labelMedium,
                fontWeight = if (p.set) FontWeight.SemiBold else FontWeight.Normal,
                color = if (p.set) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
    Text(stringResource(R.string.timeline_hint), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    TextButton(onClick = {
        byHand = false
        trend = 0
        onChange(emptyMap())
    }) { Text(stringResource(R.string.timeline_start_over)) }
}

/** The indicator to hold and flick, with a way to set the points by hand instead. */
@Composable
private fun Indicator(gesture: TimelineGesture, onRelease: (heldMs: Long, upDpPerSecond: Float) -> Unit, onByHand: () -> Unit) {
    val haptics = LocalHapticFeedback.current
    val release by rememberUpdatedState(onRelease)
    val byHand = stringResource(R.string.timeline_by_hand)
    val description = stringResource(R.string.timeline_indicator)
    LaunchedEffect(gesture.holding, gesture.moving) {
        if (!gesture.holding || gesture.moving) return@LaunchedEffect
        val start = withFrameMillis { it }
        while (true) {
            withFrameMillis { t -> gesture.heldFraction = ((t - start).toFloat() / FULL_HOLD_MS).coerceAtMost(1f) }
        }
    }
    Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Box(Modifier.fillMaxWidth().height(172.dp), contentAlignment = Alignment.Center) {
            Bud(
                heldFraction = 0f,
                modifier = Modifier
                    .onGloballyPositioned { gesture.centre = it.boundsInRoot().center }
                    // While held or flying, the host draws it over the dimmed screen.
                    .graphicsLayer { alpha = if (gesture.holding || gesture.flight != null) 0f else 1f }
                    .semantics {
                        contentDescription = description
                        onClick(label = byHand) {
                            onByHand()
                            true
                        }
                    }
                    .pointerInput(Unit) {
                        awaitEachGesture {
                            val down = awaitFirstDown()
                            down.consume()
                            haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                            gesture.heldFraction = 0f
                            gesture.drag = Offset.Zero
                            gesture.moving = false
                            gesture.holding = true
                            val tracker = VelocityTracker()
                            tracker.addPosition(down.uptimeMillis, down.position)
                            var total = Offset.Zero
                            var movedAt: Long? = null
                            var endAt = down.uptimeMillis
                            while (true) {
                                val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                                endAt = change.uptimeMillis
                                change.consume()
                                if (!change.pressed) break
                                tracker.addPosition(change.uptimeMillis, change.position)
                                total += change.positionChange()
                                if (movedAt == null && total.getDistance() > viewConfiguration.touchSlop) {
                                    movedAt = change.uptimeMillis
                                    gesture.moving = true
                                }
                                gesture.drag = total * DRAG_FOLLOW
                            }
                            val velocity = tracker.calculateVelocity()
                            val held = (movedAt ?: endAt) - down.uptimeMillis
                            val up = -velocity.y / density
                            gesture.holding = false
                            gesture.onLanded = { release(held, up) }
                            gesture.flight = Offset(velocity.x, velocity.y)
                        }
                    },
            )
        }
        Text(stringResource(R.string.timeline_prompt), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        TextButton(onClick = onByHand) { Text(byHand) }
    }
}

/** A little green bud with rising bubbles, like the app's mark, that wobbles to be touched. */
@Composable
private fun Bud(heldFraction: Float, modifier: Modifier) {
    val context = LocalContext.current
    val primary = MaterialTheme.colorScheme.primary
    val surface = MaterialTheme.colorScheme.surface
    val ring = MaterialTheme.colorScheme.tertiary
    val moving = remember { animationsOn(context) }
    val transition = rememberInfiniteTransition(label = "bud")
    val wobble by transition.animateFloat(0f, if (moving) 1f else 0f, infiniteRepeatable(tween(2_600, easing = LinearEasing)), label = "wobble")
    val rise by transition.animateFloat(0f, if (moving) 1f else 0f, infiniteRepeatable(tween(2_800, easing = LinearEasing)), label = "rise")
    Box(modifier.size(BudSize), contentAlignment = Alignment.Center) {
        Canvas(Modifier.matchParentSize()) {
            val r = size.minDimension / 2 * 0.66f
            val w = sin(2 * PI * wobble).toFloat()
            scale(1f + 0.04f * w, 1f - 0.03f * w) {
                drawCircle(
                    Brush.radialGradient(
                        listOf(lerp(primary, Color.White, 0.4f), primary),
                        center = center + Offset(-r * 0.3f, -r * 0.4f),
                        radius = r * 1.6f,
                    ),
                    radius = r,
                )
            }
            // Three bubbles rising off its top right, a third of a cycle apart.
            val stroke = 2.dp.toPx()
            listOf(6.dp, 4.dp, 3.dp).forEachIndexed { i, size ->
                val phase = (rise + i / 3f) % 1f
                val alpha = if (phase < 0.25f) phase * 4 else 1f - (phase - 0.25f) / 0.75f
                val at = Offset(center.x + r * (0.55f + 0.18f * i), center.y - r * 0.85f - phase * 22.dp.toPx())
                drawCircle(surface, radius = size.toPx(), center = at, alpha = alpha)
                drawCircle(primary, radius = size.toPx(), center = at, alpha = alpha, style = Stroke(stroke))
            }
            if (heldFraction > 0f) {
                val ringR = r + 9.dp.toPx()
                drawArc(
                    ring,
                    startAngle = -90f,
                    sweepAngle = 360f * heldFraction,
                    useCenter = false,
                    topLeft = center - Offset(ringR, ringR),
                    size = Size(ringR * 2, ringR * 2),
                    style = Stroke(5.dp.toPx(), cap = StrokeCap.Round),
                )
            }
        }
        Text(stringResource(R.string.timeline_touch), color = MaterialTheme.colorScheme.onPrimary, style = MaterialTheme.typography.labelMedium)
    }
}

/** The chart: the points along the timeline, dragged up or down to set them. */
@Composable
private fun TimelineChart(points: List<TimelinePoint>, onSet: (minute: Int, level: Int) -> Unit) {
    val line = MaterialTheme.colorScheme.primary
    val grid = MaterialTheme.colorScheme.outlineVariant
    val card = MaterialTheme.colorScheme.surface
    val labelStyle = MaterialTheme.typography.labelSmall.copy(color = MaterialTheme.colorScheme.onSurfaceVariant)
    val measurer = rememberTextMeasurer()
    val labels = points.map { durationLabel(it.minute) }
    val names = stringArrayResource(R.array.timeline_levels)
    val guess = stringResource(R.string.timeline_guess)
    val current by rememberUpdatedState(points)
    val set by rememberUpdatedState(onSet)
    val density = LocalDensity.current
    BoxWithConstraints(Modifier.fillMaxWidth().height(184.dp)) {
        val left = with(density) { 22.dp.toPx() }
        val right = with(density) { 14.dp.toPx() }
        val top = with(density) { 12.dp.toPx() }
        val bottom = with(density) { 26.dp.toPx() }
        val width = with(density) { maxWidth.toPx() }
        val height = with(density) { maxHeight.toPx() }
        val span = (points.maxOfOrNull { it.minute } ?: 1).coerceAtLeast(1)
        fun x(minute: Int) = left + (width - left - right) * minute / span
        fun y(level: Int) = top + (height - top - bottom) * (1 - level.toFloat() / MAX_LEVEL)
        fun levelAt(py: Float) = (MAX_LEVEL * (1 - (py - top) / (height - top - bottom))).roundToInt().coerceIn(0, MAX_LEVEL)

        Canvas(
            Modifier
                .matchParentSize()
                .clearAndSetSemantics { }
                .pointerInput(Unit) {
                    awaitEachGesture {
                        val down = awaitFirstDown()
                        // Only a touch on a point moves it, so scrolling
                        // past the chart leaves the points alone.
                        val reach = 28.dp.toPx()
                        val target = current
                            .filter { (Offset(x(it.minute), y(it.level)) - down.position).getDistance() <= reach }
                            .minByOrNull { abs(x(it.minute) - down.position.x) }
                            ?: return@awaitEachGesture
                        down.consume()
                        var moved = false
                        while (true) {
                            val change = awaitPointerEvent().changes.firstOrNull { it.id == down.id } ?: break
                            change.consume()
                            if (!change.pressed) {
                                // A tap keeps a guessed point's level and sets it.
                                if (!moved) set(target.minute, target.level)
                                break
                            }
                            moved = true
                            set(target.minute, levelAt(change.position.y))
                        }
                    }
                },
        ) {
            for (level in 0..MAX_LEVEL) {
                drawLine(
                    grid,
                    Offset(left, y(level)),
                    Offset(size.width - right, y(level)),
                    strokeWidth = 1.dp.toPx(),
                    pathEffect = if (level == 0) null else PathEffect.dashPathEffect(floatArrayOf(2.dp.toPx(), 4.dp.toPx())),
                )
            }
            for ((text, level) in listOf("0" to 0, "$MAX_LEVEL" to MAX_LEVEL)) {
                val m = measurer.measure(text, labelStyle)
                drawText(m, topLeft = Offset(left - m.size.width - 6.dp.toPx(), y(level) - m.size.height / 2))
            }
            points.forEachIndexed { i, p ->
                val m = measurer.measure(labels[i], labelStyle)
                val cx = x(p.minute).coerceIn(m.size.width / 2f, size.width - m.size.width / 2f)
                drawText(m, topLeft = Offset(cx - m.size.width / 2, size.height - m.size.height))
            }
            val all = listOf(Offset(x(0), y(0))) + points.map { Offset(x(it.minute), y(it.level)) }
            val lastSet = points.indexOfLast { it.set } + 1
            drawPath(smooth(all).apply {
                lineTo(all.last().x, y(0))
                close()
            }, line, alpha = 0.12f)
            if (lastSet > 0) drawPath(smooth(all.take(lastSet + 1)), line, style = Stroke(3.dp.toPx(), cap = StrokeCap.Round))
            if (lastSet < all.size - 1) {
                drawPath(
                    smooth(all.drop(lastSet)),
                    line,
                    alpha = 0.65f,
                    style = Stroke(2.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(5.dp.toPx(), 6.dp.toPx()))),
                )
            }
            points.forEachIndexed { i, p ->
                val at = all[i + 1]
                drawCircle(if (p.set) line else card, radius = 7.dp.toPx(), center = at)
                drawCircle(line, radius = 7.dp.toPx(), center = at, style = Stroke(2.5.dp.toPx()))
            }
        }
        // One adjustable node per point for screen readers.
        points.forEachIndexed { i, p ->
            val cx = with(density) { x(p.minute).toDp() }
            Box(
                Modifier
                    .offset(x = cx - 20.dp)
                    .size(40.dp, maxHeight)
                    .semantics {
                        contentDescription = labels[i]
                        stateDescription = if (p.set) names[p.level] else "${names[p.level]}, $guess"
                        progressBarRangeInfo = ProgressBarRangeInfo(p.level.toFloat(), 0f..MAX_LEVEL.toFloat(), steps = MAX_LEVEL - 1)
                        setProgress { value ->
                            set(p.minute, value.roundToInt().coerceIn(0, MAX_LEVEL))
                            true
                        }
                    },
            )
        }
    }
}

/** A smooth line through [points] (Catmull-Rom as cubic Béziers). */
private fun smooth(points: List<Offset>): Path = Path().apply {
    if (points.isEmpty()) return@apply
    moveTo(points[0].x, points[0].y)
    for (i in 0 until points.size - 1) {
        val p0 = points.getOrElse(i - 1) { points[i] }
        val p1 = points[i]
        val p2 = points[i + 1]
        val p3 = points.getOrElse(i + 2) { p2 }
        cubicTo(
            p1.x + (p2.x - p0.x) / 6, p1.y + (p2.y - p0.y) / 6,
            p2.x - (p3.x - p1.x) / 6, p2.y - (p3.y - p1.y) / 6,
            p2.x, p2.y,
        )
    }
}

/** "You last updated 2 min ago", kept current while shown. */
@Composable
private fun UpdatedAgo(updatedMs: Long) {
    var now by remember { mutableLongStateOf(System.currentTimeMillis()) }
    LaunchedEffect(updatedMs) {
        while (true) {
            now = System.currentTimeMillis()
            delay(15_000)
        }
    }
    val minutes = ((now - updatedMs) / 60_000).coerceAtLeast(0).toInt()
    val text = when {
        minutes < 1 -> stringResource(R.string.timeline_updated_now)
        minutes < 60 -> pluralStringResource(R.plurals.timeline_updated_minutes, minutes, minutes)
        minutes < 48 * 60 -> pluralStringResource(R.plurals.timeline_updated_hours, minutes / 60, minutes / 60)
        else -> pluralStringResource(R.plurals.timeline_updated_days, minutes / 1440, minutes / 1440)
    }
    Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.tertiary)
}

/** A time after consumption: "20 min", "1 h", "1 h 36". */
@Composable
private fun durationLabel(minutes: Int): String = when {
    minutes < 60 -> stringResource(R.string.timeline_minutes, minutes)
    minutes % 60 == 0 -> stringResource(R.string.timeline_hours, minutes / 60)
    else -> stringResource(R.string.timeline_hours_minutes, minutes / 60, minutes % 60)
}

/** False when the user turned animations off (animator duration scale 0). */
private fun animationsOn(context: Context): Boolean =
    Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f) != 0f
