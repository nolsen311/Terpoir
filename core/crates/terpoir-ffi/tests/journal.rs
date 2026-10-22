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
    BrandData, CannabinoidAmountData, CoreError, ExportArchive, IdentifierData, InsightFeedback,
    Journal, LocalTimeData, NewKeys, PackageSizeData, ProductData, ProductLineData, PurchaseData,
    PurchaseLineData, RecordType, SessionData, SnippetData, SpotData, StrainCatalog, StrainData,
    SymptomData, TagData, TagIntensity, TagProvenance, TagRefData, TerpeneLevelData,
    TimelineLevelData, check_export_archive, check_in_minutes_for_methods, check_recovery_key,
    create_keys, keys_for_restore, lookup_address, lookup_template_from_example,
    lookup_template_valid, master_key_from_recovery, read_export_archive, readable_blobs,
    subtypes_for_form, sync_folder_key_file, timeline_plan_for_methods, trackable_data_points,
    vocabulary,
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
        timeline_levels: vec![],
        timeline_updated_ms: None,
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
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
                breeder: None,
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
                cannabinoids: vec![],
                subtype: None,
                default_size: None,
                receipt_names: vec![],
                archived: false,
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
                form: None,
                method: None,
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
                phones: vec![],
                licences: vec![],
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
                    size: None,
                    quantity: 1,
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
                cannabinoids: vec![],
                subtype: None,
                default_size: None,
                receipt_names: vec![],
                archived: false,
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
    let header = rows.next().unwrap();
    assert!(header.starts_with("id,date,time,"));
    assert!(header.ends_with(",symptoms,timeline"));
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
fn sessions_keep_their_timeline_levels() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let mut data = session(None);
    data.timeline_levels = vec![
        TimelineLevelData {
            minute: 20,
            level: 3,
        },
        TimelineLevelData {
            minute: 40,
            level: 5,
        },
    ];
    data.timeline_updated_ms = Some(1_760_000_000_000);
    let id = journal.save_session(None, data.clone(), 1).unwrap();
    let saved = journal.get_session(id.clone()).unwrap().unwrap();
    assert!(saved.timeline_levels == data.timeline_levels);
    assert_eq!(saved.timeline_updated_ms, data.timeline_updated_ms);

    // The plaintext export carries the set points, and only those: in the
    // JSON as one entry per minute, in sessions.csv as "minute=level".
    let files = journal.export_plaintext(10).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&files[0].bytes).unwrap();
    let levels = &json["records"]["sessions"][0]["fields"]["timeline_levels"];
    assert_eq!(levels["20"], 3);
    assert_eq!(levels["40"], 5);
    assert_eq!(levels.as_object().unwrap().len(), 2);
    let sessions_csv = String::from_utf8(files[1].bytes.clone()).unwrap();
    let row = sessions_csv.split("\r\n").nth(1).unwrap();
    assert!(row.ends_with(",20=3; 40=5"));

    // A level above 5, or a minute twice, is refused.
    let mut bad = data.clone();
    bad.timeline_levels[1].level = 6;
    assert!(matches!(
        journal.save_session(Some(id.clone()), bad, 2),
        Err(CoreError::InvalidInput)
    ));
    let mut twice = data;
    twice.timeline_levels[1].minute = 20;
    assert!(matches!(
        journal.save_session(Some(id), twice, 3),
        Err(CoreError::InvalidInput)
    ));
}

#[test]
fn timeline_plans_come_from_the_methods() {
    let dab = timeline_plan_for_methods(vec!["dabbed".into()]);
    assert_eq!(dab.point_minutes, [20, 40, 60]);
    assert_eq!(dab.gesture_points, 1);
    let mixed = timeline_plan_for_methods(vec!["smoked".into(), "tincture".into()]);
    assert_eq!(mixed.minutes, 240);
    assert_eq!(mixed.point_minutes.len(), 5);
    assert_eq!(timeline_plan_for_methods(vec![]).minutes, 120);
    // A patch: 12 hours, unevenly spaced points; and the longest plan wins.
    let patch = timeline_plan_for_methods(vec!["transdermal".into(), "edible".into()]);
    assert_eq!(patch.minutes, 720);
    assert_eq!(patch.point_minutes, [60, 120, 240, 480, 720]);
    assert_eq!(patch.gesture_points, 2);
}

#[test]
fn check_ins_come_from_the_methods() {
    let minutes = |methods: &[&str]| {
        check_in_minutes_for_methods(methods.iter().map(|m| (*m).to_owned()).collect())
    };
    assert_eq!(minutes(&["edible"]), Some(90));
    assert_eq!(minutes(&["beverage"]), Some(90));
    assert_eq!(minutes(&["tincture"]), Some(45));
    assert_eq!(minutes(&["transdermal"]), Some(120));
    // The slowest method wins.
    assert_eq!(minutes(&["tincture", "edible", "smoked"]), Some(90));
    assert_eq!(minutes(&["edible", "transdermal"]), Some(120));
    // Inhaled and topical get none, nor does a method from a newer app.
    assert_eq!(
        minutes(&["smoked", "vaped_flower", "vape_cart", "dabbed", "topical"]),
        None
    );
    assert_eq!(minutes(&["nebulized"]), None);
    assert_eq!(minutes(&[]), None);
    assert!(vocabulary().methods.contains(&"transdermal".to_owned()));
}

