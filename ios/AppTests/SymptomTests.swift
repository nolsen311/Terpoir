// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import TerpoirCore
import XCTest
@testable import Terpoir

@MainActor
final class SymptomTests: XCTestCase {
    private var files: JournalFiles!
    private var vault: MasterKeyVault!
    private var repo: JournalRepository!

    override func setUp() async throws {
        files = JournalFiles(root: FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-symptom-\(UUID().uuidString)"))
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

    func testTheResultsAreTheCores() {
        XCTAssertEqual(Set(symptomResults.map(\.slug)), Set(vocabulary().symptomResults))
        XCTAssertEqual(vocabulary().symptoms.filter { vocabularyLabels[$0] == nil }, [])
    }

    func testSymptomsAreSavedAndDeletedEverywhere() throws {
        let custom = try repo.saveTag(id: nil, TagData(name: "Restless legs", category: symptomCategory))
        let pain = TagRefData.builtin(slug: vocabulary().symptoms[0])
        var saved = SessionData(
            when: LocalTimeData(unixMs: 1_000, utcOffsetMin: 0), lines: [], activity: nil, intention: nil,
            moodBefore: nil, social: nil, effects: [], onsetMin: nil, peakMin: nil, durationMin: nil,
            moodAfter: nil, rating: nil, wouldRepeat: nil, spot: nil, symptoms: [], attachments: []
        )
        saved.symptoms = [SymptomData(tag: pain, result: "helped"), SymptomData(tag: .custom(id: custom), result: "not_rated")]
        let id = try repo.saveSession(id: nil, saved)
        XCTAssertEqual(repo.session(id)?.symptoms.count, 2)

        XCTAssertEqual(try repo.deleteSymptoms(), 1)
        XCTAssertEqual(repo.session(id)?.symptoms.count, 0)
        XCTAssertTrue(repo.tags(category: symptomCategory).isEmpty)
    }
}
