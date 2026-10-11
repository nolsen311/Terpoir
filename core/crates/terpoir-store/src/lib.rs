// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Terpoir's encrypted local store.
//!
//! Two layers (see `docs/design-decisions.md`, "Local storage format"):
//!
//! 1. **Per-record encryption.** Each record is serialized
//!    ([`Document::encode`]) and sealed with its own [`RecordKey`]; the
//!    record key is stored wrapped by the master key. The sealed form is
//!    what sync transfers ([`SealedRecord`]).
//! 2. **SQLCipher outside.** The whole database (including the WAL) is
//!    encrypted with a [`DatabaseKey`], hiding structure, row counts and the
//!    index.
//!
//! The on-disk index holds only record IDs, kinds, references between
//! records, local dates and confirmed tag keys. Free text is never indexed.
//!
//! **Deletion is crypto-erasure.** [`Store::delete`] removes the record's
//! wrapped key along with its ciphertext, index rows and attached files.
//! `secure_delete` overwrites freed pages and the WAL is checkpointed and
//! truncated, so leftovers on flash are ciphertext under a key that no
//! longer exists.
//!
//! Attached files (photos, voice clips) are encrypted with the STREAM
//! construction under a key derived from the owning record's key, and
//! named by random file ID.
//!
//! The store never holds the master key: callers pass it per operation, so
//! locking the app is dropping the master key and closing the store.

#![forbid(unsafe_code)]

mod error;
mod index;
mod schema;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use terpoir_crypto::{
    DatabaseKey, MasterKey, RecordKey, WrappedKey, decrypt_stream, encrypt_stream,
};
use terpoir_model::{Document, Record, RecordId, RecordKind, migrate};
use zeroize::Zeroizing;

pub use error::{Error, Result};
use index::IndexEntries;

/// A record in its encrypted form, as stored and synced.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SealedRecord {
    /// Record ID.
    pub id: RecordId,
    /// Record kind.
    pub kind: RecordKind,
    /// The record key, wrapped by the master key and bound to `id`.
    pub wrapped_key: WrappedKey,
    /// The document sealed with the record key.
    pub ciphertext: Vec<u8>,
}

impl SealedRecord {
    /// Decrypts and decodes the record, checking that its contents match
    /// its ID and kind. The document is not migrated.
    ///
    /// # Errors
    ///
    /// [`Error::Crypto`] if it does not decrypt under `master`;
    /// [`Error::Mismatch`] if the contents disagree with the ID or kind.
    pub fn open(&self, master: &MasterKey) -> Result<Document> {
        open_sealed(master, self)
    }
}

/// Associated data binding a record's ciphertext to its ID and kind, so
/// rows cannot be swapped.
fn record_aad(id: &RecordId, kind: RecordKind) -> [u8; 17] {
    let mut aad = [0u8; 17];
    aad[..16].copy_from_slice(id.as_bytes());
    aad[16] = kind.code();
    aad
}

/// The encrypted local store.
pub struct Store {
    conn: Connection,
    files_dir: PathBuf,
}

impl core::fmt::Debug for Store {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Store(..)")
    }
}

impl Store {
    /// Opens (or creates) the database at `db_path`, with attached files
    /// under `files_dir`.
    ///
    /// Both paths must be in app-private storage excluded from OS backups;
    /// that is the platform layer's job.
    ///
    /// # Errors
    ///
    /// [`Error::WrongKey`] if `key` does not open an existing database;
    /// [`Error::NewerDatabase`] if a newer app created it; otherwise
    /// [`Error::Database`] or [`Error::Io`].
    pub fn open(db_path: &Path, files_dir: &Path, key: &DatabaseKey) -> Result<Self> {
        fs::create_dir_all(files_dir)?;
        let conn = Connection::open(db_path)?;
        // Wipe SQLCipher's freed memory (it held the key and plaintext pages).
        conn.execute_batch("PRAGMA cipher_memory_security = ON;")?;
        conn.execute_batch(&key_pragma(key))?;
        // The first read fails with NotADatabase if the key is wrong.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })?;
        conn.pragma_update(None, "secure_delete", "ON")?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        schema::migrate(&conn)?;
        Ok(Self {
            conn,
            files_dir: files_dir.to_path_buf(),
        })
    }

