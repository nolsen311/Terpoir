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
    BrandData, CoreError, ExportArchive, IdentifierData, InsightFeedback, Journal, LocalTimeData,
    NewKeys, ProductData, ProductLineData, PurchaseData, PurchaseLineData, RecordType, SessionData,
    SnippetData, SpotData, StrainData, SymptomData, TagData, TagIntensity, TagProvenance,
    TagRefData, check_export_archive, check_recovery_key, create_keys, keys_for_restore,
    master_key_from_recovery, read_export_archive, trackable_data_points,
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
            purchase_id: None,
            purchase_line_id: None,
            shop: None,
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
                package_date_day: Some(20_300),
                identifiers: vec![IdentifierData {
                    code: "https://example.test/coa/1".into(),
                    kind: "qr".into(),
                }],
                shops: vec![],
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
                added_while_locked: false,
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
                purchase_prompts: true,
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
        journal
            .with_tag(TagRefData::Custom { id: tag.clone() })
            .unwrap(),
        vec![snippet.clone()]
    );
    let day = 1_760_000_000_000i64.div_euclid(86_400_000) - 1;
    assert_eq!(journal.on_days(day, day + 1).unwrap().len(), 2);

    // A purchase at the spot, and the session's line pointing at it.
    let purchase = journal
        .save_purchase(
            None,
            PurchaseData {
                when: LocalTimeData {
                    unix_ms: 1_759_900_000_000,
                    utc_offset_min: -420,
                },
                spot: Some(spot.clone()),
                lines: vec![PurchaseLineData {
                    line_id: None,
                    product: Some(product.clone()),
                    batch: Some("B-7".into()),
                    price_minor_units: Some(2_000),
                    price_currency: Some("USD".into()),
                }],
            },
            4,
        )
        .unwrap();
    let bought = journal.get_purchase(purchase.clone()).unwrap().unwrap();
    assert!(bought.lines[0].price_currency.as_deref() == Some("USD"));
    assert!(bought.lines[0].line_id.is_some());
    assert_eq!(
        journal.list(RecordType::Purchase).unwrap(),
        vec![purchase.clone()]
    );
    let mut linked = journal.get_session(id.clone()).unwrap().unwrap();
    linked.lines[0].purchase_id = Some(purchase.clone());
    linked.lines[0]
        .purchase_line_id
        .clone_from(&bought.lines[0].line_id);
    journal
        .save_session(Some(id.clone()), linked.clone(), 4)
        .unwrap();
    assert!(journal.get_session(id.clone()).unwrap().unwrap() == linked);
    let mut refs = journal.referencing(purchase.clone()).unwrap();
    refs.sort();
    assert_eq!(refs, vec![id.clone()]);
    let stored_product = journal.get_product(product.clone()).unwrap().unwrap();
    assert_eq!(stored_product.package_date_day, Some(20_300));
    assert!(stored_product.identifiers[0].kind == "qr");
    assert!(
        journal
            .get_spot(spot.clone())
            .unwrap()
            .unwrap()
            .purchase_prompts
    );

    // Tag suggestions: the custom "giggly" effect wins over the built-in
    // synonym; negated mentions are skipped.
    let suggestions = journal
        .suggest_tags("So giggly and relaxed, tasted lemony. Not anxious.".into())
        .unwrap();
    let found: Vec<(String, u32, u32)> = suggestions
        .iter()
        .map(|s| {
            let tag = match &s.tag {
                TagRefData::Builtin { slug } => slug.clone(),
                TagRefData::Custom { id } => format!("custom:{id}"),
            };
            (tag, s.start, s.end)
        })
        .collect();
    assert_eq!(
        found,
        vec![
            (format!("custom:{tag}"), 3, 9),
            ("relaxed".into(), 14, 21),
            ("citrus".into(), 30, 36),
        ]
    );

    // Duplicate detection and shop matching.
    let by_code = journal
        .find_products(None, None, vec![" https://example.test/coa/1 ".into()])
        .unwrap();
    assert_eq!(by_code.len(), 1);
    assert!(by_code[0].by_code && by_code[0].id == product);
    let by_name = journal
        .find_products(Some("  gummies ".into()), Some(brand.clone()), vec![])
        .unwrap();
    assert!(by_name.len() == 1 && !by_name[0].by_code);
    let other_brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Other".into(),
            },
            4,
        )
        .unwrap();
    assert!(
        journal
            .find_products(Some("Gummies".into()), Some(other_brand), vec![])
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        journal.match_spot("SPOT 1".into()).unwrap(),
        Some(spot.clone())
    );
    assert_eq!(journal.match_spot("Elsewhere".into()).unwrap(), None);
    // "Show me": the spot (377_000_000, -1_224_000_000, radius 150 m) is
    // near a fix ~111 m away, not one ~1.1 km away.
    let near = journal.spots_near(377_010_000, -1_224_000_000, 20).unwrap();
    assert_eq!(near.len(), 1);
    assert!(near[0].id == spot && near[0].name == "Spot 1");
    // Not opted in to purchase prompts: nothing to watch for them.
    assert!(journal.purchase_prompt_fences().unwrap().is_empty());
    assert!(
        (105..=118).contains(&near[0].distance_m),
        "{}",
        near[0].distance_m
    );
    assert!(
        journal
            .spots_near(377_100_000, -1_224_000_000, 20)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        journal.spots_near(900_000_001, 0, 1).err(),
        Some(CoreError::InvalidInput)
    );
    // "Notify me": the spot with what an alert may show, from the one
    // session attached to it.
    let alerts = journal.spot_alerts().unwrap();
    assert_eq!(alerts.len(), 1);
    let alert = &alerts[0];
    assert!(alert.lat_e7 == 377_000_000 && alert.lon_e7 == -1_224_000_000 && alert.radius_m == 150);
    assert_eq!(alert.name, "Spot 1");
    assert_eq!(alert.sessions, 1);
    assert_eq!(alert.average_rating_tenths, Some(40));
    assert_eq!(alert.last_product.as_deref(), Some("Gummies"));

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
    let mut half_ref = session(None);
    half_ref.lines[0].purchase_id = Some(purchase.clone());
    assert_eq!(
        journal.save_session(None, half_ref, 5).err(),
        Some(CoreError::InvalidInput)
    );
    let mut half_price = bought.clone();
    half_price.lines[0].price_currency = None;
    assert_eq!(
        journal.save_purchase(None, half_price, 5).err(),
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
    // Nothing from a newer app version to keep or apply.
    assert_eq!(tablet.pending_record_count().unwrap(), 0);
    assert_eq!(tablet.apply_pending_records(20).unwrap(), 0);
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

#[test]
fn an_export_restores_on_a_new_device() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    let id = phone.save_session(None, session(None), 10).unwrap();
    let archive = phone
        .export_archive(keys.master_wrapped_by_recovery.clone())
        .unwrap();

    // Truncated, padded or foreign files are refused before any decryption.
    for bad in [
        archive[..archive.len() - 1].to_vec(),
        [archive.clone(), vec![0]].concat(),
        b"not an export".to_vec(),
    ] {
        assert_eq!(
            read_export_archive(bad).err(),
            Some(CoreError::InvalidSyncData)
        );
    }

    // The new phone needs the recovery key; another journal's is refused.
    let parsed = read_export_archive(archive).unwrap();
    assert_eq!(
        parsed.master_wrapped_by_recovery,
        keys.master_wrapped_by_recovery
    );
    let other = create_keys().unwrap();
    assert!(
        keys_for_restore(
            other.recovery_key.clone(),
            parsed.master_wrapped_by_recovery.clone()
        )
        .is_err()
    );
    let restored = keys_for_restore(
        keys.recovery_key.clone(),
        parsed.master_wrapped_by_recovery.clone(),
    )
    .unwrap();
    assert_eq!(restored.master_key, keys.master_key);

    // Checked before anything changes: everything readable, nothing newer.
    let check = |blobs: Vec<Vec<u8>>| {
        check_export_archive(
            keys.recovery_key.clone(),
            ExportArchive {
                master_wrapped_by_recovery: keys.master_wrapped_by_recovery.clone(),
                blobs,
            },
        )
        .unwrap()
    };
    let current = check(parsed.blobs.clone());
    assert_eq!(
        (
            current.records,
            current.newer_records,
            current.newer_format_blobs
        ),
        (1, 0, 0)
    );
    // A record of a kind a newer app added is counted, not an error.
    let master = terpoir_crypto::MasterKey::from_slice(&keys.master_key).unwrap();
    let mut newer = terpoir_sync::Blob {
        device: terpoir_model::DeviceId::from_bytes([7; 16]),
        sequence: 1,
        records: Vec::new(),
        pending: Vec::new(),
        deletions: Vec::new(),
    };
    let record = terpoir_model::RecordId::random().unwrap();
    let record_key = terpoir_crypto::RecordKey::generate().unwrap();
    newer.pending.push(terpoir_store::PendingRecord {
        id: record,
        kind_code: 200,
        wrapped_key: master.wrap_record_key(&record_key, &record).unwrap(),
        ciphertext: record_key.seal(b"", b"later").unwrap(),
    });
    let newer =
        terpoir_sync::encode_blob(&terpoir_sync::container_key(&master).unwrap(), &newer).unwrap();
    let mixed = check([parsed.blobs.clone(), vec![newer]].concat());
    assert_eq!((mixed.records, mixed.newer_records), (1, 1));

    let new_dir = Device::new();
    let new_phone = Journal::open(
        new_dir
            .dir
            .join("journal.db")
            .to_string_lossy()
            .into_owned(),
        new_dir.dir.join("files").to_string_lossy().into_owned(),
        restored.master_key,
        restored.wrapped_database_key,
    )
    .unwrap();
    for blob in parsed.blobs {
        assert_eq!(new_phone.import_sync_blob(blob, 20).unwrap().deferred, 0);
    }
    assert!(new_phone.get_session(id.clone()).unwrap() == phone.get_session(id).unwrap());
    // Importing into a journal checks under its own key.
    let again = read_export_archive(
        phone
            .export_archive(keys.master_wrapped_by_recovery.clone())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(new_phone.check_export_archive(again).unwrap().records, 1);
}

#[test]
fn the_plaintext_export_has_every_record_readably() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Fire Bros".into(),
            },
            1,
        )
        .unwrap();
    let product = journal
        .save_product(
            None,
            ProductData {
                name: "Block Berry, \"live\" cart".into(),
                form: Some("vape_cart".into()),
                brand: Some(brand),
                strains: vec![],
                thc_milli: Some(85_500),
                thc_unit: Some("percent".into()),
                cbd_milli: None,
                cbd_unit: None,
                terpenes: vec![],
                batch: Some("=1+1".into()),
                package_date_day: Some(20_737),
                identifiers: vec![],
                shops: vec![],
            },
            2,
        )
        .unwrap();
    let session_id = journal
        .save_session(None, session(Some(product.clone())), 3)
        .unwrap();

    let files = journal.export_plaintext(10).unwrap();
    let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "terpoir.json",
            "sessions.csv",
            "products.csv",
            "purchases.csv"
        ]
    );

    let json: serde_json::Value = serde_json::from_slice(&files[0].bytes).unwrap();
    assert_eq!(json["format"], "terpoir-plaintext");
    let sessions = json["records"]["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["id"], session_id);
    assert_eq!(sessions[0]["fields"]["effects"]["relaxed"], 4);
    assert_eq!(json["records"]["products"][0]["id"], product);

    let sessions_csv = String::from_utf8(files[1].bytes.clone()).unwrap();
    let mut rows = sessions_csv.split("\r\n");
    assert!(rows.next().unwrap().starts_with("id,date,time,"));
    let row = rows.next().unwrap();
    assert!(row.contains(&session_id));
    // The product's label carries its brand, quoted for its comma and quotes.
    assert!(row.contains("\"Fire Bros Block Berry, \"\"live\"\" cart\""));
    assert!(row.contains("relaxed=4"));

    let products_csv = String::from_utf8(files[2].bytes.clone()).unwrap();
    // A batch that looks like a formula is defused; the date is ISO.
    assert!(products_csv.contains(",'=1+1,2026-10-11"));
    assert!(products_csv.contains(",85.5,percent,"));
}