#[test]
fn products_take_their_usual_method_from_subtype_then_form() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let builtin = |slug: &str| Some(TagRefData::Builtin { slug: slug.into() });
    let usual = |form: Option<&str>, subtype: Option<TagRefData>| {
        journal
            .usual_method(form.map(str::to_owned), subtype)
            .unwrap()
    };
    // A patch is transdermal; a balm, like any topical, topical.
    assert_eq!(
        usual(Some("topical"), builtin("patch")).as_deref(),
        Some("transdermal")
    );
    assert_eq!(
        usual(Some("topical"), builtin("balm")).as_deref(),
        Some("topical")
    );
    assert_eq!(usual(Some("topical"), None).as_deref(), Some("topical"));
    assert_eq!(usual(Some("concentrate"), None).as_deref(), Some("dabbed"));
    // A subtype under another form: the form's method.
    assert_eq!(
        usual(Some("edible"), builtin("patch")).as_deref(),
        Some("edible")
    );
    assert_eq!(usual(None, None), None);
    assert!(subtypes_for_form("topical".into()).contains(&"patch".to_owned()));

    // A custom subtype with its own method, and one keeping its form's.
    let strip = journal
        .save_tag(
            None,
            TagData {
                name: "Strip".into(),
                category: "subtype".into(),
                form: Some("edible".into()),
                method: Some("tincture".into()),
            },
            1,
        )
        .unwrap();
    let saved = journal.get_tag(strip.clone()).unwrap().unwrap();
    assert_eq!(
        (saved.form.as_deref(), saved.method.as_deref()),
        (Some("edible"), Some("tincture"))
    );
    let brownie = journal
        .save_tag(
            None,
            TagData {
                name: "Brownie bite".into(),
                category: "subtype".into(),
                form: Some("edible".into()),
                method: None,
            },
            2,
        )
        .unwrap();
    let custom = |id: &String| Some(TagRefData::Custom { id: id.clone() });
    assert_eq!(
        usual(Some("edible"), custom(&strip)).as_deref(),
        Some("tincture")
    );
    assert_eq!(
        usual(Some("edible"), custom(&brownie)).as_deref(),
        Some("edible")
    );
    // Under a form that isn't its parent: the form's method.
    assert_eq!(
        usual(Some("beverage"), custom(&strip)).as_deref(),
        Some("beverage")
    );
    // A deleted custom subtype counts as none.
    assert!(journal.delete_record(strip.clone()).unwrap());
    assert_eq!(
        usual(Some("edible"), custom(&strip)).as_deref(),
        Some("edible")
    );
    assert!(matches!(
        journal.usual_method(None, builtin("")),
        Err(CoreError::InvalidInput)
    ));

    // A product keeps its subtype; a custom one is indexed as its tag.
    let mut product = strain_product("Relief patch", String::new(), vec![]);
    product.strains = vec![];
    product.form = Some("topical".into());
    product.subtype = builtin("patch");
    let id = journal.save_product(None, product.clone(), 3).unwrap();
    let saved = journal.get_product(id.clone()).unwrap().unwrap();
    assert!(saved.subtype == product.subtype);
    product.form = Some("edible".into());
    product.subtype = custom(&brownie);
    journal.save_product(Some(id.clone()), product, 4).unwrap();
    assert_eq!(
        journal.referencing(brownie.clone()).unwrap(),
        vec![id.clone()]
    );
    assert_eq!(
        journal
            .with_tag(TagRefData::Custom { id: brownie })
            .unwrap(),
        vec![id]
    );
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
                form: None,
                method: None,
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
                form: None,
                method: None,
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
        cannabinoids: vec![],
        subtype: None,
        default_size: None,
        receipt_names: vec![],
        archived: false,
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
                    cannabinoids: vec![],
                    subtype: None,
                    default_size: None,
                    receipt_names: vec![],
                    archived: false,
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
                    phones: vec![],
                    licences: vec![],
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
                    cannabinoids: vec![],
                    subtype: None,
                    default_size: None,
                    receipt_names: vec![],
                    archived: false,
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
                        size: None,
                        quantity: 1,
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
                    phones: vec![],
                    licences: vec![],
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
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
                breeder: None,
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
                    cannabinoids: vec![],
                    subtype: None,
                    default_size: None,
                    receipt_names: vec![],
                    archived: false,
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
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
                breeder: None,
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
                phones: vec![],
                licences: vec![],
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
                cannabinoids: vec![],
                subtype: None,
                default_size: None,
                receipt_names: vec![],
                archived: false,
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
                    size: None,
                    quantity: 1,
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

#[test]
fn a_sync_folder_restores_on_a_new_device() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    let id = phone.save_session(None, session(None), 10).unwrap();

    // What the folder holds: the key file, the phone's blobs, a blob still
    // being written, and one from another journal.
    assert_eq!(
        sync_folder_key_file(b"not a key".to_vec()).err(),
        Some(CoreError::InvalidInput)
    );
    let key_file = sync_folder_key_file(keys.master_wrapped_by_recovery.clone()).unwrap();
    let own: Vec<Vec<u8>> = phone
        .export_sync_blobs()
        .unwrap()
        .into_iter()
        .map(|b| b.bytes)
        .collect();
    let half = own[0][..own[0].len() / 2].to_vec();
    let other_keys = create_keys().unwrap();
    let other_dir = Device::new();
    let other = other_dir.open(&other_keys).unwrap();
    other.save_session(None, session(None), 10).unwrap();
    let foreign = other.export_sync_blobs().unwrap().remove(0).bytes;

    // The key file reads as an export with no blobs.
    let parsed = read_export_archive(key_file).unwrap();
    assert!(parsed.blobs.is_empty());
    let folder = ExportArchive {
        master_wrapped_by_recovery: parsed.master_wrapped_by_recovery,
        blobs: [own.clone(), vec![half, foreign]].concat(),
    };

    // Only this journal's recovery key opens it.
    let wrong = ExportArchive {
        master_wrapped_by_recovery: folder.master_wrapped_by_recovery.clone(),
        blobs: Vec::new(),
    };
    assert!(readable_blobs(other_keys.recovery_key.clone(), wrong).is_err());

    // Unreadable blobs are left out; what remains restores like an export.
    let usable = readable_blobs(keys.recovery_key.clone(), folder).unwrap();
    assert_eq!(usable.blobs, own);
    let check = check_export_archive(
        keys.recovery_key.clone(),
        ExportArchive {
            master_wrapped_by_recovery: usable.master_wrapped_by_recovery.clone(),
            blobs: usable.blobs.clone(),
        },
    )
    .unwrap();
    assert_eq!((check.records, check.newer_format_blobs), (1, 0));
    let restored =
        keys_for_restore(keys.recovery_key.clone(), usable.master_wrapped_by_recovery).unwrap();
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
    for blob in usable.blobs {
        new_phone.import_sync_blob(blob, 20).unwrap();
    }
    assert!(new_phone.get_session(id.clone()).unwrap() == phone.get_session(id).unwrap());
}

#[test]
fn strains_keep_levels_parents_and_cannabinoids() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let parent = journal
        .save_strain(
            None,
            StrainData {
                name: "Haze".into(),
                lineage: None,
                terpenes: vec![],
                strain_type: None,
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
                breeder: None,
            },
            1,
        )
        .unwrap();
    let saved = StrainData {
        name: "Jack Herer".into(),
        lineage: Some("Haze × Northern Lights #5 × Shiva Skunk".into()),
        terpenes: vec![builtin("alpha_pinene"), builtin("myrcene")],
        strain_type: Some("sativa".into()),
        terpene_levels: vec![TerpeneLevelData {
            terpene: builtin("alpha_pinene"),
            level: "dominant".into(),
        }],
        parents: vec![parent.clone()],
        cannabinoids: vec!["cbg".into()],
        breeder: None,
    };
    let mut with_stray = saved.clone();
    // A level for a terpene the strain doesn't list is dropped.
    with_stray.terpene_levels.push(TerpeneLevelData {
        terpene: builtin("limonene"),
        level: "trace".into(),
    });
    let id = journal.save_strain(None, with_stray, 2).unwrap();
    // Records have no Debug, so nothing private can reach a log.
    assert!(journal.get_strain(id).unwrap() == Some(saved));

    // The detail step: Terpenes until chosen, then synced like any choice.
    assert_eq!(journal.strain_detail().unwrap(), "terpenes");
    assert_eq!(
        vocabulary().strain_details,
        ["simple", "terpenes", "detailed"]
    );
    journal.set_strain_detail("detailed".into(), 3).unwrap();
    assert_eq!(journal.strain_detail().unwrap(), "detailed");
    assert!(matches!(
        journal.set_strain_detail("lab_grade".into(), 4),
        Err(CoreError::InvalidInput)
    ));
    // Hiding data points leaves the step alone, and the step leaves them.
    journal
        .set_data_point_tracked("brand".into(), false, 5)
        .unwrap();
    assert_eq!(journal.strain_detail().unwrap(), "detailed");
    assert_eq!(journal.hidden_data_points().unwrap(), ["brand"]);
}

