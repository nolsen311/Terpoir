// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class NoteTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-note-\(UUID().uuidString)"))
        vault = MasterKeyVault(label: "app.terpoir.tests.\(UUID().uuidString)", wrappedFile: files.wrappedMasterKey, secureEnclave: false)
        let session = JournalSession(files: files, vault: vault)
        _ = try session.create(protection: .unprotectedForTests)
        guard case let .unlocked(journal) = session.state else { return XCTFail("not unlocked") }
        repo = JournalRepository(journal: journal)
    }

    override func tearDown() async throws {
        vault.delete()
        try? FileManager.default.removeItem(at: files.root)
    }

    private func note(_ text: String, at ms: Int64, session: String? = nil) -> SnippetData {
        SnippetData(session: session, at: LocalTimeData(unixMs: ms, utcOffsetMin: 0), text: text, tags: [])
    }

    func testStandaloneNotesAreListedNewestFirst() throws {
        let older = try repo.saveNote(id: nil, note("first", at: 1_000))
        let newer = try repo.saveNote(id: nil, note("second", at: 2_000))
        let sessionId = try repo.saveSession(id: nil, SessionData(
            when: LocalTimeData(unixMs: 500, utcOffsetMin: 0), lines: [], activity: nil, intention: nil,
            moodBefore: nil, social: nil, effects: [], onsetMin: nil, peakMin: nil, durationMin: nil,
            moodAfter: nil, rating: nil, wouldRepeat: nil, spot: nil, symptoms: [], attachments: []
        ))
        try repo.saveNote(id: nil, note("on a session", at: 3_000, session: sessionId))
        XCTAssertEqual(repo.standaloneNotes().map(\.id), [newer, older])
    }

    func testTagsAreSavedWithWhereTheyCameFrom() throws {
        var saved = note("Felt relaxed", at: 1_000)
        saved.tags = [TagProvenance(tag: .builtin(slug: "relaxed"), provenance: "keyword")]
        let id = try repo.saveNote(id: nil, saved)
        XCTAssertEqual(repo.snippet(id)?.tags.map(\.provenance), ["keyword"])
    }

    func testSuggestionsQuoteTheWordsTheyCameFrom() {
        let text = "🌿 Felt really relaxed after"
        let suggestions = repo.suggestTags(text)
        let relaxed = suggestions.first { $0.tag == .builtin(slug: "relaxed") }
        XCTAssertNotNil(relaxed, "\(suggestions.map(\.tag))")
        if let relaxed {
            XCTAssertTrue(phrase(of: relaxed, in: text).lowercased().contains("relax"), phrase(of: relaxed, in: text))
        }
    }
}
