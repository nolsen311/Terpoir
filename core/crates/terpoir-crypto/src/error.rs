// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Error types. Messages never include key material, plaintext or IDs.

use core::fmt;

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors from cryptographic operations.
///
/// Deliberately coarse: decryption failures do not say why, so callers
/// cannot be used as an oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The operating system's random number generator failed.
    Random,
    /// Authentication failed: wrong key, wrong context, or tampered data.
    Decrypt,
    /// Input is not in a format this version understands.
    Format,
    /// Encryption failed (for example, a stream grew past its limit).
    Encrypt,
    /// Passphrase key derivation failed, usually because of invalid
    /// parameters.
    Kdf,
    /// A recovery key could not be parsed.
    RecoveryKey(RecoveryKeyError),
    /// Reading from or writing to a stream failed.
    Io(std::io::ErrorKind),
}

/// Why a recovery key was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryKeyError {
    /// Wrong number of characters.
    Length,
    /// A character outside the recovery-key alphabet.
    Character,
    /// The checksum does not match: most likely a typo.
    Checksum,
    /// The key was made by a newer, unsupported format version.
    Version,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Random => f.write_str("random number generator failed"),
            Self::Decrypt => f.write_str("decryption failed"),
            Self::Format => f.write_str("unrecognized format"),
            Self::Encrypt => f.write_str("encryption failed"),
            Self::Kdf => f.write_str("key derivation failed"),
            Self::RecoveryKey(e) => write!(f, "invalid recovery key: {e}"),
            Self::Io(kind) => write!(f, "i/o error: {kind}"),
        }
    }
}

impl fmt::Display for RecoveryKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Length => "wrong length",
            Self::Character => "unexpected character",
            Self::Checksum => "checksum mismatch",
            Self::Version => "unsupported version",
        })
    }
}

impl std::error::Error for Error {}

impl From<RecoveryKeyError> for Error {
    fn from(e: RecoveryKeyError) -> Self {
        Self::RecoveryKey(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        // Keep only the kind; OS error messages can include paths.
        Self::Io(e.kind())
    }
}