#[test]
fn insights_fall_back_to_a_strains_dominant_terpene() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let strain = |name: &str, terpene: &str| StrainData {
        name: name.into(),
        lineage: None,
        terpenes: vec![builtin(terpene)],
        strain_type: None,
        terpene_levels: vec![TerpeneLevelData {
            terpene: builtin(terpene),
            level: "dominant".into(),
        }],
        parents: vec![],
        cannabinoids: vec![],
        breeder: None,
    };
    let product = |name: &str, strain: String, terpenes: Vec<TagIntensity>| ProductData {
        name: name.into(),
        form: Some("flower".into()),
        brand: None,
        strains: vec![strain],
        thc_milli: None,
        thc_unit: None,
        cbd_milli: None,
        cbd_unit: None,
        terpenes,
        batch: None,
        package_date_day: None,
        identifiers: vec![],
        shops: vec![],
        cannabinoids: vec![],
        subtype: None,
        default_size: None,
        receipt_names: vec![],
        archived: false,
    };
    let sleepy = journal
        .save_strain(None, strain("Sleepy", "myrcene"), 1)
        .unwrap();
    let zesty = journal
        .save_strain(None, strain("Zesty", "limonene"), 2)
        .unwrap();
    // No terpenes on this label: its strain's dominant myrcene stands in.
    let heavy = journal
        .save_product(None, product("Heavy", sleepy, vec![]), 3)
        .unwrap();
    // A label's terpenes win over its strain's profile.
    let bright = journal
        .save_product(
            None,
            product(
                "Bright",
                zesty,
                vec![TagIntensity {
                    tag: builtin("limonene"),
                    value: 800,
                }],
            ),
            4,
        )
        .unwrap();
    for (product, rating) in [(&heavy, 2), (&bright, 4)] {
        for i in 0..5 {
            let mut s = session(Some(product.clone()));
            s.rating = Some(rating);
            journal.save_session(None, s, 10 + i).unwrap();
        }
    }
    let found = journal.insights().unwrap();
    let myrcene = found.iter().find(|i| i.key == "terpene:myrcene").unwrap();
    assert_eq!(myrcene.from_strain_profiles, 5);
    let limonene = found.iter().find(|i| i.key == "terpene:limonene").unwrap();
    assert_eq!(limonene.from_strain_profiles, 0);

    // At the Simple step strain terpenes aren't shown, so they aren't used.
    journal.set_strain_detail("simple".into(), 20).unwrap();
    let simple = journal.insights().unwrap();
    assert!(simple.iter().all(|i| i.key != "terpene:myrcene"));
}

/// A strain with these parents and, if given, a dominant terpene.
fn family_strain(name: &str, parents: Vec<String>, terpene: Option<&str>) -> StrainData {
    StrainData {
        name: name.into(),
        lineage: None,
        terpenes: terpene.map(builtin).into_iter().collect(),
        strain_type: None,
        terpene_levels: terpene
            .map(|t| TerpeneLevelData {
                terpene: builtin(t),
                level: "dominant".into(),
            })
            .into_iter()
            .collect(),
        parents,
        cannabinoids: vec![],
        breeder: None,
    }
}

/// A product of one strain, bought at these shops.
fn strain_product(name: &str, strain: String, shops: Vec<String>) -> ProductData {
    ProductData {
        name: name.into(),
        form: Some("flower".into()),
        brand: None,
        strains: vec![strain],
        thc_milli: None,
        thc_unit: None,
        cbd_milli: None,
        cbd_unit: None,
        terpenes: vec![],
        batch: None,
        package_date_day: None,
        identifiers: vec![],
        shops,
        cannabinoids: vec![],
        subtype: None,
        default_size: None,
        receipt_names: vec![],
        archived: false,
    }
}

#[test]
fn insights_group_strains_by_shared_parent() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let blueberry = journal
        .save_strain(None, family_strain("Blueberry", vec![], None), 1)
        .unwrap();
    let dream = journal
        .save_strain(
            None,
            family_strain("Blue Dream", vec![blueberry.clone()], None),
            2,
        )
        .unwrap();
    let muffin = journal
        .save_strain(
            None,
            family_strain("Blueberry Muffin", vec![blueberry.clone()], None),
            3,
        )
        .unwrap();
    let haze = journal
        .save_strain(None, family_strain("Haze", vec![], None), 4)
        .unwrap();
    let mut t = 10;
    for (strain, sessions, rating) in [(&dream, 3, 5), (&muffin, 2, 5), (&haze, 5, 2)] {
        let product = journal
            .save_product(None, strain_product("Flower", strain.clone(), vec![]), t)
            .unwrap();
        for _ in 0..sessions {
            t += 1;
            let mut s = session(Some(product.clone()));
            s.rating = Some(rating);
            journal.save_session(None, s, t).unwrap();
        }
    }
    let key = format!("lineage:{blueberry}");
    // Parents show only at the Detailed step, so the family only then.
    assert!(journal.insights().unwrap().iter().all(|i| i.key != key));
    journal.set_strain_detail("detailed".into(), 50).unwrap();
    let found = journal.insights().unwrap();
    // Two strains with a shared parent, five sessions between them.
    let family = found.iter().find(|i| i.key == key).unwrap();
    assert_eq!(family.category, "lineage");
    assert_eq!(family.subject, "Blueberry");
    assert_eq!(family.with_sessions.len(), 5);
    assert_eq!(family.with_avg_milli, 5_000);
    // A strain that is no one's parent is no family.
    assert!(found.iter().all(|i| i.key != format!("lineage:{haze}")));
}

#[test]
fn the_shop_effect_compares_strain_families_and_terpenes() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
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
                    phones: vec![],
                    licences: vec![],
                },
                1,
            )
            .unwrap()
    };
    let (a, b) = (spot("Shop A"), spot("Shop B"));
    let blueberry = journal
        .save_strain(None, family_strain("Blueberry", vec![], None), 2)
        .unwrap();
    // A different child of Blueberry at each shop, both myrcene-dominant:
    // no strain is bought at both, but the family and the terpene are.
    let dream = journal
        .save_strain(
            None,
            family_strain("Blue Dream", vec![blueberry.clone()], Some("myrcene")),
            3,
        )
        .unwrap();
    let muffin = journal
        .save_strain(
            None,
            family_strain("Blueberry Muffin", vec![blueberry], Some("myrcene")),
            4,
        )
        .unwrap();
    let at_a = journal
        .save_product(None, strain_product("Dream", dream, vec![a.clone()]), 5)
        .unwrap();
    let at_b = journal
        .save_product(None, strain_product("Muffin", muffin, vec![b]), 6)
        .unwrap();
    journal.set_strain_detail("detailed".into(), 7).unwrap();
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
            ("lineage", "Blueberry", 5_000, 2_000),
            ("terpene", "myrcene", 5_000, 2_000)
        ]
    );

    // Below Detailed there are no parents to group by.
    journal.set_strain_detail("terpenes".into(), 30).unwrap();
    let found = journal.shop_insights().unwrap();
    let shop_a = found.iter().find(|s| s.spot == a).unwrap();
    assert!(shop_a.effects.iter().all(|e| e.level != "lineage"));
}