#[test]
fn deleting_symptoms_keeps_the_rest_of_each_session() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let custom = journal
        .save_tag(
            None,
            TagData {
                name: "Cramps".into(),
                category: "symptom".into(),
            },
            5,
        )
        .unwrap();
    let effect = journal
        .save_tag(
            None,
            TagData {
                name: "Giggly".into(),
                category: "effect".into(),
            },
            6,
        )
        .unwrap();
    let mut with_symptoms = session(None);
    with_symptoms.symptoms.push(SymptomData {
        tag: TagRefData::Custom { id: custom.clone() },
        result: "not_rated".into(),
    });
    let first = journal.save_session(None, with_symptoms, 10).unwrap();
    let mut without = session(None);
    without.symptoms.clear();
    let second = journal.save_session(None, without, 11).unwrap();

    assert_eq!(journal.delete_symptoms(20).unwrap(), 1);
    let first = journal.get_session(first).unwrap().unwrap();
    assert!(first.symptoms.is_empty());
    assert_eq!(first.effects.len(), 1);
    assert_eq!(first.rating, Some(4));
    assert!(journal.get_session(second).unwrap().is_some());
    assert!(journal.get_tag(custom).unwrap().is_none());
    assert!(journal.get_tag(effect).unwrap().is_some());
    // Nothing left to delete.
    assert_eq!(journal.delete_symptoms(30).unwrap(), 0);
}

