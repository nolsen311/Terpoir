// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.map

import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.Dp
import app.terpoir.location.Fix

/**
 * Google Maps for the map picker: Play builds with a Google Maps key only
 * (`Distribution.googleMaps`). The foss build never has it.
 */
interface GoogleMapSupport {
    /**
     * A Google map filling its space, starting at [start] (or the whole
     * world). Once it is ready, [onReady] gets a function returning the
     * point under the centre, with the ground [crosshairRadius] covers at
     * the current zoom as its accuracy. Call only while the online map is
     * on and Google is the provider: creating it contacts Google.
     */
    @Composable
    fun PickerMap(start: Fix?, crosshairRadius: Dp, onReady: (pick: () -> Fix) -> Unit)
}