#[test]
fn products_keep_label_cannabinoids() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let amount = |c: &str, milli: i64| CannabinoidAmountData {
        cannabinoid: c.into(),
        milli,
        unit: "percent".into(),
    };
    let product = ProductData {
        name: "Sleepy Flower".into(),
        form: Some("flower".into()),
        brand: None,
        strains: vec![],
        thc_milli: Some(18_000),
        thc_unit: Some("percent".into()),
        cbd_milli: None,
        cbd_unit: None,
        terpenes: vec![],
        batch: None,
        package_date_day: None,
        identifiers: vec![],
        shops: vec![],
        cannabinoids: vec![amount("cbg", 1_200), amount("cbn", 400)],
        subtype: None,
        default_size: None,
        receipt_names: vec![],
        archived: false,
    };
    let id = journal.save_product(None, product.clone(), 1).unwrap();
    // Read back in cannabinoid order (cbn before cbg is not guaranteed by
    // input order), so compare as sets.
    let saved = journal.get_product(id.clone()).unwrap().unwrap();
    let mut got: Vec<_> = saved
        .cannabinoids
        .iter()
        .map(|c| (c.cannabinoid.clone(), c.milli))
        .collect();
    got.sort();
    assert_eq!(got, [("cbg".to_owned(), 1_200), ("cbn".to_owned(), 400)]);

    // Two amounts for one cannabinoid are refused.
    let mut twice = product.clone();
    twice.cannabinoids.push(amount("cbg", 900));
    assert!(matches!(
        journal.save_product(None, twice, 2),
        Err(CoreError::InvalidInput)
    ));

    // Removing one keeps the other.
    let mut fewer = saved.clone();
    fewer.cannabinoids.retain(|c| c.cannabinoid == "cbn");
    journal.save_product(Some(id.clone()), fewer, 3).unwrap();
    let after = journal.get_product(id).unwrap().unwrap();
    assert_eq!(after.cannabinoids.len(), 1);
    assert_eq!(after.cannabinoids[0].cannabinoid, "cbn");
}

#[test]
fn receipt_items_and_shops_are_matched_against_the_journal() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let strain = |name: &str| {
        journal
            .save_strain(
                None,
                StrainData {
                    name: name.into(),
                    lineage: None,
                    terpenes: vec![],
                    strain_type: None,
                    terpene_levels: vec![],
                    parents: vec![],
                    cannabinoids: vec![],
                    breeder: None,
                },
                1,
            )
            .unwrap()
    };
    let blue_dream = strain("Blue Dream");
    let bubba_diesel = strain("Bubba Diesel");
    let gsc = strain("Girl Scout Cookies");
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
                    phones: vec![],
                    licences: vec![],
                },
                2,
            )
            .unwrap()
    };
    let green_leaf = spot("Green Leaf");
    let elsewhere = spot("Elsewhere");
    let acme = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme".into(),
            },
            3,
        )
        .unwrap();
    let product = journal
        .save_product(
            None,
            ProductData {
                name: "Acme Blue Dream".into(),
                form: Some("flower".into()),
                brand: Some(acme.clone()),
                strains: vec![blue_dream.clone()],
                thc_milli: None,
                thc_unit: None,
                cbd_milli: None,
                cbd_unit: None,
                terpenes: vec![],
                batch: None,
                package_date_day: None,
                identifiers: vec![],
                shops: vec![],
                cannabinoids: vec![],
                subtype: None,
                default_size: None,
                receipt_names: vec![],
                archived: false,
            },
            4,
        )
        .unwrap();
    journal
        .save_purchase(
            None,
            PurchaseData {
                when: LocalTimeData {
                    unix_ms: 1_759_900_000_000,
                    utc_offset_min: -420,
                },
                spot: Some(green_leaf.clone()),
                lines: vec![PurchaseLineData {
                    line_id: None,
                    product: Some(product.clone()),
                    batch: None,
                    price_minor_units: Some(3_500),
                    price_currency: Some("USD".into()),
                    size: None,
                    quantity: 1,
                }],
            },
            5,
        )
        .unwrap();

    // BD at the shop where the user buys Blue Dream: Blue Dream first,
    // Bubba Diesel still offered as a close choice. Never automatic.
    let found = journal
        .suggest_for_receipt_item("BD 3.5G HYB".into(), Some(green_leaf.clone()))
        .unwrap();
    let got: Vec<(&str, &str, &str, bool, bool)> = found
        .iter()
        .map(|s| {
            (
                s.id.as_str(),
                s.record_type.as_str(),
                s.match_kind.as_str(),
                s.at_shop,
                s.close,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (blue_dream.as_str(), "strain", "initials", true, true),
            (bubba_diesel.as_str(), "strain", "initials", false, true),
        ]
    );
    // At another shop, both are equal choices.
    let found = journal
        .suggest_for_receipt_item("BD PR 1G".into(), Some(elsewhere))
        .unwrap();
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|s| s.close && !s.at_shop && s.score == 50));
    // Initials of a three-word name; the brand and its product by name.
    let found = journal
        .suggest_for_receipt_item("GSC 1/8 IND".into(), None)
        .unwrap();
    assert!(found.len() == 1 && found[0].id == gsc);
    let found = journal
        .suggest_for_receipt_item("ACME BLUE DREAM 3.5G 35.00".into(), None)
        .unwrap();
    assert!(found[0].id == product && found[0].match_kind == "exact");
    assert!(
        found
            .iter()
            .any(|s| s.id == acme && s.record_type == "brand")
    );
    assert!(matches!(
        journal.suggest_for_receipt_item("BD".into(), Some("nope".into())),
        Err(CoreError::InvalidInput)
    ));

    // The shop from the header: by name only, so a guess.
    let header = vec![
        "GREEN LEAF".to_owned(),
        "Tel (555) 010-2030".to_owned(),
        "BD 3.5G HYB 35.00".to_owned(),
    ];
    let shop = journal.match_receipt_shop(header).unwrap();
    assert_eq!(shop.spot, Some(green_leaf));
    assert_eq!(shop.kind.as_deref(), Some("name"));
    assert!(!shop.strong);
    assert_eq!((shop.row, shop.shop_row), (Some(0), Some(0)));
    assert_eq!(shop.phones, ["5550102030"]);
    let none = journal
        .match_receipt_shop(vec!["SOMEWHERE NEW".into()])
        .unwrap();
    assert!(none.spot.is_none() && none.kind.is_none());
    assert_eq!(none.shop.as_deref(), Some("SOMEWHERE NEW"));
    journal.lock();
    assert!(matches!(
        journal.match_receipt_shop(vec![]),
        Err(CoreError::Locked)
    ));
}