#[test]
fn insights_need_evidence_and_respect_feedback() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let product = |name: &str, terpene: &str| ProductData {
        name: name.into(),
        form: Some("flower".into()),
        brand: None,
        strains: vec![],
        thc_milli: None,
        thc_unit: None,
        cbd_milli: None,
        cbd_unit: None,
        terpenes: vec![TagIntensity {
            tag: builtin(terpene),
            value: 800,
        }],
        batch: None,
        package_date_day: None,
        identifiers: vec![],
        shops: vec![],
    };
    let heavy = journal
        .save_product(None, product("Heavy", "myrcene"), 1)
        .unwrap();
    let bright = journal
        .save_product(None, product("Bright", "limonene"), 2)
        .unwrap();
    let rated = |product: &String, rating: u8| {
        let mut s = session(Some(product.clone()));
        s.rating = Some(rating);
        s
    };
    // Four a side is not enough evidence.
    let mut heavy_sessions = Vec::new();
    for i in 0..4 {
        heavy_sessions.push(
            journal
                .save_session(None, rated(&heavy, 2), 10 + i)
                .unwrap(),
        );
        journal
            .save_session(None, rated(&bright, 4), 20 + i)
            .unwrap();
    }
    assert!(journal.insights().unwrap().is_empty());

    heavy_sessions.push(journal.save_session(None, rated(&heavy, 2), 30).unwrap());
    journal.save_session(None, rated(&bright, 4), 31).unwrap();
    let found = journal.insights().unwrap();
    let myrcene = found.iter().find(|i| i.key == "terpene:myrcene").unwrap();
    assert_eq!(myrcene.category, "terpene");
    assert_eq!(myrcene.subject, "myrcene");
    assert_eq!(myrcene.with_avg_milli, 2_000);
    assert_eq!(myrcene.without_avg_milli, 4_000);
    assert_eq!(myrcene.without_count, 5);
    let mut with = myrcene.with_sessions.clone();
    with.sort();
    heavy_sessions.sort();
    assert_eq!(with, heavy_sessions);

    journal
        .set_insight_feedback("terpene:myrcene".into(), InsightFeedback::Dismissed, 32)
        .unwrap();
    let after = journal.insights().unwrap();
    assert!(after.iter().all(|i| i.key != "terpene:myrcene"));
    assert!(after.iter().any(|i| i.key == "terpene:limonene"));
    journal
        .set_insight_feedback("terpene".into(), InsightFeedback::Muted, 33)
        .unwrap();
    assert!(
        journal
            .insights()
            .unwrap()
            .iter()
            .all(|i| i.category != "terpene")
    );
    journal.forget_insight_feedback(34).unwrap();
    assert!(
        journal
            .insights()
            .unwrap()
            .iter()
            .any(|i| i.key == "terpene:myrcene")
    );

    // "What I track": with terpenes hidden, no insight is built on them.
    journal
        .set_data_point_tracked("terpenes".into(), false, 40)
        .unwrap();
    assert!(
        journal
            .insights()
            .unwrap()
            .iter()
            .all(|i| i.category != "terpene")
    );
}

