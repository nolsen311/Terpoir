// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Error type. Messages carry error kinds only: no SQL, paths, content or
//! IDs.

use core::fmt;

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors from the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The database key is wrong, or the file is not a Terpoir database.
    WrongKey,
    /// The database was created by a newer app version.
    NewerDatabase,
    /// A SQLite error, by its extended result code.
    Database(i32),
    /// A cryptographic error (including authentication failures).
    Crypto(terpoir_crypto::Error),
    /// A record could not be decoded or validated.
    Model(terpoir_model::Error),
    /// A sealed record's ID or kind does not match its contents.
    Mismatch,
    /// A file operation failed.
    Io(std::io::ErrorKind),
    /// The record or file does not exist.
    NotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongKey => f.write_str("wrong database key"),
            Self::NewerDatabase => f.write_str("database is from a newer app version"),
            Self::Database(code) => write!(f, "database error {code}"),
            Self::Crypto(e) => write!(f, "{e}"),
            Self::Model(e) => write!(f, "{e}"),
            Self::Mismatch => f.write_str("sealed record does not match its contents"),
            Self::Io(kind) => write!(f, "i/o error: {kind}"),
            Self::NotFound => f.write_str("not found"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        match e {
            rusqlite::Error::SqliteFailure(err, _) => {
                if err.code == rusqlite::ErrorCode::NotADatabase {
                    Self::WrongKey
                } else {
                    Self::Database(err.extended_code)
                }
            }
            rusqlite::Error::QueryReturnedNoRows => Self::NotFound,
            // Other variants are API misuse or type conversion problems; keep
            // no details (some include SQL text or values).
            _ => Self::Database(-1),
        }
    }
}

impl From<terpoir_crypto::Error> for Error {
    fn from(e: terpoir_crypto::Error) -> Self {
        match e {
            terpoir_crypto::Error::Io(kind) => Self::Io(kind),
            other => Self::Crypto(other),
        }
    }
}

impl From<terpoir_model::Error> for Error {
    fn from(e: terpoir_model::Error) -> Self {
        Self::Model(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.kind())
    }
}
