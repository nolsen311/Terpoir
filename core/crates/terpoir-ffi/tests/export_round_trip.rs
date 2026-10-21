// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The encrypted export, end to end through the platform API: a journal
//! with every kind of record is exported, restored on a new device, and
//! compared field by field (through the plaintext export, which reads
//! every field of every record generically). Also: importing back,
//! merging into a journal that changed since, large journals, and files
//! that must be refused.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use terpoir_ffi::{
    BrandData, CannabinoidAmountData, CoreError, IdentifierData, Journal, LocalTimeData, NewKeys,
    ProductData, ProductLineData, PurchaseData, PurchaseLineData, RecordType, SessionData,
    SnippetData, SpotData, StrainData, SymptomData, TagData, TagIntensity, TagProvenance,
    TagRefData, TimelineLevelData, create_keys, keys_for_restore, read_export_archive,
};

const TYPES: [RecordType; 8] = [
    RecordType::Session,
    RecordType::Product,
    RecordType::Strain,
    RecordType::Brand,
    RecordType::Spot,
    RecordType::Snippet,
    RecordType::Tag,
    RecordType::Purchase,
];

/// One device's app storage.
struct Device {
    dir: PathBuf,
}

impl Device {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "terpoir-export-test-{}",
            terpoir_model::RecordId::random().unwrap().to_hex()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn open_with(&self, master_key: Vec<u8>, wrapped_database_key: Vec<u8>) -> Arc<Journal> {
        Journal::open(
            self.dir.join("journal.db").to_string_lossy().into_owned(),
            self.dir.join("files").to_string_lossy().into_owned(),
            master_key,
            wrapped_database_key,
        )
        .unwrap()
    }

    fn create(&self, keys: &NewKeys) -> Arc<Journal> {
        self.open_with(keys.master_key.clone(), keys.wrapped_database_key.clone())
    }