/// Five sessions a side with a myrcene product (rated 2) and a limonene
/// one (rated 4): enough evidence for terpene insights on both.
fn with_terpene_insights(journal: &Journal) {
    for (name, terpene, rating) in [("Heavy", "myrcene", 2), ("Bright", "limonene", 4)] {
        let product = journal
            .save_product(
                None,
                ProductData {
                    name: name.into(),
                    form: Some("flower".into()),
                    brand: None,
                    strains: vec![],
                    thc_milli: None,
                    thc_unit: None,
                    cbd_milli: None,
                    cbd_unit: None,
                    terpenes: vec![TagIntensity {
                        tag: builtin(terpene),
                        value: 800,
                    }],
                    batch: None,
                    package_date_day: None,
                    identifiers: vec![],
                    shops: vec![],
                },
                1,
            )
            .unwrap();
        for i in 0..5 {
            let mut s = session(Some(product.clone()));
            s.rating = Some(rating);
            journal.save_session(None, s, 10 + i).unwrap();
        }
    }
}

fn shown(journal: &Journal) -> Vec<String> {
    let mut keys: Vec<String> = journal
        .insights()
        .unwrap()
        .into_iter()
        .map(|i| i.key)
        .collect();
    keys.sort();
    keys
}

#[test]
fn insight_feedback_syncs_and_travels_in_exports() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    with_terpene_insights(&phone);
    let both = vec!["terpene:limonene".to_owned(), "terpene:myrcene".to_owned()];
    assert_eq!(shown(&phone), both);

    // Dismissed on the phone, then restored from an export on a tablet:
    // the tablet hides it too.
    phone
        .set_insight_feedback("terpene:myrcene".into(), InsightFeedback::Dismissed, 100)
        .unwrap();
    let archive = read_export_archive(
        phone
            .export_archive(keys.master_wrapped_by_recovery.clone())
            .unwrap(),
    )
    .unwrap();
    let restored = keys_for_restore(
        keys.recovery_key.clone(),
        archive.master_wrapped_by_recovery.clone(),
    )
    .unwrap();
    let tablet_dir = Device::new();
    let tablet = Journal::open(
        tablet_dir
            .dir
            .join("journal.db")
            .to_string_lossy()
            .into_owned(),
        tablet_dir.dir.join("files").to_string_lossy().into_owned(),
        restored.master_key,
        restored.wrapped_database_key,
    )
    .unwrap();
    for blob in archive.blobs {
        tablet.import_sync_blob(blob, 110).unwrap();
    }
    assert_eq!(shown(&tablet), vec!["terpene:limonene".to_owned()]);

    // Feedback given on the tablet syncs back, merging per insight.
    tablet
        .set_insight_feedback("terpene:limonene".into(), InsightFeedback::Wrong, 120)
        .unwrap();
    sync(&tablet, &phone, 130);
    assert!(shown(&phone).is_empty());

    // Forgetting all feedback (the toggle's off-action) reaches every device.
    phone.forget_insight_feedback(140).unwrap();
    assert_eq!(shown(&phone), both);
    sync(&phone, &tablet, 150);
    assert_eq!(shown(&tablet), both);

    // Master-key rotation keeps it.
    phone
        .set_insight_feedback("terpene".into(), InsightFeedback::Muted, 160)
        .unwrap();
    phone
        .rotate_master_key(
            keys.recovery_key.clone(),
            keys.master_wrapped_by_recovery.clone(),
        )
        .unwrap();
    assert!(shown(&phone).is_empty());
}

