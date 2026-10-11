// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

/** Privacy Center sections, in display order. */
enum class PrivacyGroup {
    LOCATION,
    NOTIFICATIONS,
    CATALOGS,
    OS_INTEGRATION,
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
 */
class PrivacyToggle(
    val id: String,
    val group: PrivacyGroup,
    val card: PrivacyCard,
    val privateState: Boolean,
    val isOn: () -> Boolean,
    private val onAction: suspend () -> Unit,
    private val offAction: suspend () -> Unit,
) {
    /** Whether the toggle is in its most private state. */
    val isPrivate: Boolean get() = isOn() == privateState

    /** Runs the on- or off-action, then checks that it took effect. */
    suspend fun set(on: Boolean) {
        if (on) onAction() else offAction()
        check(isOn() == on) { "Privacy toggle $id did not turn ${if (on) "on" else "off"}" }
    }
}

/**
 * Every privacy-relevant toggle, in one registry. See "Privacy Center" in
 * docs/design-decisions.md.
 */
class PrivacyCenter(val toggles: List<PrivacyToggle>) {
    init {
        val duplicates = toggles.groupBy { it.id }.filterValues { it.size > 1 }.keys
        require(duplicates.isEmpty()) { "Duplicate privacy toggle IDs: $duplicates" }
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
        for (toggle in toggles) toggle.set(toggle.privateState)
    }
}
