// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.journal

import app.terpoir.core.TagRefData
import app.terpoir.core.TagSuggestion
import org.junit.Assert.assertEquals
import org.junit.Test

class NoteTagsTest {
    private fun suggestion(start: Int, end: Int) =
        TagSuggestion(TagRefData.Builtin("relaxed"), "effect", start.toUInt(), end.toUInt())

    @Test
    fun quotesThePhraseByUtf16Span() {
        // The core counts UTF-16 units, as Kotlin strings do: the emoji is two.
        val text = "😀 chill vibes"
        assertEquals("chill", phraseOf(text, suggestion(3, 8)))
    }

    @Test
    fun aStaleSpanIsClampedNotThrown() {
        // The text can change while a suggestion is on screen.
        assertEquals("ill", phraseOf("chill", suggestion(2, 99)))
        assertEquals("", phraseOf("hi", suggestion(9, 12)))
    }
}
