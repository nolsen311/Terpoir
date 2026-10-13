// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Sync for Terpoir: how devices agree without any server.
//!
//! - [`Clock`]: a hybrid logical clock that stamps every local edit.
//! - [`merge_document`]: per-field last-writer-wins; the later stamp wins,
//!   ties broken by device ID (see [`terpoir_model::Stamp`]).
//! - [`apply_changes`]: new records merge as a set union; deletion markers
//!   carry only the record ID and always win.
//! - [`export_blobs`] / [`import_blob`]: the same, against the encrypted
//!   store. Records a newer app version wrote are kept sealed and carried
//!   along, and [`apply_pending`] applies them once this version can read
//!   them.
//! - [`Blob`]: the file format written to the user's own storage: sealed
//!   records and deletion markers, padded to a size bucket, encrypted under
//!   a key derived from the master key, and stored under a random name.
//!
//! The merge is a state-based CRDT: merging is commutative, associative
//! and idempotent, so every device converges to the same journal however
//! and in whatever order blobs arrive. Property tests in `tests/` check
//! exactly that.

#![forbid(unsafe_code)]

mod blob;
mod error;
mod exchange;
mod hlc;
mod merge;

pub use blob::{
    Blob, MAX_BLOB_LEN, container_key, decode_blob, encode_blob, opaque_name, padded_len,
};
pub use error::{Error, Result};
pub use exchange::{
    BlobInspection, ImportSummary, StoreReplica, apply_pending, export_blobs, import_blob,
    inspect_blob,
};
pub use hlc::{Clock, MAX_DRIFT_MS};
pub use merge::{MemoryReplica, Replica, apply_changes, merge_document, prunable_markers};
