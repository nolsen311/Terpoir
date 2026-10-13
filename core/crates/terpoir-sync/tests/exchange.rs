// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Two devices with real encrypted stores syncing through blobs.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use terpoir_crypto::{DatabaseKey, MasterKey, RecordKey};
use terpoir_model::{Brand, DeviceId, Document, Hlc, LocalTime, RecordId, RecordKind, Session};
use terpoir_store::{PendingRecord, SealedRecord, Store};
use terpoir_sync::{
    Blob, Clock, apply_pending, container_key, encode_blob, export_blobs, import_blob,
};

struct Device {
    dir: PathBuf,
    store: Store,
    clock: Clock,
}

impl Device {
    fn new(n: u8) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "terpoir-sync-test-{}",
            RecordId::random().unwrap().to_hex()
        ));
        fs::create_dir_all(&dir).unwrap();
        let store = Store::open(
            &dir.join("journal.db"),
            &dir.join("files"),
            &DatabaseKey::generate().unwrap(),
        )
        .unwrap();
        Self {
            dir,
            store,
            clock: Clock::new(DeviceId::from_bytes([n; 16]), Hlc::default()),
        }
    }

    fn send(&self, master: &MasterKey) -> Vec<Vec<u8>> {
        export_blobs(&self.store, master, self.clock.device(), 1)
            .unwrap()
            .into_iter()
            .map(|(_, bytes)| bytes)
            .collect()
    }

    fn receive(&mut self, master: &MasterKey, blobs: &[Vec<u8>], now: u64) {
        for blob in blobs {
            import_blob(&mut self.store, master, &mut self.clock, blob, now).unwrap();
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn two_devices_converge_through_blobs() {
    // Both devices share the master key (the second restored it from the
    // recovery key); each has its own database key.
    let master = MasterKey::generate().unwrap();
    let mut phone = Device::new(1);
    let mut tablet = Device::new(2);

    let brand = Document::create(
        &Brand {
            name: "Acme".into(),
        },
        phone.clock.stamp(1_000),
    )
    .unwrap();
    phone.store.put(&master, &brand).unwrap();
    let session = Session::new(LocalTime {
        unix_ms: 1_000,
        utc_offset_min: 0,
    });
    let session_doc = Document::create(&session, phone.clock.stamp(1_000)).unwrap();
    phone.store.put(&master, &session_doc).unwrap();
    let photo = phone
        .store
        .write_file(&master, &session_doc.id(), &b"photo bytes"[..])
        .unwrap();

    tablet.receive(&master, &phone.send(&master), 2_000);

    // New records arrive unchanged: same wrapped key, same ciphertext.
    assert_eq!(
        tablet.store.sealed(&brand.id()).unwrap(),
        phone.store.sealed(&brand.id()).unwrap()
    );
    // So the record key is the same on both devices, and a file encrypted
    // on the phone decrypts with the tablet's copy of the key. (Moving the
    // file itself is the sync backend's job.)
    let mut out = Vec::new();
    let key = master
        .unwrap_record_key(
            &tablet
                .store
                .sealed(&session_doc.id())
                .unwrap()
                .unwrap()
                .wrapped_key,
            &session_doc.id(),
        )
        .unwrap();
    terpoir_crypto::decrypt_stream(
        &key,
        &photo,
        fs::File::open(phone.dir.join("files").join(photo.to_hex())).unwrap(),
        &mut out,
    )
    .unwrap();
    assert_eq!(out, b"photo bytes");

    // Concurrent edits to different fields both survive.
    let mut on_phone = phone.store.get(&master, &brand.id()).unwrap().unwrap();
    on_phone.set(
        "name",
        terpoir_model::Value::Text("Acme Co".into()),
        phone.clock.stamp(3_000),
    );
    phone.store.put(&master, &on_phone).unwrap();
    let mut on_tablet = tablet.store.get(&master, &brand.id()).unwrap().unwrap();
    on_tablet.set(
        "note",
        terpoir_model::Value::Int(5),
        tablet.clock.stamp(3_000),
    );
    tablet.store.put(&master, &on_tablet).unwrap();

    // The tablet deletes the session; the deletion reaches the phone.
    tablet.store.delete(&session_doc.id()).unwrap();

    let from_phone = phone.send(&master);
    let from_tablet = tablet.send(&master);
    phone.receive(&master, &from_tablet, 4_000);
    tablet.receive(&master, &from_phone, 4_000);

    for device in [&phone, &tablet] {
        let merged = device.store.get(&master, &brand.id()).unwrap().unwrap();
        assert_eq!(
            merged.value("name"),
            &terpoir_model::Value::Text("Acme Co".into())
        );
        assert_eq!(merged.value("note"), &terpoir_model::Value::Int(5));
        assert!(
            device
                .store
                .get(&master, &session_doc.id())
                .unwrap()
                .is_none()
        );
        assert!(device.store.is_deleted(&session_doc.id()).unwrap());
    }
    // The phone's copy of the session's files is gone too.
    assert!(!phone.dir.join("files").join(photo.to_hex()).exists());

    // Importing everything again changes nothing.
    let before = tablet.store.get(&master, &brand.id()).unwrap();
    tablet.receive(&master, &phone.send(&master), 5_000);
    assert_eq!(tablet.store.get(&master, &brand.id()).unwrap(), before);
}

#[test]
fn blobs_from_another_journal_are_rejected() {
    let mut device = Device::new(1);
    let mut other = Device::new(2);
    let other_master = MasterKey::generate().unwrap();
    let doc = Document::create(&Brand { name: "x".into() }, other.clock.stamp(1)).unwrap();
    other.store.put(&other_master, &doc).unwrap();
    let blobs = other.send(&other_master);
    let master = MasterKey::generate().unwrap();
    assert!(import_blob(&mut device.store, &master, &mut device.clock, &blobs[0], 1).is_err());
}

#[test]
fn clock_moves_past_imported_stamps() {
    let master = MasterKey::generate().unwrap();
    let mut a = Device::new(1);
    let mut b = Device::new(2);
    let doc = Document::create(&Brand { name: "x".into() }, a.clock.stamp(50_000)).unwrap();
    a.store.put(&master, &doc).unwrap();
    // b's wall clock is behind, but its next stamp still orders after a's.
    b.receive(&master, &a.send(&master), 10_000);
    assert!(b.clock.stamp(10_000) > doc.latest_stamp().unwrap());
}

/// A record sealed the way the store seals one, with any kind code and
/// any plaintext, as a newer app version might write it.
fn sealed_as(master: &MasterKey, kind_code: u8, plaintext: &[u8]) -> PendingRecord {
    let id = RecordId::random().unwrap();
    let key = RecordKey::generate().unwrap();
    let mut aad = id.as_bytes().to_vec();
    aad.push(kind_code);
    PendingRecord {
        id,
        kind_code,
        wrapped_key: master.wrap_record_key(&key, &id).unwrap(),
        ciphertext: key.seal(&aad, plaintext).unwrap(),
    }
}

#[test]
fn records_from_a_newer_version_are_kept_carried_and_applied_later() {
    let master = MasterKey::generate().unwrap();
    let mut old = Device::new(1);
    let mut other = Device::new(2);

    // A kind this version doesn't know, and a known kind in a newer
    // document envelope.
    let unknown = sealed_as(&master, 200, b"from the future");
    let newer = sealed_as(&master, RecordKind::Brand.code(), &[2, 0, 0]);
    let blob = Blob {
        device: DeviceId::from_bytes([9; 16]),
        sequence: 1,
        records: vec![newer.to_sealed().unwrap()],
        pending: vec![unknown.clone()],
        deletions: Vec::new(),
    };
    let bytes = encode_blob(&container_key(&master).unwrap(), &blob).unwrap();

    let summary = import_blob(&mut old.store, &master, &mut old.clock, &bytes, 1).unwrap();
    assert_eq!((summary.records, summary.deferred), (0, 2));
    assert_eq!(old.store.pending_count().unwrap(), 2);
    // Still unreadable: nothing applied, nothing lost.
    assert_eq!(
        apply_pending(&mut old.store, &master, &mut old.clock, 1).unwrap(),
        0
    );
    assert_eq!(old.store.pending().unwrap(), {
        let mut both = vec![unknown.clone(), newer.clone()];
        both.sort_by_key(|r| r.id);
        both
    });

    // Exports carry them, so a restore or another old device keeps them.
    other.receive(&master, &old.send(&master), 2);
    assert_eq!(other.store.pending().unwrap(), old.store.pending().unwrap());

    // Master key rotation keeps them openable.
    let rotated = MasterKey::generate().unwrap();
    other.store.rewrap_all(&master, &rotated).unwrap();
    for record in other.store.pending().unwrap() {
        rotated
            .unwrap_record_key(&record.wrapped_key, &record.id)
            .unwrap();
    }

    // A deletion drops a kept record, and it doesn't come back.
    let deletion = Blob {
        device: DeviceId::from_bytes([9; 16]),
        sequence: 2,
        records: Vec::new(),
        pending: Vec::new(),
        deletions: vec![unknown.id],
    };
    let key = container_key(&master).unwrap();
    old.receive(&master, &[encode_blob(&key, &deletion).unwrap(), bytes], 3);
    assert_eq!(old.store.pending().unwrap(), vec![newer]);

    // Once the app can read a kept record, it is merged and no longer kept.
    let brand = Document::create(
        &Brand {
            name: "Acme".into(),
        },
        old.clock.stamp(4),
    )
    .unwrap();
    let mut scratch = Device::new(3);
    scratch.store.put(&master, &brand).unwrap();
    let sealed: SealedRecord = scratch.store.sealed(&brand.id()).unwrap().unwrap();
    old.store
        .put_pending(&PendingRecord::from(&sealed))
        .unwrap();
    assert_eq!(
        apply_pending(&mut old.store, &master, &mut old.clock, 5).unwrap(),
        1
    );
    assert_eq!(old.store.get(&master, &brand.id()).unwrap(), Some(brand));
    assert_eq!(old.store.sealed(&sealed.id).unwrap(), Some(sealed));
    assert_eq!(old.store.pending_count().unwrap(), 1);
}
