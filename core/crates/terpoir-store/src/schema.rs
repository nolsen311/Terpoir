// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Database schema and its migrations (tracked in `PRAGMA user_version`).

use rusqlite::Connection;

use crate::{Error, Result};

/// `STEPS[i]` upgrades the schema from version `i` to `i + 1`.
const STEPS: &[&str] = &[
    // 0 -> 1: initial schema.
    "CREATE TABLE records (
         id BLOB PRIMARY KEY NOT NULL,
         kind INTEGER NOT NULL,
         wrapped_key BLOB NOT NULL,
         ciphertext BLOB NOT NULL
     ) WITHOUT ROWID;
     CREATE INDEX records_kind ON records (kind);
     CREATE TABLE refs (
         from_id BLOB NOT NULL,
         to_id BLOB NOT NULL,
         PRIMARY KEY (from_id, to_id)
     ) WITHOUT ROWID;
     CREATE INDEX refs_to ON refs (to_id);
     CREATE TABLE days (
         id BLOB PRIMARY KEY NOT NULL,
         day INTEGER NOT NULL
     ) WITHOUT ROWID;
     CREATE INDEX days_day ON days (day);
     CREATE TABLE tags (
         id BLOB NOT NULL,
         tag TEXT NOT NULL,
         PRIMARY KEY (id, tag)
     ) WITHOUT ROWID;
     CREATE INDEX tags_tag ON tags (tag);
     CREATE TABLE files (
         id BLOB PRIMARY KEY NOT NULL,
         owner BLOB NOT NULL
     ) WITHOUT ROWID;
     CREATE INDEX files_owner ON files (owner);",
    // 1 -> 2: sync state and deletion markers (record IDs only).
    "CREATE TABLE meta (
         key TEXT PRIMARY KEY NOT NULL,
         value BLOB NOT NULL
     ) WITHOUT ROWID;
     CREATE TABLE deletions (
         id BLOB PRIMARY KEY NOT NULL
     ) WITHOUT ROWID;",
    // 2 -> 3: records this app version can't read yet (an unknown kind or
    // a newer schema), kept sealed exactly as received until it can.
    "CREATE TABLE pending (
         id BLOB PRIMARY KEY NOT NULL,
         kind INTEGER NOT NULL,
         wrapped_key BLOB NOT NULL,
         ciphertext BLOB NOT NULL
     ) WITHOUT ROWID;",
    // 3 -> 4: the version of the rules each kind of derived data (such as
    // the index) was built with, so an upgrade that changes them rebuilds it.
    "CREATE TABLE derived (
         name TEXT PRIMARY KEY NOT NULL,
         version INTEGER NOT NULL
     ) WITHOUT ROWID;",
];

/// Brings the schema up to date. Returns the version it started at (0 for
/// a new database).
pub(crate) fn migrate(conn: &Connection) -> Result<usize> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    let version = usize::try_from(version).map_err(|_| Error::NewerDatabase)?;
    if version > STEPS.len() {
        return Err(Error::NewerDatabase);
    }
    for (from, step) in STEPS.iter().enumerate().skip(version) {
        let to = from + 1;
        conn.execute_batch(&format!(
            "BEGIN; {step}; PRAGMA user_version = {to}; COMMIT;"
        ))?;
    }
    Ok(version)
}
