// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

package app.terpoir.privacy

/** What a block of the privacy policy is, for its text style. */
enum class PolicyBlockKind { TITLE, HEADING, SUBHEADING, PARAGRAPH, ITEM }

/** One heading, paragraph or list item, with its inline HTML (bold, links). */
data class PolicyBlock(val kind: PolicyBlockKind, val html: String)

/**
 * The website's privacy policy (`site/privacy.html`, bundled as an asset at
 * build time) split into blocks the app shows as text. The page is our own
 * and simple: headings, paragraphs and list items inside `<main>`.
 */
object PrivacyPolicy {
    /** The asset the build copies from `site/privacy.html`. */
    const val ASSET = "privacy.html"

    private val main = Regex("<main[^>]*>(.*?)</main>", RegexOption.DOT_MATCHES_ALL)
    private val block = Regex("<(h1|h2|h3|p|li)(?:\\s[^>]*)?>(.*?)</\\1>", RegexOption.DOT_MATCHES_ALL)

    // Links to the project's own site stay plain text: the app never sends
    // anyone to terpoir.app, not even through the browser.
    private val ownLink = Regex(
        "<a\\s+href=\"(?:https?://(?:www\\.)?terpoir\\.app[^\"]*|/[^\"]*)\"[^>]*>(.*?)</a>",
        RegexOption.DOT_MATCHES_ALL,
    )
    private val space = Regex("\\s+")

    fun parse(page: String): List<PolicyBlock> {
        val body = main.find(page)?.groupValues?.get(1) ?: return emptyList()
        return block.findAll(body).map { match ->
            val kind = when (match.groupValues[1]) {
                "h1" -> PolicyBlockKind.TITLE
                "h2" -> PolicyBlockKind.HEADING
                "h3" -> PolicyBlockKind.SUBHEADING
                "li" -> PolicyBlockKind.ITEM
                else -> PolicyBlockKind.PARAGRAPH
            }
            PolicyBlock(kind, ownLink.replace(match.groupValues[2].replace(space, " ").trim()) { it.groupValues[1] })
        }.toList()
    }
}
