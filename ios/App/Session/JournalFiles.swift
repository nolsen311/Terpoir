// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation

/// Where the journal lives (as on Android): app-private storage with
/// Complete data protection, excluded from backups.
struct JournalFiles {
    /// Everything below belongs to the journal.
    let root: URL

    /// The SQLCipher database.
    var database: URL { root.appendingPathComponent("journal.db") }
    /// Encrypted attachments.
    var attachments: URL { root.appendingPathComponent("files") }
    /// The master key wrapped by the vault's key.
    var wrappedMasterKey: URL { root.appendingPathComponent("keys/master.vault") }
    /// The master key wrapped by the recovery key.
    var masterWrappedByRecovery: URL { root.appendingPathComponent("keys/master.recovery") }
    /// The database key wrapped by the master key.
    var wrappedDatabaseKey: URL { root.appendingPathComponent("keys/database") }
    /// Notes written while the journal is locked (later; merged on unlock).
    var inbox: URL { root.appendingPathComponent("inbox") }

    /// The app's own journal, in Application Support.
    static var app: JournalFiles {
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return JournalFiles(root: support.appendingPathComponent("journal"))
    }

    /// Makes the directories, protected and excluded from backups. Files
    /// created in them later (the database, attachments) take the
    /// directory's protection.
    func prepare() throws {
        let manager = FileManager.default
        for directory in [root, root.appendingPathComponent("keys"), attachments] {
            try manager.createDirectory(
                at: directory,
                withIntermediateDirectories: true,
                attributes: [.protectionKey: FileProtectionType.complete]
            )
        }
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var excluded = root
        try excluded.setResourceValues(values)
    }

    /// Writes key material: protected, atomic, never backed up.
    func write(_ data: Data, to file: URL) throws {
        try data.write(to: file, options: [.atomic, .completeFileProtection])
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var url = file
        try url.setResourceValues(values)
    }
}