    /// Encrypts and stores a document, creating its record key on first
    /// write and reusing it after that (attached files depend on it).
    ///
    /// # Errors
    ///
    /// [`Error::Mismatch`] if a record with this ID but another kind
    /// exists; crypto, model or database errors otherwise.
    pub fn put(&mut self, master: &MasterKey, doc: &Document) -> Result<()> {
        let id = doc.id();
        let existing = self.wrapped_key_of(&id)?;
        let (record_key, wrapped) = match existing {
            Some((kind, wrapped)) => {
                if kind != doc.kind() {
                    return Err(Error::Mismatch);
                }
                (master.unwrap_record_key(&wrapped, &id)?, wrapped)
            }
            None => {
                let key = RecordKey::generate()?;
                let wrapped = master.wrap_record_key(&key, &id)?;
                (key, wrapped)
            }
        };
        let plaintext = doc.encode()?;
        let ciphertext = record_key.seal(&record_aad(&id, doc.kind()), &plaintext)?;
        let sealed = SealedRecord {
            id,
            kind: doc.kind(),
            wrapped_key: wrapped,
            ciphertext,
        };
        let entries = IndexEntries::from_document(doc);
        let tx = self.conn.transaction()?;
        write_sealed(&tx, &sealed, &entries)?;
        tx.commit()?;
        Ok(())
    }

    /// Stores a sealed record received from sync as is, after checking that
    /// it decrypts under `master` and matches its claimed ID and kind.
    ///
    /// # Errors
    ///
    /// [`Error::Crypto`] if it does not decrypt; [`Error::Mismatch`] if its
    /// contents disagree with its ID or kind.
    pub fn put_sealed(&mut self, master: &MasterKey, sealed: &SealedRecord) -> Result<()> {
        let doc = open_sealed(master, sealed)?;
        let entries = IndexEntries::from_document(&doc);
        let tx = self.conn.transaction()?;
        write_sealed(&tx, sealed, &entries)?;
        tx.commit()?;
        Ok(())
    }

