// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The exported API, exercised the way the Android app will use it.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use terpoir_ffi::{
    BrandData, CoreError, Journal, LocalTimeData, NewKeys, ProductData, ProductLineData,
    RecordType, SessionData, SnippetData, SpotData, StrainData, SymptomData, TagData, TagIntensity,
    TagProvenance, TagRefData, check_recovery_key, create_keys, master_key_from_recovery,
};

struct Device {
    dir: PathBuf,
}

impl Device {
    fn new() -> Self {
        let keys = create_keys().unwrap();
        let dir =
            std::env::temp_dir().join(format!("terpoir-ffi-test-{}", hex(&keys.master_key[..8])));
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn open(&self, keys: &NewKeys) -> Result<Arc<Journal>, CoreError> {
        Journal::open(
            self.dir.join("journal.db").to_string_lossy().into_owned(),
            self.dir.join("files").to_string_lossy().into_owned(),
            keys.master_key.clone(),
            keys.wrapped_database_key.clone(),
        )
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn builtin(slug: &str) -> TagRefData {
    TagRefData::Builtin { slug: slug.into() }
}

fn session(product: Option<String>) -> SessionData {
    SessionData {
        when: LocalTimeData {
            unix_ms: 1_760_000_000_000,
            utc_offset_min: -420,
        },
        lines: vec![ProductLineData {
            line_id: None,
            product,
            method: "edible".into(),
            dose_milli: Some(5_000),
            dose_unit: Some("mg".into()),
        }],
        activity: Some("movie night".into()),
        intention: Some("relax".into()),
        mood_before: Some(3),
        social: Some("with_others".into()),
        effects: vec![TagIntensity {
            tag: builtin("relaxed"),
            value: 4,
        }],
        onset_min: Some(60),
        peak_min: None,
        duration_min: None,
        mood_after: Some(5),
        rating: Some(4),
        would_repeat: Some(true),
        spot: None,
        symptoms: vec![SymptomData {
            tag: builtin("sleep"),
            result: "helped".into(),
        }],
        attachments: vec![],
    }
}

#[test]
fn full_lifecycle() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    check_recovery_key(keys.recovery_key.clone()).unwrap();
    let journal = device.open(&keys).unwrap();

    let brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme".into(),
            },
            1,
        )
        .unwrap();
    let strain = journal
        .save_strain(
            None,
            StrainData {
                name: "Blue Dream".into(),
                lineage: None,
                terpenes: vec![builtin("myrcene")],
                strain_type: Some("hybrid".into()),
            },
            1,
        )
        .unwrap();
    let product = journal
        .save_product(
            None,
            ProductData {
                name: "Gummies".into(),
                form: Some("edible".into()),
                brand: Some(brand.clone()),
                strains: vec![strain.clone()],
                thc_milli: Some(5_000),
                thc_unit: Some("mg_per_unit".into()),
                cbd_milli: None,
                cbd_unit: None,
                terpenes: vec![],
                batch: None,
                bought_at: None,
                price_minor_units: Some(2_000),
                price_currency: Some("USD".into()),
            },
            1,
        )
        .unwrap();
    let id = journal
        .save_session(None, session(Some(product.clone())), 2)
        .unwrap();
    let tag = journal
        .save_tag(
            None,
            TagData {
                name: "giggly".into(),
                category: "effect".into(),
            },
            2,
        )
        .unwrap();
    let snippet = journal
        .save_snippet(
            None,
            SnippetData {
                session: Some(id.clone()),
                at: LocalTimeData {
                    unix_ms: 1_760_000_100_000,
                    utc_offset_min: -420,
                },
                text: "so giggly".into(),
                tags: vec![TagProvenance {
                    tag: TagRefData::Custom { id: tag.clone() },
                    provenance: "user".into(),
                }],
            },
            3,
        )
        .unwrap();
    let spot = journal
        .save_spot(
            None,
            SpotData {
                name: "Spot 1".into(),
                purposes: vec![builtin("chill")],
                lat_e7: Some(377_000_000),
                lon_e7: Some(-1_224_000_000),
                accuracy_m: Some(500),
                radius_m: Some(150),
            },
            3,
        )
        .unwrap();

    // Read back; the new line got an ID.
    let mut stored = journal.get_session(id.clone()).unwrap().unwrap();
    assert!(stored.lines[0].line_id.is_some());
    let mut expected = session(Some(product.clone()));
    expected.lines[0]
        .line_id
        .clone_from(&stored.lines[0].line_id);
    assert!(stored == expected);
    assert!(journal.get_product(product.clone()).unwrap().unwrap().brand == Some(brand.clone()));
    assert!(journal.get_spot(spot.clone()).unwrap().unwrap().radius_m == Some(150));
    assert!(journal.get_snippet(snippet.clone()).unwrap().unwrap().text == "so giggly");

