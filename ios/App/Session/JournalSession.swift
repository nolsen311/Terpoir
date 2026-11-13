// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation
import LocalAuthentication
import TerpoirCore

/// Whether the journal is open, and the open journal if so (the iOS side of
/// Android's `JournalSession`). The keys are made, checked and used by the
/// core; this only keeps the wrapped bytes in files and the master key in
/// the vault. The master key is never kept here: it goes straight into the
/// core and the copy is wiped.
@MainActor
final class JournalSession: ObservableObject {
    enum State {
        /// No journal on this device yet.
        case noJournal
        case locked
        case unlocked(Journal)
    }

    /// A typed recovery key, checked against this journal.
    enum RecoveryCheck: Equatable {
        case matches, incomplete, invalidCharacter, typo, wrongKey
    }

    @Published private(set) var state: State

    var hasJournal: Bool {
        if case .noJournal = state { return false }
        return true
    }

    var isUnlocked: Bool {
        if case .unlocked = state { return true }
        return false
    }

    private let files: JournalFiles
    private let vault: MasterKeyVault

    init(files: JournalFiles = .app, vault: MasterKeyVault? = nil) {
        self.files = files
        self.vault = vault ?? MasterKeyVault(wrappedFile: files.wrappedMasterKey)
        state = self.vault.exists() && FileManager.default.fileExists(atPath: files.wrappedDatabaseKey.path)
            ? .locked : .noJournal
    }

    /// Creates a new journal, keeps its master key in the vault with
    /// `protection` and opens it. Returns the recovery key, to show once
    /// for the user to keep; it is never stored.
    func create(protection: MasterKeyVault.Protection) throws -> String {
        var keys = try createKeys()
        defer { keys.masterKey.resetBytes(in: 0..<keys.masterKey.count) }
        try files.prepare()
        try files.write(keys.masterWrappedByRecovery, to: files.masterWrappedByRecovery)
        try files.write(keys.wrappedDatabaseKey, to: files.wrappedDatabaseKey)
        try vault.store(masterKey: keys.masterKey, protection: protection)
        try open(masterKey: keys.masterKey)
        return keys.recoveryKey
    }

    /// Unlocks with the vault, which asks the OS to authenticate the user
    /// through `context` (Face ID, Touch ID or the passcode). The prompt
    /// blocks its thread, so the vault is asked off the main thread.
    func unlock(context: LAContext) async throws {
        let vault = self.vault
        var masterKey = try await Task.detached { try vault.masterKey(context: context) }.value
        defer { masterKey.resetBytes(in: 0..<masterKey.count) }
        try open(masterKey: masterKey)
    }

    func lock() {
        if case let .unlocked(journal) = state {
            journal.lock()
            state = .locked
        }
    }

    /// Checks a typed recovery key: the core unwraps the master key with
    /// it, and the result is wiped at once.
    func checkRecoveryKey(_ typed: String) -> RecoveryCheck {
        guard let wrapped = try? Data(contentsOf: files.masterWrappedByRecovery) else { return .wrongKey }
        do {
            var masterKey = try masterKeyFromRecovery(recoveryKey: typed, masterWrappedByRecovery: wrapped)
            masterKey.resetBytes(in: 0..<masterKey.count)
            return .matches
        } catch CoreError.RecoveryKeyLength {
            return .incomplete
        } catch CoreError.RecoveryKeyCharacter {
            return .invalidCharacter
        } catch CoreError.RecoveryKeyChecksum {
            return .typo
        } catch {
            return .wrongKey
        }
    }

    /// Deletes the journal and its vault key. Only for a journal whose
    /// setup did not finish, which holds no entries yet.
    func reset() {
        lock()
        try? FileManager.default.removeItem(at: files.root)
        vault.delete()
        state = .noJournal
    }

    private func open(masterKey: Data) throws {
        let journal = try Journal.open(
            dbPath: files.database.path,
            filesDir: files.attachments.path,
            masterKey: masterKey,
            wrappedDatabaseKey: Data(contentsOf: files.wrappedDatabaseKey)
        )
        let now = UInt64(Date().timeIntervalSince1970 * 1000)
        // As on Android: records a newer app synced join once this version
        // can read them. A failure must not keep the journal locked.
        _ = try? journal.applyPendingRecords(nowMs: now)
        state = .unlocked(journal)
    }
}
