// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! End-to-end tests against a real SQLCipher database on disk.

// Test code may unwrap (clippy.toml only covers `#[test]` functions, not
// helpers in integration-test files).
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use terpoir_crypto::{DatabaseKey, MasterKey, WrappedKey};
use terpoir_model::{
    Brand, CoreEffect, DeviceId, Document, Hlc, LocalTime, Method, Product, ProductLine, RecordId,
    RecordKind, Session, Snippet, Stamp, TagRef,
};
use terpoir_store::{Error, Store};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "terpoir-store-test-{}",
            RecordId::random().unwrap().to_hex()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A journal on disk: the database key is kept the way the app keeps it,
/// wrapped by the master key.
struct Journal {
    dir: TempDir,
    master: MasterKey,
    wrapped_db_key: WrappedKey,
}

impl Journal {
    fn new() -> Self {
        let master = MasterKey::generate().unwrap();
        let wrapped_db_key = master
            .wrap_database_key(&DatabaseKey::generate().unwrap())
            .unwrap();
        Self {
            dir: TempDir::new(),
            master,
            wrapped_db_key,
        }
    }

    fn db(&self) -> PathBuf {
        self.dir.0.join("journal.db")
    }

    fn files(&self) -> PathBuf {
        self.dir.0.join("files")
    }

    fn open(&self) -> Store {
        let key = self
            .master
            .unwrap_database_key(&self.wrapped_db_key)
            .unwrap();
        Store::open(&self.db(), &self.files(), &key).unwrap()
    }
}

