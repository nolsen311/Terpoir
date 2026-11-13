// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

import Foundation
import LocalAuthentication
import Security

/// Keeps the journal's master key wrapped by a key the user must unlock
/// (docs/design-decisions.md, "Biometric unlock"; the iOS side of Android's
/// `MasterKeyVault`).
///
/// The wrapping key is a P-256 key in the Secure Enclave where the device
/// has one (every supported iPhone), otherwise a Keychain key (the
/// simulator). Either way it is this-device-only, needs a passcode on the
/// device, and its private half can be used only after the OS has
/// authenticated the user, so a patched app cannot unwrap the master key
/// without them. The master key is wrapped with Apple's ECIES
/// (`SecKeyCreateEncryptedData`, cofactor X9.63 SHA-256 with AES-GCM) and
/// the result kept in a file with Complete data protection, excluded from
/// backups.
///
/// This type only wraps and unwraps bytes; the master key itself is created
/// and used by the Rust core.
final class MasterKeyVault {
    /// Who may use the wrapping key.
    enum Protection {
        /// Face ID, Touch ID or the device passcode (the default).
        case userPresence
        /// Only the biometrics enrolled now: enrolling another invalidates
        /// the key, and the journal is opened with the recovery key instead.
        case currentBiometrics
        #if DEBUG
        /// No prompt. Tests only: never built into a release.
        case unprotectedForTests
        #endif
    }

    enum VaultError: Error, Equatable {
        /// No wrapping key or no wrapped master key.
        case missing
        /// The device has no passcode, which the key requires.
        case noPasscode
        case keychain(OSStatus)
        case crypto
    }

    private static let algorithm = SecKeyAlgorithm.eciesEncryptionCofactorVariableIVX963SHA256AESGCM

    private let tag: Data
    private let wrappedFile: URL
    private let secureEnclave: Bool

    /// - Parameters:
    ///   - label: the wrapping key's Keychain tag; tests pass their own.
    ///   - wrappedFile: where the wrapped master key is kept.
    ///   - secureEnclave: whether to make the key in the Secure Enclave.
    init(label: String = "app.terpoir.master-key-wrapping", wrappedFile: URL, secureEnclave: Bool = MasterKeyVault.hasSecureEnclave) {
        self.tag = Data(label.utf8)
        self.wrappedFile = wrappedFile
        self.secureEnclave = secureEnclave
    }

    /// Whether this device has a Secure Enclave (never the simulator).
    static var hasSecureEnclave: Bool {
        #if targetEnvironment(simulator)
        return false
        #else
        return true
        #endif
    }

    /// Whether a wrapping key and a wrapped master key both exist. Never
    /// prompts.
    func exists() -> Bool {
        FileManager.default.fileExists(atPath: wrappedFile.path) && privateKey(context: nil) != nil
    }

    /// Makes a new wrapping key, replacing any existing one and its wrapped
    /// master key, and wraps `masterKey` with it.
    func store(masterKey: Data, protection: Protection) throws {
        delete()
        let key = try makeKey(protection: protection)
        guard let publicKey = SecKeyCopyPublicKey(key),
              SecKeyIsAlgorithmSupported(publicKey, .encrypt, Self.algorithm)
        else { throw VaultError.crypto }
        var error: Unmanaged<CFError>?
        guard let wrapped = SecKeyCreateEncryptedData(publicKey, Self.algorithm, masterKey as CFData, &error) as Data? else {
            throw VaultError.crypto
        }
        try write(wrapped)
    }

    /// The master key, after the OS has authenticated the user through
    /// `context` (which carries the prompt's reason and any reuse window).
    func masterKey(context: LAContext) throws -> Data {
        guard let wrapped = try? Data(contentsOf: wrappedFile), let key = privateKey(context: context) else {
            throw VaultError.missing
        }
        var error: Unmanaged<CFError>?
        guard let plain = SecKeyCreateDecryptedData(key, Self.algorithm, wrapped as CFData, &error) as Data? else {
            throw VaultError.crypto
        }
        return plain
    }

    /// Deletes the wrapping key and the wrapped master key.
    func delete() {
        SecItemDelete([
            kSecClass: kSecClassKey,
            kSecAttrApplicationTag: tag,
        ] as CFDictionary)
        try? FileManager.default.removeItem(at: wrappedFile)
    }

    private func makeKey(protection: Protection) throws -> SecKey {
        var flags: SecAccessControlCreateFlags
        // Only with a passcode set on the device, and never off it.
        var accessible = kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly
        switch protection {
        case .userPresence: flags = .userPresence
        case .currentBiometrics: flags = .biometryCurrentSet
        #if DEBUG
        case .unprotectedForTests:
            // The simulator has no passcode.
            flags = []
            accessible = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        #endif
        }
        if secureEnclave { flags.insert(.privateKeyUsage) }
        var error: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(nil, accessible, flags, &error) else {
            throw VaultError.crypto
        }
        var attributes: [CFString: Any] = [
            kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrKeySizeInBits: 256,
            kSecPrivateKeyAttrs: [
                kSecAttrIsPermanent: true,
                kSecAttrApplicationTag: tag,
                kSecAttrAccessControl: access,
            ] as [CFString: Any],
        ]
        if secureEnclave { attributes[kSecAttrTokenID] = kSecAttrTokenIDSecureEnclave }
        guard let key = SecKeyCreateRandomKey(attributes as CFDictionary, &error) else {
            let status = (error?.takeRetainedValue()).map { OSStatus(CFErrorGetCode($0)) } ?? errSecParam
            // errSecAuthFailed / errSecNotAvailable: no passcode set.
            if status == errSecAuthFailed || status == errSecNotAvailable { throw VaultError.noPasscode }
            throw VaultError.keychain(status)
        }
        return key
    }

    /// The private key's reference. Looking it up never prompts; using it
    /// with `context` does.
    private func privateKey(context: LAContext?) -> SecKey? {
        var query: [CFString: Any] = [
            kSecClass: kSecClassKey,
            kSecAttrApplicationTag: tag,
            kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
            kSecReturnRef: true,
        ]
        if let context { query[kSecUseAuthenticationContext] = context }
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess, let item else { return nil }
        // kSecClassKey with kSecReturnRef always gives a SecKey.
        return (item as! SecKey)
    }

    private func write(_ wrapped: Data) throws {
        var directory = wrappedFile.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        try wrapped.write(to: wrappedFile, options: [.atomic, .completeFileProtection])
        // App data never goes to iCloud or computer backups (CLAUDE.md).
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try directory.setResourceValues(values)
        var file = wrappedFile
        try file.setResourceValues(values)
    }
}