fn plain_product(name: &str) -> ProductData {
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
        package_date_day: None,
        identifiers: vec![],
        shops: vec![],
        cannabinoids: vec![],
        subtype: None,
        default_size: None,
        receipt_names: vec![],
        archived: false,
    }
}

fn plain_spot(name: &str) -> SpotData {
    SpotData {
        name: name.into(),
        purposes: vec![builtin("re_up")],
        lat_e7: None,
        lon_e7: None,
        accuracy_m: None,
        radius_m: None,
        purchase_prompts: false,
        phones: vec![],
        licences: vec![],
    }
}

fn grams(milli: i64) -> PackageSizeData {
    PackageSizeData {
        milli: Some(milli),
        unit: Some("g".into()),
        pack: None,
        per: None,
    }
}

#[test]
fn purchase_lines_keep_size_and_quantity_and_products_a_default_size() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let mut data = plain_product("Blue Dream");
    data.default_size = Some(grams(3_500));
    let product = journal.save_product(None, data, 1).unwrap();
    let saved = journal.get_product(product.clone()).unwrap().unwrap();
    assert!(saved.default_size == Some(grams(3_500)));

    // A scanned line's breakdown maps straight onto the purchase line.
    let item = terpoir_ffi::describe_receipt_item("2 X BD PR 5PK 0.5G".into());
    let mut size = PackageSizeData {
        milli: item.size_milli,
        unit: item.size_unit.clone(),
        pack: item.pack,
        per: None,
    };
    // Five half-gram pre-rolls: each piece's, and the reading is kept.
    size.per = terpoir_ffi::package_amount_per(size.clone(), item.form.clone());
    assert_eq!(size.per.as_deref(), Some("each"));
    let line = |size: Option<PackageSizeData>, quantity: u32| PurchaseLineData {
        line_id: None,
        product: Some(product.clone()),
        batch: None,
        price_minor_units: None,
        price_currency: None,
        size,
        quantity,
    };
    let purchase = |lines| PurchaseData {
        when: LocalTimeData {
            unix_ms: 1_759_900_000_000,
            utc_offset_min: -420,
        },
        spot: None,
        lines,
    };
    let id = journal
        .save_purchase(
            None,
            purchase(vec![
                line(Some(size.clone()), item.quantity.unwrap()),
                line(None, 1),
            ]),
            2,
        )
        .unwrap();
    let saved = journal.get_purchase(id.clone()).unwrap().unwrap();
    let mut got: Vec<(Option<PackageSizeData>, u32)> = saved
        .lines
        .iter()
        .map(|l| (l.size.clone(), l.quantity))
        .collect();
    got.sort_by_key(|(_, q)| *q);
    assert!(got == [(None, 1), (Some(size.clone()), 2)]);
    assert!(size.milli == Some(500) && size.unit.as_deref() == Some("g") && size.pack == Some(5));

    // The CSVs carry them.
    let files = journal.export_plaintext(10).unwrap();
    let products_csv = String::from_utf8(files[2].bytes.clone()).unwrap();
    assert!(products_csv.lines().next().unwrap().ends_with(
        ",shops,default_size,default_size_unit,default_pack,default_size_per,receipt_names,archived"
    ));
    assert!(products_csv.contains(",3.5,g,,,,"));
    let purchases_csv = String::from_utf8(files[3].bytes.clone()).unwrap();
    assert!(
        purchases_csv
            .lines()
            .next()
            .unwrap()
            .ends_with(",currency,size,size_unit,pack,size_per,quantity")
    );
    assert!(purchases_csv.contains(",0.5,g,5,each,2\r\n"));
    assert!(purchases_csv.contains(",,,,,1\r\n"));

    // Malformed sizes and quantities are refused.
    for (size, quantity) in [
        (None, 0),
        (
            Some(PackageSizeData {
                milli: Some(1_000),
                unit: None,
                pack: None,
                per: None,
            }),
            1,
        ),
        (
            Some(PackageSizeData {
                milli: None,
                unit: None,
                pack: None,
                per: None,
            }),
            1,
        ),
        (Some(grams(0)), 1),
        (
            Some(PackageSizeData {
                milli: None,
                unit: None,
                pack: Some(0),
                per: None,
            }),
            1,
        ),
        // What the amount covers, without both an amount and a pack.
        (
            Some(PackageSizeData {
                per: Some("each".into()),
                ..grams(500)
            }),
            1,
        ),
        (
            Some(PackageSizeData {
                pack: Some(5),
                per: Some(String::new()),
                ..grams(500)
            }),
            1,
        ),
    ] {
        assert_eq!(
            journal.save_purchase(None, purchase(vec![line(size, quantity)]), 3),
            Err(CoreError::InvalidInput)
        );
    }
    let mut bad = plain_product("x");
    bad.default_size = Some(grams(-1));
    assert_eq!(
        journal.save_product(None, bad, 3),
        Err(CoreError::InvalidInput)
    );
}