fn stamp(ms: u64) -> Stamp {
    Stamp {
        hlc: Hlc {
            wall_ms: ms,
            counter: 0,
        },
        device: DeviceId::from_bytes([1; 16]),
    }
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_default()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

const DAY_MS: i64 = 86_400_000;

fn session_on_day(day: i64, note: &str) -> Session {
    let mut s = Session::new(LocalTime {
        unix_ms: day * DAY_MS + 12 * 3_600_000,
        utc_offset_min: 0,
    });
    s.activity = Some(note.to_owned());
    s
}

#[test]
fn put_get_round_trip_and_persistence() {
    let journal = Journal::new();
    let session = session_on_day(20_000, "walk in the park");
    let doc = Document::create(&session, stamp(1)).unwrap();
    {
        let mut store = journal.open();
        store.put(&journal.master, &doc).unwrap();
        assert_eq!(
            store.get(&journal.master, &doc.id()).unwrap(),
            Some(doc.clone())
        );
    }
    let store = journal.open();
    assert!(store.contains(&doc.id()).unwrap());
    assert_eq!(
        store
            .get_record::<Session>(&journal.master, &doc.id())
            .unwrap(),
        Some(session)
    );
    assert_eq!(
        store
            .get(&journal.master, &RecordId::random().unwrap())
            .unwrap(),
        None
    );
}

#[test]
fn wrong_database_key_is_rejected() {
    let journal = Journal::new();
    drop(journal.open());
    let wrong = DatabaseKey::generate().unwrap();
    assert_eq!(
        Store::open(&journal.db(), &journal.files(), &wrong).err(),
        Some(Error::WrongKey)
    );
}

#[test]
fn nothing_readable_on_disk() {
    let journal = Journal::new();
    let mut store = journal.open();
    let marker = "zebra-marker-7f3a";
    let doc = Document::create(&session_on_day(1, marker), stamp(1)).unwrap();
    store.put(&journal.master, &doc).unwrap();
    // Check before and after the WAL is folded into the main file.
    for _ in 0..2 {
        for path in [journal.db(), journal.db().with_extension("db-wal")] {
            let bytes = read(&path);
            assert!(!contains(&bytes, marker.as_bytes()));
            assert!(!contains(&bytes, b"SQLite format 3"));
            assert!(!contains(&bytes, doc.id().as_bytes()));
        }
        store.checkpoint().unwrap();
    }
}

#[test]
fn wrong_master_key_cannot_read_records() {
    let journal = Journal::new();
    let mut store = journal.open();
    let doc = Document::create(
        &Brand {
            name: "Acme".into(),
        },
        stamp(1),
    )
    .unwrap();
    store.put(&journal.master, &doc).unwrap();
    let other = MasterKey::generate().unwrap();
    assert!(matches!(
        store.get(&other, &doc.id()),
        Err(Error::Crypto(terpoir_crypto::Error::Decrypt))
    ));
}

#[test]
fn index_queries_follow_updates() {
    let journal = Journal::new();
    let mut store = journal.open();
    let brand = Document::create(
        &Brand {
            name: "Acme".into(),
        },
        stamp(1),
    )
    .unwrap();
    let mut product = Product::new("Gummies".into());
    product.brand = Some(brand.id());
    let product_doc = Document::create(&product, stamp(1)).unwrap();

    let mut session = session_on_day(100, "x");
    session.lines.insert(
        RecordId::random().unwrap(),
        ProductLine {
            product: Some(product_doc.id()),
            method: Method::Edible,
            dose: None,
        },
    );
    session.effects.insert(CoreEffect::Sleepy.into(), 4);
    let mut session_doc = Document::create(&session, stamp(1)).unwrap();
    let snippet = Document::create(
        &Snippet {
            session: Some(session_doc.id()),
            at: LocalTime {
                unix_ms: 101 * DAY_MS,
                utc_offset_min: 0,
            },
            text: "tasted like pine".into(),
            tags: [(
                TagRef::Builtin("pine".into()),
                terpoir_model::Provenance::User,
            )]
            .into(),
        },
        stamp(1),
    )
    .unwrap();
    for doc in [&brand, &product_doc, &session_doc, &snippet] {
        store.put(&journal.master, doc).unwrap();
    }

    assert_eq!(
        store.ids_of_kind(RecordKind::Session).unwrap(),
        vec![session_doc.id()]
    );
    assert_eq!(store.all_ids().unwrap().len(), 4);
    assert_eq!(
        store.referencing(&brand.id()).unwrap(),
        vec![product_doc.id()]
    );
    assert_eq!(
        store.referencing(&product_doc.id()).unwrap(),
        vec![session_doc.id()]
    );
    assert_eq!(
        store.referencing(&session_doc.id()).unwrap(),
        vec![snippet.id()]
    );
    assert_eq!(store.ids_on_days(100, 100).unwrap(), vec![session_doc.id()]);
    assert_eq!(store.ids_on_days(99, 101).unwrap().len(), 2);
    assert_eq!(
        store.ids_with_tag("sleepy").unwrap(),
        vec![session_doc.id()]
    );
    assert_eq!(store.ids_with_tag("pine").unwrap(), vec![snippet.id()]);

    // Move the session and drop its effect: the index follows.
    session = session_on_day(200, "x");
    session_doc.update(&session, stamp(2)).unwrap();
    store.put(&journal.master, &session_doc).unwrap();
    assert_eq!(store.ids_on_days(100, 100).unwrap(), vec![]);
    assert_eq!(store.ids_on_days(200, 200).unwrap(), vec![session_doc.id()]);
    assert!(store.ids_with_tag("sleepy").unwrap().is_empty());
    assert!(store.referencing(&product_doc.id()).unwrap().is_empty());
}

#[test]
fn kind_cannot_change() {
    let journal = Journal::new();
    let mut store = journal.open();
    let brand = Document::create(
        &Brand {
            name: "Acme".into(),
        },
        stamp(1),
    )
    .unwrap();
    store.put(&journal.master, &brand).unwrap();
    let impostor = Document::new(brand.id(), RecordKind::Tag);
    assert_eq!(
        store.put(&journal.master, &impostor).err(),
        Some(Error::Mismatch)
    );
}

#[test]
fn files_round_trip_and_are_encrypted() {
    let journal = Journal::new();
    let mut store = journal.open();
    let doc = Document::create(&session_on_day(1, "x"), stamp(1)).unwrap();
    store.put(&journal.master, &doc).unwrap();

    let photo: Vec<u8> = b"JPEG-MARKER-"
        .iter()
        .copied()
        .cycle()
        .take(200_000)
        .collect();
    let file = store
        .write_file(&journal.master, &doc.id(), photo.as_slice())
        .unwrap();
    assert_eq!(store.files_of(&doc.id()).unwrap(), vec![file]);

    let on_disk = read(&journal.files().join(file.to_hex()));
    assert!(!on_disk.is_empty());
    assert!(!contains(&on_disk, b"JPEG-MARKER-"));

    let mut out = Vec::new();
    store.read_file(&journal.master, &file, &mut out).unwrap();
    assert_eq!(out, photo);

    // Re-saving the record keeps its key, so the file still opens.
    store.put(&journal.master, &doc).unwrap();
    let mut out = Vec::new();
    store.read_file(&journal.master, &file, &mut out).unwrap();
    assert_eq!(out, photo);

    // Tampering is detected.
    let path = journal.files().join(file.to_hex());
    let mut tampered = read(&path);
    tampered[100] ^= 1;
    fs::write(&path, tampered).unwrap();
    assert!(matches!(
        store.read_file(&journal.master, &file, Vec::new()),
        Err(Error::Crypto(_))
    ));

    assert!(store.delete_file(&file).unwrap());
    assert!(!path.exists());
    assert_eq!(
        store.read_file(&journal.master, &file, Vec::new()).err(),
        Some(Error::NotFound)
    );
}

#[test]
fn write_file_needs_an_existing_owner() {
    let journal = Journal::new();
    let mut store = journal.open();
    assert_eq!(
        store
            .write_file(&journal.master, &RecordId::random().unwrap(), &b"x"[..])
            .err(),
        Some(Error::NotFound)
    );
}

#[test]
fn delete_erases_record_index_and_files() {
    let journal = Journal::new();
    let mut store = journal.open();
    let product = Document::create(&Product::new("p".into()), stamp(1)).unwrap();
    let mut session = session_on_day(5, "x");
    session.lines.insert(
        RecordId::random().unwrap(),
        ProductLine {
            product: Some(product.id()),
            method: Method::Smoked,
            dose: None,
        },
    );
    session.effects.insert(CoreEffect::Relaxed.into(), 2);
    let doc = Document::create(&session, stamp(1)).unwrap();
    store.put(&journal.master, &product).unwrap();
    store.put(&journal.master, &doc).unwrap();
    let file = store
        .write_file(&journal.master, &doc.id(), &b"photo"[..])
        .unwrap();

    assert!(store.delete(&doc.id()).unwrap());
    assert!(!store.delete(&doc.id()).unwrap());

    assert_eq!(store.get(&journal.master, &doc.id()).unwrap(), None);
    assert_eq!(store.sealed(&doc.id()).unwrap(), None);
    assert!(store.referencing(&product.id()).unwrap().is_empty());
    assert!(store.ids_on_days(5, 5).unwrap().is_empty());
    assert!(store.ids_with_tag("relaxed").unwrap().is_empty());
    assert!(store.files_of(&doc.id()).unwrap().is_empty());
    assert!(!journal.files().join(file.to_hex()).exists());
    // The product it referenced is untouched.
    assert!(store.contains(&product.id()).unwrap());
    // The WAL is truncated after deletion.
    assert_eq!(read(&journal.db().with_extension("db-wal")).len(), 0);
    // Only the ID remains, as a deletion marker for sync.
    assert!(store.is_deleted(&doc.id()).unwrap());
    assert!(!store.is_deleted(&product.id()).unwrap());
    assert_eq!(store.deletion_markers().unwrap(), vec![doc.id()]);
    store.remove_deletion_markers(&[doc.id()]).unwrap();
    assert!(store.deletion_markers().unwrap().is_empty());
}

#[test]
fn sealed_records_import_unchanged_and_are_verified() {
    let a = Journal::new();
    let mut store_a = a.open();
    let doc = Document::create(
        &Brand {
            name: "Acme".into(),
        },
        stamp(1),
    )
    .unwrap();
    store_a.put(&a.master, &doc).unwrap();
    let sealed = store_a.sealed(&doc.id()).unwrap().unwrap();

    // A second device with the same master key (restored from recovery).
    let b = Journal {
        dir: TempDir::new(),
        master: MasterKey::from_slice(a.master.expose_secret()).unwrap(),
        wrapped_db_key: a.wrapped_db_key.clone(),
    };
    let mut store_b = b.open();
    store_b.put_sealed(&b.master, &sealed).unwrap();
    assert_eq!(store_b.sealed(&doc.id()).unwrap(), Some(sealed.clone()));
    assert_eq!(
        store_b.get(&b.master, &doc.id()).unwrap(),
        Some(doc.clone())
    );

    let mut tampered = sealed.clone();
    let last = tampered.ciphertext.len() - 1;
    tampered.ciphertext[last] ^= 1;
    assert!(matches!(
        store_b.put_sealed(&b.master, &tampered),
        Err(Error::Crypto(_))
    ));

    let mut relabeled = sealed.clone();
    relabeled.kind = RecordKind::Tag;
    assert!(matches!(
        store_b.put_sealed(&b.master, &relabeled),
        Err(Error::Crypto(_))
    ));

    let mut moved = sealed;
    moved.id = RecordId::random().unwrap();
    assert!(matches!(
        store_b.put_sealed(&b.master, &moved),
        Err(Error::Crypto(_))
    ));
}

#[test]
fn master_key_rotation_rewraps_everything() {
    let journal = Journal::new();
    let mut store = journal.open();
    let doc = Document::create(&session_on_day(3, "x"), stamp(1)).unwrap();
    store.put(&journal.master, &doc).unwrap();
    let brand = Document::create(&Brand { name: "b".into() }, stamp(1)).unwrap();
    store.put(&journal.master, &brand).unwrap();
    let file = store
        .write_file(&journal.master, &doc.id(), &b"clip"[..])
        .unwrap();
    let ciphertext_before = store.sealed(&doc.id()).unwrap().unwrap().ciphertext;

    let new_master = MasterKey::generate().unwrap();
    assert_eq!(store.rewrap_all(&journal.master, &new_master).unwrap(), 2);

    assert!(store.get(&journal.master, &doc.id()).is_err());
    assert_eq!(
        store.get(&new_master, &doc.id()).unwrap(),
        Some(doc.clone())
    );
    assert_eq!(
        store.sealed(&doc.id()).unwrap().unwrap().ciphertext,
        ciphertext_before
    );
    let mut out = Vec::new();
    store.read_file(&new_master, &file, &mut out).unwrap();
    assert_eq!(out, b"clip");

    // A wrong old key changes nothing.
    let stranger = MasterKey::generate().unwrap();
    assert!(store.rewrap_all(&stranger, &journal.master).is_err());
    assert!(store.get(&new_master, &doc.id()).unwrap().is_some());
}

#[test]
fn vacuum_and_checkpoint_run() {
    let journal = Journal::new();
    let mut store = journal.open();
    for i in 0..50 {
        let doc = Document::create(&session_on_day(i, "filler"), stamp(1)).unwrap();
        store.put(&journal.master, &doc).unwrap();
        store.delete(&doc.id()).unwrap();
    }
    store.vacuum().unwrap();
    assert_eq!(read(&journal.db().with_extension("db-wal")).len(), 0);
}

#[test]
fn newer_database_is_refused() {
    let journal = Journal::new();
    drop(journal.open());
    {
        let key = journal
            .master
            .unwrap_database_key(&journal.wrapped_db_key)
            .unwrap();
        let conn = rusqlite_open(&journal, &key);
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    }
    let key = journal
        .master
        .unwrap_database_key(&journal.wrapped_db_key)
        .unwrap();
    assert_eq!(
        Store::open(&journal.db(), &journal.files(), &key).err(),
        Some(Error::NewerDatabase)
    );
}

fn rusqlite_open(journal: &Journal, key: &DatabaseKey) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(journal.db()).unwrap();
    let hex: String = key
        .expose_secret()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    conn.execute_batch(&format!("PRAGMA key = \"x'{hex}'\";"))
        .unwrap();
    conn
}