#[test]
fn feedback_kept_on_the_device_by_earlier_versions_moves_into_the_journal() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let phone = dir.open(&keys).unwrap();
    with_terpene_insights(&phone);
    phone.lock();
    drop(phone);

    // Where earlier versions kept it: this device's local metadata.
    let master = terpoir_crypto::MasterKey::from_slice(&keys.master_key).unwrap();
    let database_key = master
        .unwrap_database_key(
            &terpoir_crypto::WrappedKey::from_slice(&keys.wrapped_database_key).unwrap(),
        )
        .unwrap();
    {
        let mut store = terpoir_store::Store::open(
            &dir.dir.join("journal.db"),
            &dir.dir.join("files"),
            &database_key,
        )
        .unwrap();
        store
            .set_meta("insight_feedback", b"terpene:myrcene\tdismissed\n")
            .unwrap();
    }

    let phone = dir.open(&keys).unwrap();
    assert_eq!(shown(&phone), vec!["terpene:limonene".to_owned()]);
    // It is a synced record now: another device sees it.
    let tablet_dir = Device::new();
    let tablet = Journal::open(
        tablet_dir
            .dir
            .join("journal.db")
            .to_string_lossy()
            .into_owned(),
        tablet_dir.dir.join("files").to_string_lossy().into_owned(),
        keys.master_key.clone(),
        wrapped_db_key_for(&keys, &keys),
    )
    .unwrap();
    sync(&phone, &tablet, 200);
    assert_eq!(shown(&tablet), vec!["terpene:limonene".to_owned()]);
    // And the device-only copy is gone.
    phone.lock();
    drop(phone);
    let store = terpoir_store::Store::open(
        &dir.dir.join("journal.db"),
        &dir.dir.join("files"),
        &database_key,
    )
    .unwrap();
    assert_eq!(store.meta("insight_feedback").unwrap(), None);
}

