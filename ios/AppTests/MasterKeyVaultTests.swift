// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

// The vault on the simulator: a Keychain wrapping key (no Secure Enclave)
// with no prompt, so the tests run unattended. The prompt and the Secure
// Enclave are the OS's; these tests cover what the vault itself does.

import LocalAuthentication
import XCTest
@testable import Terpoir

final class MasterKeyVaultTests: XCTestCase {
    private var dir: URL!
    private var vault: MasterKeyVault!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("terpoir-vault-\(UUID().uuidString)")
        vault = MasterKeyVault(
            label: "app.terpoir.tests.\(UUID().uuidString)",
            wrappedFile: dir.appendingPathComponent("vault/master.wrapped"),
            secureEnclave: false
        )
    }

    override func tearDownWithError() throws {
        vault.delete()
        try? FileManager.default.removeItem(at: dir)
    }

    private func masterKey() -> Data { Data((0..<32).map { _ in UInt8.random(in: .min ... .max) }) }

    func testAStoredMasterKeyComesBack() throws {
        let key = masterKey()
        XCTAssertFalse(vault.exists())
        try vault.store(masterKey: key, protection: .unprotectedForTests)
        XCTAssertTrue(vault.exists())
        XCTAssertEqual(try vault.masterKey(context: LAContext()), key)
    }

    func testTheFileHoldsNoPlainKey() throws {
        let key = masterKey()
        try vault.store(masterKey: key, protection: .unprotectedForTests)
        let wrapped = try Data(contentsOf: dir.appendingPathComponent("vault/master.wrapped"))
        XCTAssertNil(wrapped.range(of: key))
        XCTAssertGreaterThan(wrapped.count, key.count)
    }

    func testTheWrappedKeyIsExcludedFromBackups() throws {
        try vault.store(masterKey: masterKey(), protection: .unprotectedForTests)
        let values = try dir.appendingPathComponent("vault/master.wrapped").resourceValues(forKeys: [.isExcludedFromBackupKey])
        XCTAssertEqual(values.isExcludedFromBackup, true)
    }

    func testStoringAgainReplacesTheOldKey() throws {
        try vault.store(masterKey: masterKey(), protection: .unprotectedForTests)
        let old = try Data(contentsOf: dir.appendingPathComponent("vault/master.wrapped"))
        let key = masterKey()
        try vault.store(masterKey: key, protection: .unprotectedForTests)
        XCTAssertEqual(try vault.masterKey(context: LAContext()), key)
        // The old wrapped key no longer opens with the new wrapping key.
        try old.write(to: dir.appendingPathComponent("vault/master.wrapped"))
        XCTAssertThrowsError(try vault.masterKey(context: LAContext()))
    }

    func testDeleteRemovesBoth() throws {
        try vault.store(masterKey: masterKey(), protection: .unprotectedForTests)
        vault.delete()
        XCTAssertFalse(vault.exists())
        XCTAssertThrowsError(try vault.masterKey(context: LAContext())) { error in
            XCTAssertEqual(error as? MasterKeyVault.VaultError, .missing)
        }
    }
}