    /// The sealed form of a record, for sync.
    ///
    /// # Errors
    ///
    /// Database or format errors.
    pub fn sealed(&self, id: &RecordId) -> Result<Option<SealedRecord>> {
        self.conn
            .query_row(
                "SELECT kind, wrapped_key, ciphertext FROM records WHERE id = ?1",
                params![id.as_bytes()],
                |r| {
                    Ok((
                        r.get::<_, u8>(0)?,
                        r.get::<_, Vec<u8>>(1)?,
                        r.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()?
            .map(|(kind, wrapped, ciphertext)| {
                Ok(SealedRecord {
                    id: *id,
                    kind: RecordKind::from_code(kind).ok_or(Error::Mismatch)?,
                    wrapped_key: WrappedKey::from_slice(&wrapped)?,
                    ciphertext,
                })
            })
            .transpose()
    }

    /// Decrypts a record. The document is not migrated; use
    /// [`Store::get_record`] for typed access.
    ///
    /// # Errors
    ///
    /// Crypto, model or database errors.
    pub fn get(&self, master: &MasterKey, id: &RecordId) -> Result<Option<Document>> {
        self.sealed(id)?
            .map(|sealed| open_sealed(master, &sealed))
            .transpose()
    }

    /// Decrypts, migrates and reads a typed record.
    ///
    /// # Errors
    ///
    /// [`Error::Model`] with `WrongKind` if the record is another kind, or
    /// `NewerSchema` if a newer app wrote it; crypto or database errors.
    pub fn get_record<R: Record>(&self, master: &MasterKey, id: &RecordId) -> Result<Option<R>> {
        self.get(master, id)?
            .map(|doc| Ok(R::from_document(&migrate(doc)?)?))
            .transpose()
    }

    /// Whether a record exists.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn contains(&self, id: &RecordId) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM records WHERE id = ?1",
                params![id.as_bytes()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// IDs of every record of `kind`.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn ids_of_kind(&self, kind: RecordKind) -> Result<Vec<RecordId>> {
        self.query_ids(
            "SELECT id FROM records WHERE kind = ?1 ORDER BY id",
            params![kind.code()],
        )
    }

    /// IDs of every record.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn all_ids(&self) -> Result<Vec<RecordId>> {
        self.query_ids("SELECT id FROM records ORDER BY id", [])
    }

    /// IDs of sessions and snippets dated within `first_day..=last_day`
    /// (days since the Unix epoch, in the time zone where each entry was
    /// recorded).
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn ids_on_days(&self, first_day: i64, last_day: i64) -> Result<Vec<RecordId>> {
        self.query_ids(
            "SELECT id FROM days WHERE day BETWEEN ?1 AND ?2 ORDER BY day, id",
            params![first_day, last_day],
        )
    }

    /// IDs of records that reference `id` (sessions using a product,
    /// products of a brand, snippets of a session, and so on).
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn referencing(&self, id: &RecordId) -> Result<Vec<RecordId>> {
        self.query_ids(
            "SELECT from_id FROM refs WHERE to_id = ?1 ORDER BY from_id",
            params![id.as_bytes()],
        )
    }

    /// IDs of records carrying a confirmed tag, by tag key (a vocabulary
    /// slug such as `relaxed`, or `custom:<hex id>`).
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn ids_with_tag(&self, tag: &str) -> Result<Vec<RecordId>> {
        self.query_ids(
            "SELECT id FROM tags WHERE tag = ?1 ORDER BY id",
            params![tag],
        )
    }

    fn query_ids(&self, sql: &str, params: impl rusqlite::Params) -> Result<Vec<RecordId>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params, |r| r.get::<_, Vec<u8>>(0))?;
        rows.map(|bytes| Ok(RecordId::from_slice(&bytes?)?))
            .collect()
    }

    /// Deletes a record by destroying its wrapped key, ciphertext, index
    /// rows and attached files, then truncates the WAL, and records a
    /// deletion marker (the ID only) for sync. Returns whether the record
    /// existed. Other records that reference it are left as they are
    /// (deleting a spot detaches it rather than deleting sessions).
    ///
    /// # Errors
    ///
    /// Database or file errors. If removing a file fails after the record
    /// is gone, the file is unreadable anyway (its key is destroyed).
    pub fn delete(&mut self, id: &RecordId) -> Result<bool> {
        let files = self.files_of(id)?;
        let tx = self.conn.transaction()?;
        clear_index(&tx, id)?;
        tx.execute("DELETE FROM files WHERE owner = ?1", params![id.as_bytes()])?;
        let existed = tx.execute("DELETE FROM records WHERE id = ?1", params![id.as_bytes()])? > 0;
        tx.execute(
            "INSERT OR IGNORE INTO deletions (id) VALUES (?1)",
            params![id.as_bytes()],
        )?;
        tx.commit()?;
        for file in files {
            remove_if_exists(&self.file_path(&file))?;
        }
        self.checkpoint()?;
        Ok(existed)
    }

    /// Whether a deletion marker exists for `id`.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn is_deleted(&self, id: &RecordId) -> Result<bool> {
        Ok(self
            .conn
            .query_row(
                "SELECT 1 FROM deletions WHERE id = ?1",
                params![id.as_bytes()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Every deletion marker.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn deletion_markers(&self) -> Result<Vec<RecordId>> {
        self.query_ids("SELECT id FROM deletions ORDER BY id", [])
    }

    /// Drops deletion markers (see `terpoir_sync::prunable_markers` for
    /// when that is safe).
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn remove_deletion_markers(&mut self, ids: &[RecordId]) -> Result<()> {
        let tx = self.conn.transaction()?;
        for id in ids {
            tx.execute(
                "DELETE FROM deletions WHERE id = ?1",
                params![id.as_bytes()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// A small piece of local state (for example the sync clock), stored
    /// inside the encrypted database.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Sets a piece of local state.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn set_meta(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Re-wraps every record key from `old` to `new` in one transaction,
    /// for master-key rotation. Ciphertext and files are unchanged. Returns
    /// the number of records.
    ///
    /// # Errors
    ///
    /// [`Error::Crypto`] if any record key does not unwrap under `old`
    /// (nothing is changed in that case).
    pub fn rewrap_all(&mut self, old: &MasterKey, new: &MasterKey) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let rows: Vec<(Vec<u8>, Vec<u8>)> = {
            let mut stmt = tx.prepare("SELECT id, wrapped_key FROM records")?;
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?
        };
        for (id, wrapped) in &rows {
            let record = RecordId::from_slice(id)?;
            let rewrapped =
                old.rewrap_record_key(&WrappedKey::from_slice(wrapped)?, &record, new)?;
            tx.execute(
                "UPDATE records SET wrapped_key = ?1 WHERE id = ?2",
                params![rewrapped.as_bytes().as_slice(), id],
            )?;
        }
        tx.commit()?;
        self.checkpoint()?;
        Ok(rows.len())
    }

    /// Checkpoints the WAL into the database and truncates it to zero
    /// bytes. Run after deletions and periodically.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn checkpoint(&self) -> Result<()> {
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(())
    }

    /// Rebuilds the database file so free pages (which `secure_delete`
    /// already zeroed) are dropped and the file shrinks. Run periodically.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn vacuum(&self) -> Result<()> {
        self.conn.execute_batch("VACUUM;")?;
        self.checkpoint()
    }

    fn wrapped_key_of(&self, id: &RecordId) -> Result<Option<(RecordKind, WrappedKey)>> {
        self.conn
            .query_row(
                "SELECT kind, wrapped_key FROM records WHERE id = ?1",
                params![id.as_bytes()],
                |r| Ok((r.get::<_, u8>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .optional()?
            .map(|(kind, wrapped)| {
                Ok((
                    RecordKind::from_code(kind).ok_or(Error::Mismatch)?,
                    WrappedKey::from_slice(&wrapped)?,
                ))
            })
            .transpose()
    }

    fn record_key(&self, master: &MasterKey, id: &RecordId) -> Result<RecordKey> {
        let (_, wrapped) = self.wrapped_key_of(id)?.ok_or(Error::NotFound)?;
        Ok(master.unwrap_record_key(&wrapped, id)?)
    }

    fn file_path(&self, file: &RecordId) -> PathBuf {
        self.files_dir.join(file.to_hex())
    }

    /// Encrypts a file (photo, voice clip) for record `owner` and returns
    /// its new file ID. Add the ID to the owning record (for a session,
    /// its `attachments`) and save the record.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] if `owner` does not exist; crypto, file or
    /// database errors otherwise.
    pub fn write_file(
        &mut self,
        master: &MasterKey,
        owner: &RecordId,
        reader: impl Read,
    ) -> Result<RecordId> {
        let key = self.record_key(master, owner)?;
        let file = RecordId::random()?;
        let path = self.file_path(&file);
        let tmp = path.with_extension("tmp");
        let result = (|| {
            let mut out = BufWriter::new(File::create(&tmp)?);
            encrypt_stream(&key, &file, reader, &mut out)?;
            out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
            fs::rename(&tmp, &path)?;
            self.conn.execute(
                "INSERT INTO files (id, owner) VALUES (?1, ?2)",
                params![file.as_bytes(), owner.as_bytes()],
            )?;
            Ok(file)
        })();
        if result.is_err() {
            let _ = remove_if_exists(&tmp);
            let _ = remove_if_exists(&path);
        }
        result
    }

    /// Decrypts a file into `writer`. If this fails, discard whatever was
    /// written: it may be incomplete or tampered.
    ///
    /// # Errors
    ///
    /// [`Error::NotFound`] if the file is unknown; [`Error::Crypto`] if it
    /// fails authentication.
    pub fn read_file(
        &self,
        master: &MasterKey,
        file: &RecordId,
        writer: impl Write,
    ) -> Result<u64> {
        let owner: Vec<u8> = self
            .conn
            .query_row(
                "SELECT owner FROM files WHERE id = ?1",
                params![file.as_bytes()],
                |r| r.get(0),
            )
            .optional()?
            .ok_or(Error::NotFound)?;
        let key = self.record_key(master, &RecordId::from_slice(&owner)?)?;
        let input = BufReader::new(File::open(self.file_path(file))?);
        Ok(decrypt_stream(&key, file, input, writer)?)
    }

    /// IDs of files attached to `owner`.
    ///
    /// # Errors
    ///
    /// Database errors.
    pub fn files_of(&self, owner: &RecordId) -> Result<Vec<RecordId>> {
        self.query_ids(
            "SELECT id FROM files WHERE owner = ?1 ORDER BY id",
            params![owner.as_bytes()],
        )
    }

    /// Deletes one attached file. Returns whether it existed. Remove its ID
    /// from the owning record too.
    ///
    /// # Errors
    ///
    /// Database or file errors.
    pub fn delete_file(&mut self, file: &RecordId) -> Result<bool> {
        let existed = self
            .conn
            .execute("DELETE FROM files WHERE id = ?1", params![file.as_bytes()])?
            > 0;
        remove_if_exists(&self.file_path(file))?;
        self.checkpoint()?;
        Ok(existed)
    }
}

fn key_pragma(key: &DatabaseKey) -> Zeroizing<String> {
    // Raw-key form: SQLCipher uses these 32 bytes directly, no KDF.
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut sql = Zeroizing::new(String::with_capacity(96));
    sql.push_str("PRAGMA key = \"x'");
    for byte in key.expose_secret() {
        sql.push(char::from(DIGITS[usize::from(byte >> 4)]));
        sql.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    sql.push_str("'\";");
    sql
}

fn open_sealed(master: &MasterKey, sealed: &SealedRecord) -> Result<Document> {
    let key = master.unwrap_record_key(&sealed.wrapped_key, &sealed.id)?;
    let plaintext = key.open(&record_aad(&sealed.id, sealed.kind), &sealed.ciphertext)?;
    let doc = Document::decode(&plaintext)?;
    if doc.id() != sealed.id || doc.kind() != sealed.kind {
        return Err(Error::Mismatch);
    }
    Ok(doc)
}

fn write_sealed(tx: &Transaction<'_>, sealed: &SealedRecord, entries: &IndexEntries) -> Result<()> {
    let id = sealed.id.as_bytes();
    tx.execute(
        "INSERT INTO records (id, kind, wrapped_key, ciphertext) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (id) DO UPDATE SET kind = excluded.kind,
             wrapped_key = excluded.wrapped_key, ciphertext = excluded.ciphertext",
        params![
            id,
            sealed.kind.code(),
            sealed.wrapped_key.as_bytes().as_slice(),
            sealed.ciphertext
        ],
    )?;
    clear_index(tx, &sealed.id)?;
    for to in &entries.refs {
        tx.execute(
            "INSERT INTO refs (from_id, to_id) VALUES (?1, ?2)",
            params![id, to.as_bytes()],
        )?;
    }
    if let Some(day) = entries.day {
        tx.execute(
            "INSERT INTO days (id, day) VALUES (?1, ?2)",
            params![id, day],
        )?;
    }
    for tag in &entries.tags {
        tx.execute(
            "INSERT INTO tags (id, tag) VALUES (?1, ?2)",
            params![id, tag],
        )?;
    }
    Ok(())
}

fn clear_index(tx: &Transaction<'_>, id: &RecordId) -> Result<()> {
    let id = id.as_bytes();
    tx.execute("DELETE FROM refs WHERE from_id = ?1", params![id])?;
    tx.execute("DELETE FROM days WHERE id = ?1", params![id])?;
    tx.execute("DELETE FROM tags WHERE id = ?1", params![id])?;
    Ok(())
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
