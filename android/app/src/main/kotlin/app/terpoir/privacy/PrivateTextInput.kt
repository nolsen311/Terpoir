// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import android.view.inputmethod.EditorInfo
import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.InterceptPlatformTextInput
import androidx.compose.ui.platform.PlatformTextInputMethodRequest

/**
 * Asks the keyboard not to learn from anything typed in [content]: every
 * text field inside gets `IME_FLAG_NO_PERSONALIZED_LEARNING`. The
 * keyboard itself stays the user's choice. Third-party keyboards may
 * ignore the request; see "Keyboards" in docs/design-decisions.md.
 */
@Composable
fun PrivateTextInput(content: @Composable () -> Unit) {
    InterceptPlatformTextInput(
        interceptor = { request, nextHandler ->
            nextHandler.startInputMethod(
                PlatformTextInputMethodRequest { outAttrs ->
                    request.createInputConnection(outAttrs).also { applyPrivateImeOptions(outAttrs) }
                },
            )
        },
        content = content,
    )
}

/** Adds the no-learning flag to a text field's [EditorInfo]. */
fun applyPrivateImeOptions(outAttrs: EditorInfo) {
    outAttrs.imeOptions = outAttrs.imeOptions or EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
}
