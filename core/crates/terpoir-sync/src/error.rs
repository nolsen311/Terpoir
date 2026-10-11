// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Error type. No content, keys or IDs in messages.

use core::fmt;

/// Result alias for this crate.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors from sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Two documents with different IDs or kinds were asked to merge.
    Mismatch,
    /// A blob is malformed, too large, or does not decrypt.
    Blob,
    /// A cryptographic error.
    Crypto(terpoir_crypto::Error),
    /// A store error while applying changes.
    Store(terpoir_store::Error),
    /// A record could not be decoded or migrated.
    Model(terpoir_model::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mismatch => f.write_str("documents do not match"),
            Self::Blob => f.write_str("invalid sync blob"),
            Self::Crypto(e) => write!(f, "{e}"),
            Self::Store(e) => write!(f, "{e}"),
            Self::Model(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<terpoir_crypto::Error> for Error {
    fn from(e: terpoir_crypto::Error) -> Self {
        Self::Crypto(e)
    }
}

impl From<terpoir_store::Error> for Error {
    fn from(e: terpoir_store::Error) -> Self {
        Self::Store(e)
    }
}
