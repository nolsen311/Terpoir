// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The error type exposed to the platform. Variants name a kind of
//! failure only; messages never include content, keys, IDs or paths.

use core::fmt;

/// Why a core call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Error)]
#[uniffi(flat_error)]
#[non_exhaustive]
pub enum CoreError {
    /// The journal is locked; unlock it again.
    Locked,
    /// The key, passphrase or recovery key is wrong, or data was tampered
    /// with.
    WrongKey,
    /// The recovery key has the wrong length.
    RecoveryKeyLength,
    /// The recovery key contains a character that cannot appear in one.
    RecoveryKeyCharacter,
    /// The recovery key has a typo (checksum mismatch).
    RecoveryKeyChecksum,
    /// An ID, slug, or value is invalid.
    InvalidInput,
    /// The record or attachment does not exist.
    NotFound,
    /// The record is a different type than requested.
    WrongType,
    /// Data was written by a newer app version; update the app.
    NewerVersion,
    /// A sync blob is malformed or belongs to another journal.
    InvalidSyncData,
    /// Local storage failed (disk full, I/O error).
    Storage,
    /// Something unexpected; the operation did not complete.
    Internal,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Locked => "journal is locked",
            Self::WrongKey => "wrong key",
            Self::RecoveryKeyLength => "recovery key has the wrong length",
            Self::RecoveryKeyCharacter => "recovery key contains an invalid character",
            Self::RecoveryKeyChecksum => "recovery key has a typo",
            Self::InvalidInput => "invalid input",
            Self::NotFound => "not found",
            Self::WrongType => "wrong record type",
            Self::NewerVersion => "written by a newer app version",
            Self::InvalidSyncData => "invalid sync data",
            Self::Storage => "storage error",
            Self::Internal => "internal error",
        })
    }
}

impl std::error::Error for CoreError {}

impl From<std::io::Error> for CoreError {
    fn from(_: std::io::Error) -> Self {
        Self::Storage
    }
}

impl From<terpoir_crypto::Error> for CoreError {
    fn from(e: terpoir_crypto::Error) -> Self {
        use terpoir_crypto::{Error, RecoveryKeyError};
        match e {
            Error::Decrypt | Error::Kdf => Self::WrongKey,
            Error::Format => Self::InvalidInput,
            Error::RecoveryKey(RecoveryKeyError::Length) => Self::RecoveryKeyLength,
            Error::RecoveryKey(RecoveryKeyError::Character) => Self::RecoveryKeyCharacter,
            Error::RecoveryKey(RecoveryKeyError::Checksum) => Self::RecoveryKeyChecksum,
            Error::RecoveryKey(RecoveryKeyError::Version) => Self::NewerVersion,
            Error::Io(_) => Self::Storage,
            _ => Self::Internal,
        }
    }
}

impl From<terpoir_model::Error> for CoreError {
    fn from(e: terpoir_model::Error) -> Self {
        use terpoir_model::Error;
        match e {
            Error::NewerSchema => Self::NewerVersion,
            Error::WrongKind => Self::WrongType,
            Error::InvalidField(_) | Error::Format => Self::InvalidInput,
            _ => Self::Internal,
        }
    }
}

impl From<terpoir_store::Error> for CoreError {
    fn from(e: terpoir_store::Error) -> Self {
        use terpoir_store::Error;
        match e {
            Error::WrongKey => Self::WrongKey,
            Error::NewerDatabase => Self::NewerVersion,
            Error::Crypto(c) => c.into(),
            Error::Model(m) => m.into(),
            Error::NotFound => Self::NotFound,
            Error::Mismatch => Self::WrongType,
            Error::Database(_) | Error::Io(_) => Self::Storage,
            _ => Self::Internal,
        }
    }
}

impl From<terpoir_sync::Error> for CoreError {
    fn from(e: terpoir_sync::Error) -> Self {
        use terpoir_sync::Error;
        match e {
            Error::Blob => Self::InvalidSyncData,
            Error::NewerFormat => Self::NewerVersion,
            Error::Mismatch => Self::InvalidSyncData,
            Error::Crypto(c) => c.into(),
            Error::Store(s) => s.into(),
            Error::Model(m) => m.into(),
            _ => Self::Internal,
        }
    }
}

/// Result alias for exported functions.
pub type Result<T> = core::result::Result<T, CoreError>;