#[test]
fn shop_insights_attribute_sessions_to_where_products_were_bought() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let shop = |name: &str| {
        journal
            .save_spot(
                None,
                SpotData {
                    name: name.into(),
                    purposes: vec![builtin("re_up")],
                    lat_e7: None,
                    lon_e7: None,
                    accuracy_m: None,
                    radius_m: None,
                    purchase_prompts: false,
                },
                1,
            )
            .unwrap()
    };
    let (a, b) = (shop("Shop A"), shop("Shop B"));
    let product = |name: &str| {
        journal
            .save_product(
                None,
                ProductData {
                    name: name.into(),
                    form: None,
                    brand: None,
                    strains: vec![],
                    thc_milli: None,
                    thc_unit: None,
                    cbd_milli: None,
                    cbd_unit: None,
                    terpenes: vec![],
                    batch: None,
                    // 30 days before the purchases below.
                    package_date_day: Some(20_340),
                    identifiers: vec![],
                    shops: vec![],
                },
                2,
            )
            .unwrap()
    };
    let (gummies, cart) = (product("Gummies"), product("Cart"));
    let buy = |spot: &String, product: &String| {
        let id = journal
            .save_purchase(
                None,
                PurchaseData {
                    when: LocalTimeData {
                        unix_ms: 1_760_000_000_000,
                        utc_offset_min: -420,
                    },
                    spot: Some(spot.clone()),
                    lines: vec![PurchaseLineData {
                        line_id: None,
                        product: Some(product.clone()),
                        batch: None,
                        price_minor_units: None,
                        price_currency: None,
                    }],
                },
                3,
            )
            .unwrap();
        let line = journal.get_purchase(id.clone()).unwrap().unwrap().lines[0]
            .line_id
            .clone();
        (id, line)
    };
    // Gummies only ever come from Shop A: their sessions count there
    // without naming the purchase.
    buy(&a, &gummies);
    // The cart comes from both shops; its sessions name their purchase.
    let cart_a = buy(&a, &cart);
    let cart_b = buy(&b, &cart);
    let rated = |product: &String, purchase: Option<&(String, Option<String>)>, rating: u8| {
        let mut s = session(Some(product.clone()));
        s.rating = Some(rating);
        s.would_repeat = None;
        if let Some((id, line)) = purchase {
            s.lines[0].purchase_id = Some(id.clone());
            s.lines[0].purchase_line_id = line.clone();
        }
        s
    };
    for i in 0..5 {
        journal
            .save_session(None, rated(&gummies, None, 2), 10 + i)
            .unwrap();
        journal
            .save_session(None, rated(&cart, Some(&cart_a), 5), 20 + i)
            .unwrap();
        journal
            .save_session(None, rated(&cart, Some(&cart_b), 3), 30 + i)
            .unwrap();
    }
    // A cart session that names no purchase has no known shop: left out.
    journal
        .save_session(None, rated(&cart, None, 1), 40)
        .unwrap();
    // One where the user picked the shop ("From:") counts there.
    let mut picked = rated(&cart, None, 3);
    picked.lines[0].shop = Some(b.clone());
    let picked = journal.save_session(None, picked, 41).unwrap();
    assert_eq!(
        journal.get_session(picked).unwrap().unwrap().lines[0].shop,
        Some(b.clone())
    );

    // Receipts filled each product's "Where I buy it".
    let mut cart_shops = journal.get_product(cart.clone()).unwrap().unwrap().shops;
    cart_shops.sort();
    let mut both = vec![a.clone(), b.clone()];
    both.sort();
    assert_eq!(cart_shops, both);
    assert_eq!(
        journal.get_product(gummies.clone()).unwrap().unwrap().shops,
        vec![a.clone()]
    );

    let found = journal.shop_insights().unwrap();
    let shop_a = found.iter().find(|s| s.spot == a).unwrap();
    assert_eq!(shop_a.name, "Shop A");
    assert_eq!(shop_a.sessions.len(), 10);
    // Five 2★ gummies and five 5★ carts.
    assert_eq!(shop_a.avg_rating_milli, Some(3_500));
    assert_eq!(shop_a.hit_rate_percent, Some(50));
    // Freshness only from the purchases sessions name (the carts).
    assert_eq!(shop_a.avg_age_days, Some(30));
    assert_eq!(shop_a.effects.len(), 1);
    let effect = &shop_a.effects[0];
    assert!(effect.level == "product" && effect.subject == cart && effect.label == "Cart");
    assert_eq!(
        (effect.here_avg_milli, effect.elsewhere_avg_milli),
        (5_000, 3_000)
    );
    let shop_b = found.iter().find(|s| s.spot == b).unwrap();
    assert_eq!(shop_b.sessions.len(), 6);
    assert_eq!(shop_b.avg_rating_milli, Some(3_000));

    // "What I track": package dates hidden, no freshness; prices and shops
    // hidden, no shop insights at all.
    journal
        .set_data_point_tracked("batch".into(), false, 50)
        .unwrap();
    let found = journal.shop_insights().unwrap();
    assert!(found.iter().all(|s| s.avg_age_days.is_none()));
    journal
        .set_data_point_tracked("price_and_shop".into(), false, 51)
        .unwrap();
    assert!(journal.shop_insights().unwrap().is_empty());
}

#[test]
fn shop_insights_compare_brands_and_strains_across_shops() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let spot = |name: &str| {
        journal
            .save_spot(
                None,
                SpotData {
                    name: name.into(),
                    purposes: vec![builtin("re_up")],
                    lat_e7: None,
                    lon_e7: None,
                    accuracy_m: None,
                    radius_m: None,
                    purchase_prompts: false,
                },
                1,
            )
            .unwrap()
    };
    let (a, b) = (spot("Shop A"), spot("Shop B"));
    let brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme".into(),
            },
            2,
        )
        .unwrap();
    let strain = journal
        .save_strain(
            None,
            StrainData {
                name: "OG".into(),
                lineage: None,
                terpenes: vec![],
                strain_type: None,
            },
            2,
        )
        .unwrap();
    // Each shop carries its own Acme products, set by hand in "Where I buy it".
    let product = |name: &str, shop: &String| {
        journal
            .save_product(
                None,
                ProductData {
                    name: name.into(),
                    form: None,
                    brand: Some(brand.clone()),
                    strains: vec![strain.clone()],
                    thc_milli: None,
                    thc_unit: None,
                    cbd_milli: None,
                    cbd_unit: None,
                    terpenes: vec![],
                    batch: None,
                    package_date_day: None,
                    identifiers: vec![],
                    shops: vec![shop.clone()],
                },
                3,
            )
            .unwrap()
    };
    let (at_a, at_b) = (product("Pre-roll", &a), product("Cart", &b));
    for i in 0..5 {
        let mut s = session(Some(at_a.clone()));
        s.rating = Some(5);
        journal.save_session(None, s, 10 + i).unwrap();
        let mut s = session(Some(at_b.clone()));
        s.rating = Some(2);
        journal.save_session(None, s, 20 + i).unwrap();
    }
    let found = journal.shop_insights().unwrap();
    let shop_a = found.iter().find(|s| s.spot == a).unwrap();
    let effects: Vec<_> = shop_a
        .effects
        .iter()
        .map(|e| {
            (
                e.level.as_str(),
                e.label.as_str(),
                e.here_avg_milli,
                e.elsewhere_avg_milli,
            )
        })
        .collect();
    assert_eq!(
        effects,
        vec![
            ("brand", "Acme", 5_000, 2_000),
            ("strain", "OG", 5_000, 2_000)
        ]
    );
}

