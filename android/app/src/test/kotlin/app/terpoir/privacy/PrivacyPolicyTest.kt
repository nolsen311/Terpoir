// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class PrivacyPolicyTest {
    @Test
    fun splitsMainIntoBlocks() {
        val page = """
            <header><a href="/">Terpoir</a></header>
            <main><h1>Privacy policy</h1>
              <p class="meta">Effective
                today.</p>
              <h2>Who</h2><ul><li><strong>Camera</strong>: scans.</li></ul>
              <h3>Maps</h3></main>
            <footer><p>Footer</p></footer>
        """.trimIndent()
        assertEquals(
            listOf(
                PolicyBlock(PolicyBlockKind.TITLE, "Privacy policy"),
                PolicyBlock(PolicyBlockKind.PARAGRAPH, "Effective today."),
                PolicyBlock(PolicyBlockKind.HEADING, "Who"),
                PolicyBlock(PolicyBlockKind.ITEM, "<strong>Camera</strong>: scans."),
                PolicyBlock(PolicyBlockKind.SUBHEADING, "Maps"),
            ),
            PrivacyPolicy.parse(page),
        )
    }

    @Test
    fun ownSiteLinksBecomeText() {
        val page = """<main><p>See <a href="https://terpoir.app">https://terpoir.app</a>, <a href="/privacy">here</a> and <a href="https://openfreemap.org/privacy/">OpenFreeMap</a>.</p></main>"""
        assertEquals(
            "See https://terpoir.app, here and <a href=\"https://openfreemap.org/privacy/\">OpenFreeMap</a>.",
            PrivacyPolicy.parse(page).single().html,
        )
    }

    @Test
    fun theWebsitePolicyParses() {
        // The same file the build bundles as the asset.
        val blocks = PrivacyPolicy.parse(File("../../site/privacy.html").readText())
        assertEquals(PolicyBlockKind.TITLE, blocks.first().kind)
        assertTrue(blocks.count { it.kind == PolicyBlockKind.HEADING } >= 8)
        assertTrue(blocks.any { it.kind == PolicyBlockKind.ITEM })
        assertFalse(blocks.any { "terpoir.app\"" in it.html || "href=\"/" in it.html })
    }
}