#[test]
fn learned_receipt_names_link_exactly_and_automatically() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let tablet_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    let blue_dream = phone
        .save_product(None, plain_product("Blue Dream"), 1)
        .unwrap();
    let bubba = phone
        .save_product(None, plain_product("Bubba Diesel"), 1)
        .unwrap();

    // Before learning, BD is only a suggestion.
    let found = phone
        .suggest_for_receipt_item("BD 3.5G HYB".into(), None)
        .unwrap();
    assert!(!found.is_empty() && found.iter().all(|s| !s.learned));

    // The user links the line to Blue Dream: the exact text is kept.
    phone
        .learn_receipt_name(blue_dream.clone(), "bd  3.5g hyb ".into(), 2)
        .unwrap();
    let saved = phone.get_product(blue_dream.clone()).unwrap().unwrap();
    assert_eq!(saved.receipt_names, ["BD 3.5G HYB"]);
    let found = phone
        .suggest_for_receipt_item("BD 3.5G  HYB".into(), None)
        .unwrap();
    assert_eq!(found[0].id, blue_dream);
    assert!(found[0].learned && !found[0].close);
    assert_eq!(found[0].match_kind, "learned");
    assert_eq!(found[0].record_type, "product");
    // Listed once, ahead of the ranked suggestions.
    assert_eq!(found.iter().filter(|s| s.id == blue_dream).count(), 1);
    assert!(found[1..].iter().all(|s| !s.learned));
    // A line that differs by anything but case and spacing (here a
    // recognition confusion) is only a suggestion.
    let found = phone
        .suggest_for_receipt_item("BD 3.5C HYB".into(), None)
        .unwrap();
    assert!(found.iter().all(|s| !s.learned));

    // It syncs with the product.
    let tablet = second_device(&keys, &tablet_dir);
    sync(&phone, &tablet, 3);
    let found = tablet
        .suggest_for_receipt_item("BD 3.5G HYB".into(), None)
        .unwrap();
    assert!(found[0].learned && found[0].id == blue_dream);

    // Linking the same line elsewhere moves it.
    phone
        .learn_receipt_name(bubba.clone(), "BD 3.5G HYB".into(), 4)
        .unwrap();
    assert!(
        phone
            .get_product(blue_dream.clone())
            .unwrap()
            .unwrap()
            .receipt_names
            .is_empty()
    );
    let found = phone
        .suggest_for_receipt_item("BD 3.5G HYB".into(), None)
        .unwrap();
    assert!(found[0].learned && found[0].id == bubba);
    // Two products that learned one line on different devices are offered
    // as equal choices, not linked.
    tablet
        .learn_receipt_name(blue_dream.clone(), "GSC 1/8".into(), 5)
        .unwrap();
    phone
        .learn_receipt_name(bubba.clone(), "GSC 1/8".into(), 5)
        .unwrap();
    sync(&tablet, &phone, 6);
    let found = phone
        .suggest_for_receipt_item("GSC 1/8".into(), None)
        .unwrap();
    assert!(found[..2].iter().all(|s| s.learned && s.close));

    // Undo forgets it.
    assert!(
        phone
            .forget_receipt_name(bubba.clone(), "bd 3.5g hyb".into(), 7)
            .unwrap()
    );
    assert!(
        !phone
            .forget_receipt_name(bubba.clone(), "bd 3.5g hyb".into(), 7)
            .unwrap()
    );
    let found = phone
        .suggest_for_receipt_item("BD 3.5G HYB".into(), None)
        .unwrap();
    assert!(found.iter().all(|s| !s.learned));

    // Learned names are part of the product (and the editor keeps them),
    // and an empty text is refused.
    assert_eq!(
        phone.learn_receipt_name(bubba.clone(), "  ".into(), 8),
        Err(CoreError::InvalidInput)
    );
    assert_eq!(
        phone.learn_receipt_name("00".repeat(16), "X".into(), 8),
        Err(CoreError::NotFound)
    );
    let mut data = plain_product("Bubba Diesel");
    data.receipt_names = vec!["bubba  1g".into()];
    phone.save_product(Some(bubba.clone()), data, 9).unwrap();
    assert_eq!(
        phone.get_product(bubba).unwrap().unwrap().receipt_names,
        ["BUBBA 1G"]
    );
    let mut bad = plain_product("x");
    bad.receipt_names = vec![" ".into()];
    assert_eq!(
        phone.save_product(None, bad, 10),
        Err(CoreError::InvalidInput)
    );
}

#[test]
fn a_confirmed_shop_is_recognised_strongly_by_its_numbers() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let green_leaf = journal
        .save_spot(None, plain_spot("Green Leaf"), 1)
        .unwrap();
    let header = vec![
        "GREEN LEAF DISPENSARY".to_owned(),
        "Tel (555) 010-2030".to_owned(),
        "Lic# C10-0000123-LIC".to_owned(),
        "BD 3.5G HYB 35.00".to_owned(),
    ];
    // By name only: a guess.
    let shop = journal.match_receipt_shop(header.clone()).unwrap();
    assert!(shop.spot.as_ref() == Some(&green_leaf) && !shop.strong);

    // Confirmed once: its numbers are kept.
    assert_eq!(
        journal
            .remember_receipt_shop(green_leaf.clone(), header.clone(), 2)
            .unwrap(),
        2
    );
    assert_eq!(
        journal
            .remember_receipt_shop(green_leaf.clone(), header.clone(), 3)
            .unwrap(),
        0
    );
    let saved = journal.get_spot(green_leaf.clone()).unwrap().unwrap();
    assert_eq!(saved.phones, ["5550102030"]);
    assert_eq!(saved.licences, ["C100000123LIC"]);

    // Its next receipt, under another name, is recognised by phone.
    let shop = journal
        .match_receipt_shop(vec![
            "GL CANNABIS CO".into(),
            "+1 555 010 2030".into(),
            "BD 3.5G HYB 35.00".into(),
        ])
        .unwrap();
    assert!(shop.spot.as_ref() == Some(&green_leaf) && shop.strong);
    assert_eq!(shop.kind.as_deref(), Some("phone"));
    assert_eq!(shop.row, Some(1));
    // Or by licence.
    let shop = journal
        .match_receipt_shop(vec!["SOMEWHERE".into(), "License: C10-0000123-LIC".into()])
        .unwrap();
    assert!(shop.strong && shop.kind.as_deref() == Some("licence"));

    // Typed numbers are normalised as receipts read them; others refused.
    let mut data = plain_spot("Corner");
    data.phones = vec!["(555) 999-0000".into(), "1-555-999-0000".into()];
    data.licences = vec!["ocm-aucp-23-000123".into()];
    let corner = journal.save_spot(None, data, 4).unwrap();
    let saved = journal.get_spot(corner).unwrap().unwrap();
    assert_eq!(saved.phones, ["5559990000"]);
    assert_eq!(saved.licences, ["OCMAUCP23000123"]);
    for (phones, licences) in [
        (vec!["123".to_owned()], vec![]),
        (vec!["555 CALL NOW".to_owned()], vec![]),
        (vec![], vec!["A-1".to_owned()]),
    ] {
        let mut data = plain_spot("Bad");
        data.phones = phones;
        data.licences = licences;
        assert_eq!(
            journal.save_spot(None, data, 5),
            Err(CoreError::InvalidInput)
        );
    }
    assert_eq!(
        journal.remember_receipt_shop("00".repeat(16), header, 6),
        Err(CoreError::NotFound)
    );
}