#[test]
fn maptiler_key_is_local_and_validated() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    assert_eq!(journal.maptiler_key().unwrap(), None);
    journal.set_maptiler_key(Some("AbC123xyz".into())).unwrap();
    assert_eq!(
        journal.maptiler_key().unwrap().as_deref(),
        Some("AbC123xyz")
    );
    for bad in ["", "has space", "dash-ed", "ünïcode", &"a".repeat(129)] {
        assert_eq!(
            journal.set_maptiler_key(Some(bad.into())),
            Err(CoreError::InvalidInput)
        );
    }
    // Local only: a journal restored from this one's sync blobs has no key.
    let other = Device::new();
    let restored = other.open(&keys).unwrap();
    for blob in journal.export_sync_blobs().unwrap() {
        restored.import_sync_blob(blob.bytes, 5).unwrap();
    }
    assert_eq!(restored.maptiler_key().unwrap(), None);
    journal.set_maptiler_key(None).unwrap();
    assert_eq!(journal.maptiler_key().unwrap(), None);
    journal.lock();
    assert_eq!(journal.maptiler_key(), Err(CoreError::Locked));
}

/// A second device on the same journal (same master key, its own database).
fn second_device(keys: &NewKeys, device: &Device) -> Arc<Journal> {
    let restored = master_key_from_recovery(
        keys.recovery_key.clone(),
        keys.master_wrapped_by_recovery.clone(),
    )
    .unwrap();
    Journal::open(
        device.dir.join("journal.db").to_string_lossy().into_owned(),
        device.dir.join("files").to_string_lossy().into_owned(),
        restored,
        wrapped_db_key_for(keys, keys),
    )
    .unwrap()
}

fn sync(from: &Journal, to: &Journal, now_ms: u64) {
    for blob in from.export_sync_blobs().unwrap() {
        assert_eq!(to.import_sync_blob(blob.bytes, now_ms).unwrap().deferred, 0);
    }
}

#[test]
fn what_i_track_choices_sync_and_merge_per_data_point() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let tablet_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    let tablet = second_device(&keys, &tablet_dir);

    let all = trackable_data_points();
    assert_eq!(all.len(), 12);
    assert_eq!(all[0], "brand");
    assert!(phone.hidden_data_points().unwrap().is_empty());
    assert_eq!(
        phone.set_data_point_tracked("vibes".into(), false, 1),
        Err(CoreError::InvalidInput)
    );

    // Each device hides something before they ever sync: both records
    // have the same derived ID, so they merge instead of competing.
    phone
        .set_data_point_tracked("brand".into(), false, 10)
        .unwrap();
    tablet
        .set_data_point_tracked("mood".into(), false, 11)
        .unwrap();
    sync(&phone, &tablet, 12);
    sync(&tablet, &phone, 13);
    for device in [&phone, &tablet] {
        assert_eq!(device.hidden_data_points().unwrap(), ["brand", "mood"]);
    }

    // Showing brand again on the phone wins, as the later edit.
    phone
        .set_data_point_tracked("brand".into(), true, 20)
        .unwrap();
    sync(&phone, &tablet, 21);
    assert_eq!(tablet.hidden_data_points().unwrap(), ["mood"]);

    // Hiding purchases hides prices and shops with them.
    tablet
        .set_data_point_tracked("purchases".into(), false, 30)
        .unwrap();
    assert_eq!(
        tablet.hidden_data_points().unwrap(),
        ["price_and_shop", "purchases", "mood"]
    );
}

#[test]
fn what_i_track_survives_master_key_rotation() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    journal
        .set_data_point_tracked("dose".into(), false, 1)
        .unwrap();
    journal
        .rotate_master_key(
            keys.recovery_key.clone(),
            keys.master_wrapped_by_recovery.clone(),
        )
        .unwrap();
    assert_eq!(journal.hidden_data_points().unwrap(), ["dose"]);
    journal
        .set_data_point_tracked("dose".into(), true, 2)
        .unwrap();
    assert!(journal.hidden_data_points().unwrap().is_empty());
}

