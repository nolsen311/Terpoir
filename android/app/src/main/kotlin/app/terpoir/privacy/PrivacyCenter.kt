// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import kotlin.coroutines.cancellation.CancellationException
import androidx.annotation.StringRes

/** Privacy Center sections, in display order. */
enum class PrivacyGroup {
    JOURNAL,
    LOCATION,
    NOTIFICATIONS,
    CATALOGS,
    SCANNING,
    OS_INTEGRATION,
    KEYBOARD_AND_VOICE,
    PHOTOS,
    SYNC_AND_KEYS,
    DEVICE_SECURITY,
}

/**
 * A toggle's disclosure card, as string resource IDs: what it does, what
 * leaves the app when it is relaxed, who can then see it, and how to undo.
 */
class PrivacyCard(
    val title: Int,
    val whatItDoes: Int,
    val whatLeaves: Int,
    val whoCanSee: Int,
    val howToUndo: Int,
)

/**
 * One Privacy Center entry. It cannot exist without both actions: the
 * on-action and the off-action that cleans up whatever the on-action
 * created (indexes, geofences, shortcuts, files). Both must be idempotent,
 * because "Restore maximum privacy" runs every cleanup whatever the state.
 *
 * @property privateState the state that is most private, and the default.
 *   Usually off; on for protections such as screen protection.
 * @property isOn reads the current state.
 * @property offChoice for a toggle that collects journal data: turning it
 *   off by hand asks whether to keep that data hidden or delete it.
 * @property presetOnAction runs instead of the on-action when a preset
 *   (by ID) turns the toggle on, for a toggle whose preset state carries a
 *   choice (YOLO Mode's map provider). It must leave the toggle on.
 */
class PrivacyToggle(
    val id: String,
    val group: PrivacyGroup,
    val card: PrivacyCard,
    val privateState: Boolean,
    val isOn: () -> Boolean,
    private val onAction: suspend () -> Unit,
    private val offAction: suspend () -> Unit,
    val offChoice: OffChoice? = null,
    private val presetOnAction: (suspend (presetId: String) -> Unit)? = null,
) {
    /** Whether the toggle is in its most private state. */
    val isPrivate: Boolean get() = isOn() == privateState

    /** Runs the on- or off-action, then checks that it took effect. */
    suspend fun set(on: Boolean) {
        if (on) onAction() else offAction()
        check(isOn() == on) { "Privacy toggle $id did not turn ${if (on) "on" else "off"}" }
    }

    /** As [set], for preset [presetId]. */
    suspend fun setForPreset(on: Boolean, presetId: String) {
        val forPreset = presetOnAction
        if (!on || forPreset == null) return set(on)
        forPreset(presetId)
        check(isOn()) { "Privacy toggle $id did not turn on" }
    }
}

/**
 * The question asked when a toggle that collects journal data is turned off
 * by hand: keep the data, hidden, or delete it ([delete] runs after the
 * off-action). Presets and "Restore maximum privacy" only hide; nothing is
 * deleted without asking.
 */
class OffChoice(
    @StringRes val question: Int,
    @StringRes val hideLabel: Int,
    @StringRes val deleteLabel: Int,
    val delete: suspend () -> Unit,
)

/**
 * A preset: a full set of toggle states (docs/design-decisions.md, "Presets
 * and YOLO Mode"). The toggles in [relaxed] leave their most private state;
 * every other toggle is at its most private state. IDs this build does not
 * register are ignored. A preset never bypasses a toggle: applying one runs
 * each toggle's own action.
 */
class PrivacyPreset(
    val id: String,
    @StringRes val title: Int,
    @StringRes val description: Int,
    private val relaxed: Set<String>,
    /** Settings that aren't privacy toggles, set too, after the toggles; listed in the confirmation. */
    val extras: List<PresetExtra> = emptyList(),
) {
    /** The state this preset gives [toggle]. */
    fun stateFor(toggle: PrivacyToggle): Boolean =
        if (toggle.id in relaxed) !toggle.privateState else toggle.privateState
}

/**
 * A setting a preset also sets that isn't a privacy toggle (a journal or
 * lock setting): [label] in the preset's confirmation, [apply] when it is
 * chosen. Leaving the preset doesn't undo it, since it relaxes nothing.
 */
class PresetExtra(@StringRes val label: Int, val apply: suspend () -> Unit)

/**
 * Every privacy-relevant toggle, in one registry, and the presets over
 * them. See "Privacy Center" in docs/design-decisions.md.
 */
class PrivacyCenter(val toggles: List<PrivacyToggle>, val presets: List<PrivacyPreset> = emptyList()) {
    init {
        val duplicates = toggles.groupBy { it.id }.filterValues { it.size > 1 }.keys
        require(duplicates.isEmpty()) { "Duplicate privacy toggle IDs: $duplicates" }
        require(presets.map { it.id }.toSet().size == presets.size) { "Duplicate preset IDs" }
    }

    /** The preset every toggle currently matches, or `null` for "Custom". */
    fun currentPreset(): PrivacyPreset? =
        presets.firstOrNull { preset -> toggles.all { it.isOn() == preset.stateFor(it) } }

    /** The toggles [preset] would change, each with the state it would set. */
    fun changes(preset: PrivacyPreset): List<Pair<PrivacyToggle, Boolean>> =
        toggles.mapNotNull { toggle -> preset.stateFor(toggle).takeIf { it != toggle.isOn() }?.let { toggle to it } }

    /**
     * Sets every toggle to [preset]'s state by running its action, including
     * toggles already there (as [restoreMaximumPrivacy] does): leaving a
     * preset runs the off-actions of everything it turned on.
     */
    suspend fun apply(preset: PrivacyPreset) {
        for (toggle in toggles) toggle.setForPreset(preset.stateFor(toggle), preset.id)
        for (extra in preset.extras) extra.apply()
    }

    /** Toggles by section, in display order; empty sections are left out. */
    val groups: Map<PrivacyGroup, List<PrivacyToggle>>
        get() = toggles.groupBy { it.group }.toSortedMap()

    /** How many toggles are not in their most private state. */
    fun relaxedCount(): Int = toggles.count { !it.isPrivate }

    /**
     * Returns every toggle to its most private state by running its action
     * for that state, including toggles already there, so leftovers from
     * an interrupted cleanup are removed too.
     */
    suspend fun restoreMaximumPrivacy() {
        // Every toggle gets its turn even if one fails, so a broken cleanup
        // can't leave the rest relaxed; the first failure is reported after.
        var failure: Throwable? = null
        for (toggle in toggles) {
            try {
                toggle.set(toggle.privateState)
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                if (failure == null) failure = e else failure.addSuppressed(e)
            }
        }
        failure?.let { throw it }
    }
}