    /// What "Restore from an export" does on a new phone: the recovery
    /// key unwraps the master key, a new database key is made, and every
    /// blob is imported.
    fn restore(&self, recovery_key: &str, archive: Vec<u8>) -> Arc<Journal> {
        let parsed = read_export_archive(archive).unwrap();
        let keys = keys_for_restore(
            recovery_key.into(),
            parsed.master_wrapped_by_recovery.clone(),
        )
        .unwrap();
        let journal = self.open_with(keys.master_key, keys.wrapped_database_key);
        for blob in parsed.blobs {
            let result = journal.import_sync_blob(blob, 1_000).unwrap();
            assert_eq!(result.deferred, 0);
        }
        journal
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn builtin(slug: &str) -> TagRefData {
    TagRefData::Builtin { slug: slug.into() }
}

fn at(unix_ms: i64) -> LocalTimeData {
    LocalTimeData {
        unix_ms,
        utc_offset_min: -420,
    }
}

/// Every record of the journal, every field, as the plaintext export
/// writes it (with a fixed export time, so two journals compare equal
/// exactly when their contents do).
fn contents(journal: &Journal) -> Vec<(String, String)> {
    journal
        .export_plaintext(0)
        .unwrap()
        .into_iter()
        .map(|f| (f.name, String::from_utf8(f.bytes).unwrap()))
        .collect()
}

fn ids(journal: &Journal) -> Vec<Vec<String>> {
    TYPES.iter().map(|t| journal.list(*t).unwrap()).collect()
}

/// IDs of what [`fill`] wrote, for later edits and checks.
struct Filled {
    session: String,
    product: String,
    deleted: String,
}

/// A journal with every kind of record, every optional field set
/// somewhere, references between them, edits and a deletion.
fn fill(journal: &Journal) -> Filled {
    let brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme".into(),
            },
            1,
        )
        .unwrap();
    let deleted = journal
        .save_brand(
            None,
            BrandData {
                name: "Gone".into(),
            },
            1,
        )
        .unwrap();
    let strain = journal
        .save_strain(
            None,
            StrainData {
                name: "Blue Dream".into(),
                lineage: Some("Blueberry x Haze".into()),
                terpenes: vec![builtin("myrcene"), builtin("pinene")],
                strain_type: Some("hybrid".into()),
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
                breeder: Some("DJ Short".into()),
            },
            1,
        )
        .unwrap();
    let spot = journal
        .save_spot(
            None,
            SpotData {
                name: "Corner shop".into(),
                purposes: vec![builtin("buy")],
                lat_e7: Some(377_000_000),
                lon_e7: Some(-1_224_000_000),
                accuracy_m: Some(25),
                radius_m: Some(150),
                purchase_prompts: true,
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
                strains: vec![strain],
                thc_milli: Some(5_000),
                thc_unit: Some("mg_per_unit".into()),
                cbd_milli: Some(1_000),
                cbd_unit: Some("mg_per_unit".into()),
                terpenes: vec![TagIntensity {
                    tag: builtin("limonene"),
                    value: 1_250,
                }],
                batch: Some("B-7".into()),
                package_date_day: Some(20_300),
                identifiers: vec![IdentifierData {
                    code: "0123456789012".into(),
                    kind: "barcode".into(),
                }],
                shops: vec![spot.clone()],
                cannabinoids: vec![CannabinoidAmountData {
                    cannabinoid: "cbn".into(),
                    milli: 2_000,
                    unit: "mg_per_unit".into(),
                }],
                subtype: None,
                default_size: None,
                receipt_names: vec![],
            },
            2,
        )
        .unwrap();
    let purchase = journal
        .save_purchase(
            None,
            PurchaseData {
                when: at(1_759_900_000_000),
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
    let bought = journal.get_purchase(purchase.clone()).unwrap().unwrap();
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
    let session = journal
        .save_session(
            None,
            SessionData {
                when: at(1_760_000_000_000),
                lines: vec![ProductLineData {
                    line_id: None,
                    product: Some(product.clone()),
                    method: "edible".into(),
                    dose_milli: Some(5_000),
                    dose_unit: Some("mg".into()),
                    purchase_id: Some(purchase),
                    purchase_line_id: bought.lines[0].line_id.clone(),
                    shop: None,
                }],
                activity: Some("movie night".into()),
                intention: Some("relax".into()),
                mood_before: Some(3),
                social: Some("with_others".into()),
                effects: vec![
                    TagIntensity {
                        tag: builtin("relaxed"),
                        value: 4,
                    },
                    TagIntensity {
                        tag: TagRefData::Custom { id: tag.clone() },
                        value: 2,
                    },
                ],
                onset_min: Some(60),
                peak_min: Some(120),
                duration_min: Some(240),
                mood_after: Some(5),
                rating: Some(4),
                would_repeat: Some(true),
                spot: Some(spot),
                symptoms: vec![SymptomData {
                    tag: builtin("sleep"),
                    result: "helped".into(),
                }],
                attachments: vec![],
                timeline_levels: vec![TimelineLevelData {
                    minute: 30,
                    level: 3,
                }],
                timeline_updated_ms: Some(1_760_000_000_000),
            },
            3,
        )
        .unwrap();
    journal
        .save_snippet(
            None,
            SnippetData {
                session: Some(session.clone()),
                at: at(1_760_000_100_000),
                text: "so giggly 🙂".into(),
                tags: vec![TagProvenance {
                    tag: TagRefData::Custom { id: tag },
                    provenance: "user".into(),
                }],
                added_while_locked: false,
            },
            3,
        )
        .unwrap();
    journal
        .save_snippet(
            None,
            SnippetData {
                session: None,
                at: at(1_760_000_200_000),
                text: "a note written while locked".into(),
                tags: vec![],
                added_while_locked: true,
            },
            3,
        )
        .unwrap();
    // An edit after the first save, and a deletion.
    let mut edited = journal.get_session(session.clone()).unwrap().unwrap();
    edited.rating = Some(5);
    journal
        .save_session(Some(session.clone()), edited, 4)
        .unwrap();
    assert!(journal.delete_record(deleted.clone()).unwrap());
    Filled {
        session,
        product,
        deleted,
    }
}

#[test]
fn every_record_survives_export_and_restore_on_a_new_device() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let phone = phone_dir.create(&keys);
    let filled = fill(&phone);
    let before = contents(&phone);
    assert!(before.iter().any(|(_, text)| text.contains("Blue Dream")));

    let archive = phone
        .export_archive(keys.master_wrapped_by_recovery.clone())
        .unwrap();
    let new_dir = Device::new();
    let restored = new_dir.restore(&keys.recovery_key, archive);

    assert_eq!(contents(&restored), before);
    assert_eq!(ids(&restored), ids(&phone));
    // Deleted stays deleted, and typed reads agree too.
    assert!(restored.get_brand(filled.deleted).unwrap().is_none());
    assert!(
        restored.get_session(filled.session.clone()).unwrap()
            == phone.get_session(filled.session.clone()).unwrap()
    );
    assert!(
        restored.get_product(filled.product.clone()).unwrap()
            == phone.get_product(filled.product).unwrap()
    );

    // The restored journal is a full journal: an edit there exports and
    // imports back into the original.
    let mut edit = restored
        .get_session(filled.session.clone())
        .unwrap()
        .unwrap();
    edit.activity = Some("edited after restoring".into());
    restored
        .save_session(Some(filled.session.clone()), edit, 10)
        .unwrap();
    // (The comparison sees a one-field difference.)
    assert_ne!(contents(&phone), contents(&restored));
    let back = read_export_archive(
        restored
            .export_archive(keys.master_wrapped_by_recovery.clone())
            .unwrap(),
    )
    .unwrap();
    for blob in back.blobs {
        phone.import_sync_blob(blob, 11).unwrap();
    }
    assert_eq!(contents(&phone), contents(&restored));
    assert!(
        phone
            .get_session(filled.session)
            .unwrap()
            .unwrap()
            .activity
            .as_deref()
            == Some("edited after restoring")
    );
}

#[test]
fn importing_an_export_again_changes_nothing() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.create(&keys);
    fill(&journal);
    let before = contents(&journal);
    let archive = journal
        .export_archive(keys.master_wrapped_by_recovery.clone())
        .unwrap();
    for _ in 0..2 {
        for blob in read_export_archive(archive.clone()).unwrap().blobs {
            let result = journal.import_sync_blob(blob, 20).unwrap();
            assert_eq!(result.deferred, 0);
        }
        assert_eq!(contents(&journal), before);
    }
}