    // Edit: attach the spot.
    stored.spot = Some(spot.clone());
    journal.save_session(Some(id.clone()), stored, 4).unwrap();

    // Queries.
    assert_eq!(journal.list(RecordType::Session).unwrap(), vec![id.clone()]);
    assert_eq!(
        journal.referencing(product.clone()).unwrap(),
        vec![id.clone()]
    );
    assert_eq!(journal.referencing(spot.clone()).unwrap(), vec![id.clone()]);
    assert_eq!(
        journal.referencing(brand.clone()).unwrap(),
        vec![product.clone()]
    );
    assert_eq!(
        journal.with_tag(builtin("relaxed")).unwrap(),
        vec![id.clone()]
    );
    assert_eq!(
        journal.with_tag(TagRefData::Custom { id: tag }).unwrap(),
        vec![snippet.clone()]
    );
    let day = 1_760_000_000_000i64.div_euclid(86_400_000) - 1;
    assert_eq!(journal.on_days(day, day + 1).unwrap().len(), 2);

    // Type mismatch and bad input.
    assert_eq!(
        journal.get_brand(id.clone()).err(),
        Some(CoreError::WrongType)
    );
    assert_eq!(
        journal.get_session("nope".into()).err(),
        Some(CoreError::InvalidInput)
    );
    let mut bad = session(None);
    bad.rating = Some(9);
    assert_eq!(
        journal.save_session(None, bad, 5).err(),
        Some(CoreError::InvalidInput)
    );

    // Attachments.
    let photo = journal
        .add_attachment(id.clone(), b"jpeg".to_vec(), 6)
        .unwrap();
    assert_eq!(
        journal
            .get_session(id.clone())
            .unwrap()
            .unwrap()
            .attachments,
        vec![photo.clone()]
    );
    assert_eq!(journal.read_attachment(photo.clone()).unwrap(), b"jpeg");
    assert!(
        journal
            .delete_attachment(id.clone(), photo.clone(), 7)
            .unwrap()
    );
    assert!(
        journal
            .get_session(id.clone())
            .unwrap()
            .unwrap()
            .attachments
            .is_empty()
    );

    // Lock: everything refuses until reopened.
    journal.lock();
    assert!(journal.is_locked());
    assert_eq!(
        journal.get_session(id.clone()).err(),
        Some(CoreError::Locked)
    );

    let journal = device.open(&keys).unwrap();
    assert!(journal.get_session(id.clone()).unwrap().is_some());

    // Delete.
    assert!(journal.delete_record(id.clone()).unwrap());
    assert!(journal.get_session(id).unwrap().is_none());
    journal.maintenance().unwrap();
}

#[test]
fn wrong_master_key_is_rejected() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    drop(device.open(&keys).unwrap());
    let other = create_keys().unwrap();
    let wrong = NewKeys {
        master_key: other.master_key,
        ..keys
    };
    assert_eq!(device.open(&wrong).err(), Some(CoreError::WrongKey));
}

#[test]
fn recovery_restores_the_master_key() {
    let keys = create_keys().unwrap();
    let restored = master_key_from_recovery(
        keys.recovery_key.clone(),
        keys.master_wrapped_by_recovery.clone(),
    )
    .unwrap();
    assert_eq!(restored, keys.master_key);

    let mut typo: Vec<char> = keys.recovery_key.chars().collect();
    typo[0] = if typo[0] == '2' { '3' } else { '2' };
    let typo: String = typo.into_iter().collect();
    assert!(check_recovery_key(typo).is_err());
    assert_eq!(
        check_recovery_key("ABC".into()).err(),
        Some(CoreError::RecoveryKeyLength)
    );
    let other = create_keys().unwrap();
    assert_eq!(
        master_key_from_recovery(other.recovery_key, keys.master_wrapped_by_recovery).err(),
        Some(CoreError::WrongKey)
    );
}