#[test]
fn timeline_insights_show_how_it_usually_goes() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let product = journal
        .save_product(
            None,
            ProductData {
                name: "Gummies".into(),
                form: Some("edible".into()),
                brand: None,
                strains: vec![],
                thc_milli: None,
                thc_unit: None,
                cbd_milli: None,
                cbd_unit: None,
                terpenes: vec![],
                batch: None,
                package_date_day: None,
                identifiers: vec![],
                shops: vec![],
                cannabinoids: vec![],
                subtype: None,
                default_size: None,
                receipt_names: vec![],
                archived: false,
            },
            1,
        )
        .unwrap();
    let points = |p: &[(u32, u8)]| {
        p.iter()
            .map(|&(minute, level)| TimelineLevelData { minute, level })
            .collect::<Vec<_>>()
    };
    let shapes: [&[(u32, u8)]; 3] = [
        &[(48, 1), (96, 3), (144, 4), (240, 1)],
        &[(48, 2), (96, 4), (192, 1)],
        &[(48, 1), (144, 4), (240, 1)],
    ];
    for (n, shape) in shapes.iter().enumerate() {
        let mut s = session(Some(product.clone()));
        s.timeline_levels = points(shape);
        journal.save_session(None, s, 2 + n as u64).unwrap();
    }
    // A session without set points doesn't count.
    journal
        .save_session(None, session(Some(product.clone())), 9)
        .unwrap();

    let found = journal.timeline_insights().unwrap();
    let names: Vec<(&str, &str)> = found
        .iter()
        .map(|t| (t.category.as_str(), t.name.as_str()))
        .collect();
    assert_eq!(names, [("method", "edible"), ("product", "Gummies")]);
    let edible = &found[0];
    assert_eq!(edible.sessions.len(), 3);
    assert_eq!(edible.onset_min, Some(96));
    assert_eq!((edible.peak_min, edible.peak_level), (144, 4));
    assert_eq!(edible.eases_min, Some(240));
    // With no other edibles, nothing to compare the gummies with.
    assert!(found[1].comparisons.is_empty());

    // Three other edibles that come on and peak later, and milder.
    for n in 0..3 {
        let mut s = session(None);
        s.timeline_levels = points(&[(48, 0), (96, 0), (192, 3), (240, 1)]);
        journal.save_session(None, s, 10 + n).unwrap();
    }
    let found = journal.timeline_insights().unwrap();
    let gummies = found.iter().find(|t| t.category == "product").unwrap();
    let compared: Vec<(&str, &str, u32, u32)> = gummies
        .comparisons
        .iter()
        .map(|c| (c.method.as_str(), c.measure.as_str(), c.this, c.others))
        .collect();
    assert_eq!(
        compared,
        [
            ("edible", "onset", 96, 192),
            ("edible", "peak_time", 144, 192),
            ("edible", "peak_level", 4, 3),
        ]
    );
    assert_eq!(
        (
            gummies.comparisons[0].sessions,
            gummies.comparisons[0].others_count
        ),
        (3, 3)
    );

    // Hidden with the timeline in "What I track".
    journal
        .set_data_point_tracked("timeline".into(), false, 20)
        .unwrap();
    assert!(journal.timeline_insights().unwrap().is_empty());
}

#[test]
fn merging_two_products_keeps_the_most_and_repoints_their_lines() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let tablet_dir = Device::new();
    let phone = phone_dir.open(&keys).unwrap();
    let spot = phone.save_spot(None, plain_spot("Green Leaf"), 1).unwrap();
    let mut a = plain_product("Blue Dream");
    a.form = Some("flower".into());
    a.receipt_names = vec!["BD 3.5G".into()];
    let a = phone.save_product(None, a, 1).unwrap();
    let mut b = plain_product("");
    b.form = Some("flower".into());
    b.thc_milli = Some(22_000);
    b.thc_unit = Some("percent".into());
    b.batch = Some("LOT-9".into());
    b.shops = vec![spot.clone()];
    b.identifiers = vec![IdentifierData {
        code: "0850012345678".into(),
        kind: "barcode".into(),
    }];
    b.receipt_names = vec!["BD 3.5G".into(), "BLUE DRM 1/8".into()];
    let b = phone.save_product(None, b, 1).unwrap();
    let session_a = phone
        .save_session(None, session(Some(a.clone())), 2)
        .unwrap();
    let session_b = phone
        .save_session(None, session(Some(b.clone())), 2)
        .unwrap();
    let line = |product: &str| PurchaseLineData {
        line_id: None,
        product: Some(product.into()),
        batch: None,
        price_minor_units: None,
        price_currency: None,
        size: None,
        quantity: 1,
    };
    let purchase = phone
        .save_purchase(
            None,
            PurchaseData {
                when: LocalTimeData {
                    unix_ms: 1_760_000_000_000,
                    utc_offset_min: 0,
                },
                spot: Some(spot.clone()),
                lines: vec![line(&a), line(&b)],
            },
            2,
        )
        .unwrap();

    // Both learned "BD 3.5G" (as after a sync): offered as equal choices.
    let found = phone
        .suggest_for_receipt_item("BD 3.5G".into(), None)
        .unwrap();
    assert!(found[..2].iter().all(|s| s.learned && s.close));
    // The one recording more is kept by default, whichever comes first.
    assert_eq!(phone.product_to_keep(a.clone(), b.clone()).unwrap(), b);
    assert_eq!(phone.product_to_keep(b.clone(), a.clone()).unwrap(), b);

    let tablet = second_device(&keys, &tablet_dir);
    sync(&phone, &tablet, 3);
    // Meanwhile on the tablet: an edit to the product being merged away,
    // and a new session naming it.
    let mut renamed = tablet.get_product(b.clone()).unwrap().unwrap();
    renamed.name = "Blue Dream (tablet)".into();
    tablet.save_product(Some(b.clone()), renamed, 4).unwrap();
    let late_session = tablet
        .save_session(None, session(Some(b.clone())), 4)
        .unwrap();

    // The user keeps Blue Dream (swapping the default).
    assert_eq!(phone.merge_products(a.clone(), b.clone(), 5).unwrap(), 2);
    assert!(phone.get_product(b.clone()).unwrap().is_none());
    let merged = phone.get_product(a.clone()).unwrap().unwrap();
    assert_eq!(merged.name, "Blue Dream");
    assert_eq!(merged.thc_milli, Some(22_000));
    assert_eq!(merged.batch.as_deref(), Some("LOT-9"));
    assert_eq!(merged.shops, std::slice::from_ref(&spot));
    assert_eq!(merged.identifiers.len(), 1);
    assert_eq!(merged.receipt_names, ["BD 3.5G", "BLUE DRM 1/8"]);
    // Every line that named the other product names the kept one.
    let lines_of = |journal: &Journal, id: &str| -> Vec<Option<String>> {
        journal
            .get_session(id.into())
            .unwrap()
            .unwrap()
            .lines
            .into_iter()
            .map(|l| l.product)
            .collect()
    };
    assert_eq!(lines_of(&phone, &session_b), [Some(a.clone())]);
    let purchased: Vec<Option<String>> = phone
        .get_purchase(purchase.clone())
        .unwrap()
        .unwrap()
        .lines
        .into_iter()
        .map(|l| l.product)
        .collect();
    assert_eq!(purchased, [Some(a.clone()), Some(a.clone())]);
    // The index: nothing refers to the merged-away product.
    assert!(phone.referencing(b.clone()).unwrap().is_empty());
    let mut refs = phone.referencing(a.clone()).unwrap();
    refs.sort();
    let mut expected = vec![session_a.clone(), session_b.clone(), purchase.clone()];
    expected.sort();
    assert_eq!(refs, expected);
    // The learned name now links on its own.
    let found = phone
        .suggest_for_receipt_item("BD 3.5G".into(), None)
        .unwrap();
    assert!(found[0].learned && !found[0].close && found[0].id == a);
    // Insights and shop insights read the kept product.
    phone.insights().unwrap();
    phone.shop_insights().unwrap();

    // It syncs as edits and a deletion, both ways.
    sync(&phone, &tablet, 6);
    sync(&tablet, &phone, 7);
    for journal in [&phone, &tablet] {
        assert!(journal.get_product(b.clone()).unwrap().is_none());
        let merged = journal.get_product(a.clone()).unwrap().unwrap();
        assert_eq!(merged.receipt_names, ["BD 3.5G", "BLUE DRM 1/8"]);
        assert_eq!(lines_of(journal, &session_b), [Some(a.clone())]);
        // A session naming the other product written elsewhere before the
        // merge arrived keeps its line, as after any deletion.
        assert_eq!(lines_of(journal, &late_session), [Some(b.clone())]);
    }

    // Refused: the same product twice, one already merged, not a product.
    assert_eq!(
        phone.merge_products(a.clone(), a.clone(), 8),
        Err(CoreError::InvalidInput)
    );
    assert_eq!(
        phone.merge_products(a.clone(), b.clone(), 8),
        Err(CoreError::NotFound)
    );
    assert_eq!(
        phone.merge_products(a.clone(), session_a.clone(), 8),
        Err(CoreError::WrongType)
    );
    assert_eq!(
        phone.product_to_keep(a, session_a),
        Err(CoreError::WrongType)
    );
}