#[test]
fn sqlcipher_logging_is_compiled_out() {
    // With SQLCIPHER_OMIT_LOG (core/.cargo/config.toml), SQLCipher refuses
    // to set any log target and reports SQLITE_ERROR (1).
    let journal = Journal::new();
    drop(journal.open());
    let key = journal
        .master
        .unwrap_database_key(&journal.wrapped_db_key)
        .unwrap();
    let conn = rusqlite_open(&journal, &key);
    let status: String = conn
        .query_row("PRAGMA cipher_log = stderr", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "1");
}

#[test]
fn meta_round_trips() {
    let journal = Journal::new();
    {
        let mut store = journal.open();
        assert_eq!(store.meta("clock").unwrap(), None);
        store.set_meta("clock", b"one").unwrap();
        store.set_meta("clock", b"two").unwrap();
    }
    assert_eq!(journal.open().meta("clock").unwrap(), Some(b"two".to_vec()));
}

#[test]
fn version_one_databases_upgrade() {
    let journal = Journal::new();
    drop(journal.open());
    let key = journal
        .master
        .unwrap_database_key(&journal.wrapped_db_key)
        .unwrap();
    {
        let conn = rusqlite_open(&journal, &key);
        conn.execute_batch("DROP TABLE meta; DROP TABLE deletions; PRAGMA user_version = 1;")
            .unwrap();
    }
    let mut store = Store::open(&journal.db(), &journal.files(), &key).unwrap();
    store.set_meta("k", b"v").unwrap();
    assert!(store.deletion_markers().unwrap().is_empty());
}