#[test]
fn two_devices_sync_through_blobs() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let tablet_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    // The tablet restores from the recovery key and gets its own database
    // key.
    let restored = master_key_from_recovery(
        keys.recovery_key.clone(),
        keys.master_wrapped_by_recovery.clone(),
    )
    .unwrap();
    let tablet_keys = create_keys().unwrap();
    let tablet = Journal::open(
        tablet_dir
            .dir
            .join("journal.db")
            .to_string_lossy()
            .into_owned(),
        tablet_dir.dir.join("files").to_string_lossy().into_owned(),
        restored,
        wrapped_db_key_for(&keys, &tablet_keys),
    )
    .unwrap();

    let id = phone.save_session(None, session(None), 10).unwrap();
    for blob in phone.export_sync_blobs().unwrap() {
        assert_eq!(blob.name.len(), 32);
        let result = tablet.import_sync_blob(blob.bytes, 20).unwrap();
        assert_eq!(result.deferred, 0);
    }
    assert!(tablet.get_session(id.clone()).unwrap() == phone.get_session(id.clone()).unwrap());

    tablet.delete_record(id.clone()).unwrap();
    for blob in tablet.export_sync_blobs().unwrap() {
        phone.import_sync_blob(blob.bytes, 30).unwrap();
    }
    assert!(phone.get_session(id).unwrap().is_none());

    // A blob from an unrelated journal is refused.
    let stranger_dir = Device::new();
    let stranger = stranger_dir.open(&create_keys().unwrap()).unwrap();
    stranger
        .save_brand(None, BrandData { name: "x".into() }, 1)
        .unwrap();
    let foreign = stranger.export_sync_blobs().unwrap().remove(0);
    assert_eq!(
        phone.import_sync_blob(foreign.bytes, 40).err(),
        Some(CoreError::InvalidSyncData)
    );
}

/// The tablet's own database key, wrapped by the shared master key.
fn wrapped_db_key_for(shared: &NewKeys, _tablet: &NewKeys) -> Vec<u8> {
    let master = terpoir_crypto::MasterKey::from_slice(&shared.master_key).unwrap();
    master
        .wrap_database_key(&terpoir_crypto::DatabaseKey::generate().unwrap())
        .unwrap()
        .as_bytes()
        .to_vec()
}

#[test]
fn rotation_keeps_the_journal_readable() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let id = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme".into(),
            },
            1,
        )
        .unwrap();

    let other = create_keys().unwrap();
    assert_eq!(
        journal
            .rotate_master_key(other.recovery_key, keys.master_wrapped_by_recovery.clone())
            .err(),
        Some(CoreError::WrongKey)
    );

    let rotated = journal
        .rotate_master_key(
            keys.recovery_key.clone(),
            keys.master_wrapped_by_recovery.clone(),
        )
        .unwrap();
    assert_ne!(rotated.master_key, keys.master_key);
    assert!(journal.get_brand(id.clone()).unwrap().is_some());
    journal.lock();

    // The old master key cannot unwrap the new database key...
    let stale = NewKeys {
        wrapped_database_key: rotated.wrapped_database_key.clone(),
        ..create_keys().unwrap()
    };
    let stale = NewKeys {
        master_key: keys.master_key.clone(),
        ..stale
    };
    assert_eq!(device.open(&stale).err(), Some(CoreError::WrongKey));
    // ...and even with a leftover copy of the old wrapped database key, it
    // cannot read records: their keys are wrapped by the new master key.
    let leftover = device.open(&keys).unwrap();
    assert_eq!(
        leftover.get_brand(id.clone()).err(),
        Some(CoreError::WrongKey)
    );
    leftover.lock();

    // The new master key works, and the recovery key unwraps it.
    let new_keys = NewKeys {
        recovery_key: keys.recovery_key.clone(),
        master_key: rotated.master_key.clone(),
        master_wrapped_by_recovery: rotated.master_wrapped_by_recovery.clone(),
        wrapped_database_key: rotated.wrapped_database_key.clone(),
    };
    let journal = device.open(&new_keys).unwrap();
    assert!(
        journal.get_brand(id).unwrap().unwrap()
            == BrandData {
                name: "Acme".into()
            }
    );
    assert_eq!(
        master_key_from_recovery(keys.recovery_key, rotated.master_wrapped_by_recovery).unwrap(),
        rotated.master_key
    );
}

#[test]
fn passphrase_wraps_the_master_key() {
    let keys = create_keys().unwrap();
    let wrapped = terpoir_ffi::wrap_master_key_with_passphrase(
        keys.master_key.clone(),
        "correct horse battery staple".into(),
    )
    .unwrap();
    let back = terpoir_ffi::master_key_from_passphrase(
        "correct horse battery staple".into(),
        terpoir_ffi::PassphraseWrapped {
            params: wrapped.params.clone(),
            wrapped_master_key: wrapped.wrapped_master_key.clone(),
        },
    )
    .unwrap();
    assert_eq!(back, keys.master_key);
    assert_eq!(
        terpoir_ffi::master_key_from_passphrase("wrong".into(), wrapped).err(),
        Some(CoreError::WrongKey)
    );
}
