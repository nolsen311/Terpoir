// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Error type. Messages name fields by their static schema name only, never
//! by content or ID.

use core::fmt;

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors reading, writing or migrating records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Bytes are not a document this version can decode.
    Format,
    /// The document was written by a newer schema than this app knows.
    NewerSchema,
    /// The document's schema is older than any migration supports.
    UnsupportedSchema,
    /// The document holds a different kind of record than requested.
    WrongKind,
    /// A field holds a value of the wrong type or out of range.
    InvalidField(&'static str),
    /// Generating a random ID failed.
    Random,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format => f.write_str("malformed document"),
            Self::NewerSchema => f.write_str("document is from a newer app version"),
            Self::UnsupportedSchema => f.write_str("document schema is no longer supported"),
            Self::WrongKind => f.write_str("wrong record kind"),
            Self::InvalidField(name) => write!(f, "invalid value in field `{name}`"),
            Self::Random => f.write_str("random number generator failed"),
        }
    }
}

impl std::error::Error for Error {}

impl From<terpoir_crypto::Error> for Error {
    fn from(e: terpoir_crypto::Error) -> Self {
        match e {
            terpoir_crypto::Error::Random => Self::Random,
            _ => Self::Format,
        }
    }
}