#[test]
fn deleting_a_data_point_clears_it_everywhere() {
    let device = Device::new();
    let keys = create_keys().unwrap();
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
    let spot = journal
        .save_spot(
            None,
            SpotData {
                name: "Spot 1".into(),
                purposes: vec![builtin("re_up")],
                lat_e7: None,
                lon_e7: None,
                accuracy_m: None,
                radius_m: None,
                purchase_prompts: false,
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
                terpenes: vec![TagIntensity {
                    tag: builtin("myrcene"),
                    value: 500,
                }],
                batch: Some("B-7".into()),
                package_date_day: Some(20_300),
                identifiers: vec![],
                shops: vec![spot.clone()],
            },
            1,
        )
        .unwrap();
    let purchase = journal
        .save_purchase(
            None,
            PurchaseData {
                when: LocalTimeData {
                    unix_ms: 1_759_900_000_000,
                    utc_offset_min: -420,
                },
                spot: Some(spot.clone()),
                lines: vec![PurchaseLineData {
                    line_id: None,
                    product: Some(product.clone()),
                    batch: Some("B-7".into()),
                    price_minor_units: Some(2_000),
                    price_currency: Some("USD".into()),
                }],
            },
            2,
        )
        .unwrap();
    let line = journal
        .get_purchase(purchase.clone())
        .unwrap()
        .unwrap()
        .lines[0]
        .line_id
        .clone();
    let mut data = session(Some(product.clone()));
    data.lines[0].purchase_id = Some(purchase.clone());
    data.lines[0].purchase_line_id = line;
    data.lines[0].shop = Some(spot.clone());
    let id = journal.save_session(None, data, 3).unwrap();

    // Hiding alone keeps the values.
    journal
        .set_data_point_tracked("brand".into(), false, 4)
        .unwrap();
    assert!(
        journal
            .get_product(product.clone())
            .unwrap()
            .unwrap()
            .brand
            .is_some()
    );

    assert_eq!(
        journal.delete_data_point("nonsense".into(), 5),
        Err(CoreError::InvalidInput)
    );
    // The product loses its brand and the brand record goes.
    assert_eq!(journal.delete_data_point("brand".into(), 5).unwrap(), 2);
    assert!(
        journal
            .get_product(product.clone())
            .unwrap()
            .unwrap()
            .brand
            .is_none()
    );
    assert!(journal.get_brand(brand).unwrap().is_none());

    for point in [
        "strain",
        "potency",
        "terpenes",
        "batch",
        "dose",
        "timeline",
        "mood",
        "intention",
        "social",
        "purchases",
    ] {
        assert!(
            journal.delete_data_point(point.into(), 6).unwrap() > 0,
            "{point}"
        );
        // Nothing left to delete the second time.
        assert_eq!(
            journal.delete_data_point(point.into(), 7).unwrap(),
            0,
            "{point}"
        );
    }

    let p = journal.get_product(product).unwrap().unwrap();
    assert!(p.strains.is_empty() && p.terpenes.is_empty() && p.shops.is_empty());
    assert!(p.thc_milli.is_none() && p.batch.is_none() && p.package_date_day.is_none());
    assert_eq!(p.name, "Gummies");
    assert!(journal.get_strain(strain).unwrap().is_none());
    assert!(journal.get_purchase(purchase).unwrap().is_none());
    assert!(
        journal.get_spot(spot).unwrap().is_some(),
        "spots themselves stay"
    );

    let s = journal.get_session(id).unwrap().unwrap();
    let l = &s.lines[0];
    assert!(l.dose_milli.is_none() && l.purchase_id.is_none() && l.shop.is_none());
    assert!(s.mood_before.is_none() && s.mood_after.is_none() && s.intention.is_none());
    assert!(s.social.is_none() && s.onset_min.is_none());
    // Always tracked: untouched.
    assert_eq!((s.rating, l.method.as_str()), (Some(4), "edible"));
    assert_eq!(s.effects.len(), 1);
}

#[test]
fn unlocking_rebuilds_an_index_built_by_an_older_app() {
    let dir = Device::new();
    let keys = create_keys().unwrap();
    drop(dir.open(&keys).unwrap());

    let master = terpoir_crypto::MasterKey::from_slice(&keys.master_key).unwrap();
    let database_key = master
        .unwrap_database_key(
            &terpoir_crypto::WrappedKey::from_slice(&keys.wrapped_database_key).unwrap(),
        )
        .unwrap();
    let store = || {
        terpoir_store::Store::open(
            &dir.dir.join("journal.db"),
            &dir.dir.join("files"),
            &database_key,
        )
        .unwrap()
    };
    // As an app whose index rules were older left it.
    store()
        .set_derived_version("index", terpoir_store::INDEX_VERSION - 1)
        .unwrap();

    drop(dir.open(&keys).unwrap());
    assert_eq!(
        store().derived_version("index").unwrap(),
        Some(terpoir_store::INDEX_VERSION)
    );
}