#[test]
fn an_older_export_never_undoes_later_changes() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.create(&keys);
    let filled = fill(&journal);
    let old = journal
        .export_archive(keys.master_wrapped_by_recovery.clone())
        .unwrap();

    // After the export: an edit, a deletion and a new record.
    let mut later = journal
        .get_session(filled.session.clone())
        .unwrap()
        .unwrap();
    later.rating = Some(1);
    journal
        .save_session(Some(filled.session.clone()), later, 20)
        .unwrap();
    assert!(journal.delete_record(filled.product.clone()).unwrap());
    journal
        .save_brand(None, BrandData { name: "New".into() }, 20)
        .unwrap();
    let after = contents(&journal);

    for blob in read_export_archive(old).unwrap().blobs {
        journal.import_sync_blob(blob, 30).unwrap();
    }
    assert_eq!(contents(&journal), after);
    assert!(journal.get_product(filled.product).unwrap().is_none());
    assert!(journal.get_session(filled.session).unwrap().unwrap().rating == Some(1));
}

#[test]
fn a_large_journal_spans_several_blobs_and_restores_whole() {
    let keys = create_keys().unwrap();
    let phone_dir = Device::new();
    let phone = phone_dir.create(&keys);
    // About 6 MiB of notes: more than one blob's worth.
    for i in 0..48u8 {
        let text = format!("{i:02} {}", char::from(b'a' + i % 26)).repeat(32 * 1024);
        phone
            .save_snippet(
                None,
                SnippetData {
                    session: None,
                    at: at(1_760_000_000_000 + i64::from(i)),
                    text,
                    tags: vec![],
                    added_while_locked: false,
                },
                1,
            )
            .unwrap();
    }
    let archive = phone
        .export_archive(keys.master_wrapped_by_recovery.clone())
        .unwrap();
    assert!(read_export_archive(archive.clone()).unwrap().blobs.len() > 1);
    let new_dir = Device::new();
    let restored = new_dir.restore(&keys.recovery_key, archive);
    assert_eq!(restored.list(RecordType::Snippet).unwrap().len(), 48);
    assert_eq!(contents(&restored), contents(&phone));
}

#[test]
fn damaged_and_foreign_exports_are_refused_without_changes() {
    let keys = create_keys().unwrap();
    let dir = Device::new();
    let journal = dir.create(&keys);
    fill(&journal);
    let before = contents(&journal);
    let archive = journal
        .export_archive(keys.master_wrapped_by_recovery.clone())
        .unwrap();

    // Another journal's export: its blobs don't open under this key.
    let other_keys = create_keys().unwrap();
    let other_dir = Device::new();
    let other = other_dir.create(&other_keys);
    fill(&other);
    let foreign = other
        .export_archive(other_keys.master_wrapped_by_recovery.clone())
        .unwrap();
    for blob in read_export_archive(foreign.clone()).unwrap().blobs {
        assert_eq!(
            journal.import_sync_blob(blob, 40).err(),
            Some(CoreError::InvalidSyncData)
        );
    }
    // Restoring needs that journal's recovery key, not this one's.
    assert!(
        keys_for_restore(
            keys.recovery_key.clone(),
            read_export_archive(foreign)
                .unwrap()
                .master_wrapped_by_recovery
        )
        .is_err()
    );

    // A flipped byte inside a blob fails authentication.
    let mut parsed = read_export_archive(archive.clone()).unwrap();
    let blob = &mut parsed.blobs[0];
    let middle = blob.len() / 2;
    blob[middle] ^= 1;
    assert_eq!(
        journal.import_sync_blob(blob.clone(), 40).err(),
        Some(CoreError::InvalidSyncData)
    );
    // Truncated or not an export at all.
    assert!(read_export_archive(archive[..archive.len() / 2].to_vec()).is_err());
    assert!(read_export_archive(b"TERPOIR1".to_vec()).is_err());
    assert!(read_export_archive(Vec::new()).is_err());

    assert_eq!(contents(&journal), before);
}