#[test]
fn catalog_parents_link_to_journal_strains() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let haze = journal
        .save_strain(
            None,
            StrainData {
                name: "Haze".into(),
                lineage: None,
                terpenes: vec![],
                strain_type: None,
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
                breeder: Some("Haze Brothers".into()),
            },
            1_000,
        )
        .unwrap();
    assert_eq!(
        journal
            .get_strain(haze.clone())
            .unwrap()
            .unwrap()
            .breeder
            .as_deref(),
        Some("Haze Brothers")
    );
    let catalog = StrainCatalog::open(
        "name\ttype\tbreeder\tparents\nBlue Dream\tsativa\t\tBlueberry | Haze\n".into(),
    )
    .unwrap();
    assert_eq!(catalog.size(), 1);
    let found = catalog.search("blue dr".into(), 5);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].strain_type.as_deref(), Some("sativa"));
    assert_eq!(found[0].match_kind, "prefix");
    let ids = journal.strain_ids_named(found[0].parents.clone()).unwrap();
    assert_eq!(ids, vec![String::new(), haze]);
    assert!(StrainCatalog::open("not a catalog".into()).is_err());
}

#[test]
fn labels_name_journal_brands_and_strains() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    let acme = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme Farms".into(),
            },
            1_000,
        )
        .unwrap();
    let strain = |name: &str| StrainData {
        name: name.into(),
        lineage: None,
        terpenes: vec![],
        strain_type: None,
        terpene_levels: vec![],
        parents: vec![],
        cannabinoids: vec![],
        breeder: None,
    };
    let dream = journal
        .save_strain(None, strain("Blue Dream"), 1_001)
        .unwrap();
    journal.save_strain(None, strain("Dream"), 1_002).unwrap();
    journal.save_strain(None, strain("OG Kush"), 1_003).unwrap();
    let found = journal
        .names_on_label("ACME FARMS\nBLUE DREAM 3.5G\nTHC 22%".into())
        .unwrap();
    let got: Vec<(&str, &str)> = found
        .iter()
        .map(|n| (n.record_type.as_str(), n.id.as_str()))
        .collect();
    assert_eq!(got, [("brand", acme.as_str()), ("strain", dream.as_str())]);

    let catalog = StrainCatalog::open(
        "name\ttype\tbreeder\tparents\nBlue Dream\tsativa\t\t\nPineapple Express\t\t\t\n".into(),
    )
    .unwrap();
    let label = "BLUE DREAM\nPINEAPPLE EXPRESS";
    let names: Vec<String> = catalog
        .find_in_label(label.into(), vec!["Blue Dream".into()], 3)
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["Pineapple Express"]);
}

#[test]
fn lookup_sites_are_saved_edited_and_forgotten() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.open(&keys).unwrap();
    assert!(journal.lookup_sites().unwrap().is_empty());
    let template = lookup_template_from_example(
        "https://www.allbud.com/marijuana-strains/search?q=blue+dream".into(),
        "Blue Dream".into(),
    )
    .unwrap();
    let allbud = journal
        .save_lookup_site(None, " AllBud ".into(), template.clone(), 1_000)
        .unwrap();
    let mine = journal
        .save_lookup_site(
            None,
            "a site".into(),
            "https://example.org/{strain-}".into(),
            1_001,
        )
        .unwrap();
    let names: Vec<String> = journal
        .lookup_sites()
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["a site", "AllBud"]);
    assert_eq!(
        lookup_address(template, "OG Kush".into()).as_deref(),
        Some("https://www.allbud.com/marijuana-strains/search?q=OG+Kush")
    );

    // Refused: not https, no placeholder, an unknown ID.
    for bad in ["http://example.org/{strain}", "https://example.org/"] {
        assert_eq!(
            journal.save_lookup_site(None, "x".into(), bad.into(), 1_002),
            Err(CoreError::InvalidInput)
        );
        assert!(!lookup_template_valid(bad.into()));
    }
    assert_eq!(
        journal.save_lookup_site(
            Some("nope".into()),
            "x".into(),
            "https://e.org/{strain}".into(),
            1_002
        ),
        Err(CoreError::NotFound)
    );

    journal
        .save_lookup_site(
            Some(mine.clone()),
            "Mine".into(),
            "https://example.org/s/{strain}".into(),
            1_003,
        )
        .unwrap();
    journal.delete_lookup_site(allbud, 1_004).unwrap();
    let left = journal.lookup_sites().unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(
        (left[0].id.as_str(), left[0].name.as_str()),
        (mine.as_str(), "Mine")
    );

    // The toggle's off-action.
    journal.forget_lookup_sites(1_005).unwrap();
    assert!(journal.lookup_sites().unwrap().is_empty());
}

#[test]
fn a_new_purchase_unarchives_its_products() {
    let device = Device::new();
    let keys = create_keys().unwrap();
    let journal = device.open(&keys).unwrap();
    let mut data = plain_product("Blue Dream");
    data.archived = true;
    let product = journal.save_product(None, data, 1).unwrap();
    assert!(
        journal
            .get_product(product.clone())
            .unwrap()
            .unwrap()
            .archived
    );
    let purchase = PurchaseData {
        when: LocalTimeData {
            unix_ms: 1_759_900_000_000,
            utc_offset_min: 0,
        },
        spot: None,
        lines: vec![PurchaseLineData {
            line_id: None,
            product: Some(product.clone()),
            batch: None,
            price_minor_units: None,
            price_currency: None,
            size: None,
            quantity: 1,
        }],
    };
    let id = journal.save_purchase(None, purchase.clone(), 2).unwrap();
    assert!(
        !journal
            .get_product(product.clone())
            .unwrap()
            .unwrap()
            .archived
    );

    // Editing an older purchase leaves an archived product archived.
    let mut done = journal.get_product(product.clone()).unwrap().unwrap();
    done.archived = true;
    journal
        .save_product(Some(product.clone()), done, 3)
        .unwrap();
    journal.save_purchase(Some(id), purchase, 4).unwrap();
    assert!(journal.get_product(product).unwrap().unwrap().archived);
}
