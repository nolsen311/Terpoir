// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Writes the compatibility fixtures for one released version
//! (`core/crates/terpoir-ffi/tests/fixtures/README.md`). Not built from
//! the main branch: copy it into a checkout of the release as
//! `core/crates/terpoir-ffi/examples/make_compat_fixtures.rs` and run
//!
//! ```text
//! cargo run -p terpoir-ffi --example make_compat_fixtures -- <out dir>
//! ```
//!
//! with a `CARGO_TARGET_DIR` of its own. Every key and passphrase here is
//! a published test value: never use them for a real journal.

#![allow(clippy::unwrap_used, clippy::print_stdout, missing_docs)]

use std::fs;
use std::path::Path;
use std::sync::Arc;

use terpoir_crypto::{DatabaseKey, MasterKey, RecoveryKey};
use terpoir_ffi::{
    BrandData, CannabinoidAmountData, IdentifierData, InsightFeedback, Journal, LocalTimeData, ProductData,
    ProductLineData, PurchaseData, PurchaseLineData, SessionData, SnippetData, SpotData,
    StrainData, SymptomData, TagData, TagIntensity, TagProvenance, TagRefData,
    TerpeneLevelData, seal_capture, wrap_master_key_with_passphrase,
};

/// Recovery key bytes 0x00..=0x1f, in display form.
const RECOVERY_KEY: &str = "0400-20G3-0G2G-C1R8-1450-P30D-1R7H-048J-2CA1-A5GQ-30CH-M6RW-3MF1-Y1YN";
/// The master key: 32 bytes of 0x4d ("M").
const MASTER_KEY: [u8; 32] = [0x4d; 32];
const PASSPHRASE: &str = "terpoir fixture passphrase";
const MAPTILER_KEY: &str = "FixtureMapTilerKey0";
/// Wall clock for every write: 2025-10-09T08:53:20Z plus a step.
const T0: u64 = 1_760_000_000_000;

fn builtin(slug: &str) -> TagRefData {
    TagRefData::Builtin { slug: slug.into() }
}

fn at(unix_ms: i64) -> LocalTimeData {
    LocalTimeData {
        unix_ms,
        utc_offset_min: -420,
    }
}

/// The attachment's bytes: 3000 bytes counting 0..=255.
fn attachment_bytes() -> Vec<u8> {
    (0..3000u32).map(|i| (i % 256) as u8).collect()
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn write_blobs(dir: &Path, journal: &Journal) -> Vec<String> {
    let mut names = Vec::new();
    for blob in journal.export_sync_blobs().unwrap() {
        write(&dir.join(format!("{}.terpoir", blob.name)), &blob.bytes);
        names.push(blob.name);
    }
    names
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("usage: make_compat_fixtures <out dir>");
    let out = Path::new(&out);
    assert!(!out.exists(), "{} exists already", out.display());

    // Keys.
    let recovery = RecoveryKey::parse(RECOVERY_KEY).unwrap();
    let master = MasterKey::from_slice(&MASTER_KEY).unwrap();
    let master_wrapped_by_recovery = recovery
        .key_encryption_key()
        .unwrap()
        .wrap_master_key(&master)
        .unwrap()
        .as_bytes()
        .to_vec();
    let wrapped_database_key = master
        .wrap_database_key(&DatabaseKey::generate().unwrap())
        .unwrap()
        .as_bytes()
        .to_vec();
    let passphrase =
        wrap_master_key_with_passphrase(MASTER_KEY.to_vec(), PASSPHRASE.into()).unwrap();
    write(
        &out.join("keys/master_wrapped_by_recovery.bin"),
        &master_wrapped_by_recovery,
    );
    write(
        &out.join("keys/wrapped_database_key.bin"),
        &wrapped_database_key,
    );
    write(&out.join("keys/passphrase_params.bin"), &passphrase.params);
    write(
        &out.join("keys/passphrase_wrapped_master_key.bin"),
        &passphrase.wrapped_master_key,
    );

    // Work in a temporary place; copy the database once it is closed.
    let work = std::env::temp_dir().join(format!("terpoir-fixtures-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    let journal: Arc<Journal> = Journal::open(
        work.join("journal.db").to_string_lossy().into_owned(),
        work.join("files").to_string_lossy().into_owned(),
        MASTER_KEY.to_vec(),
        wrapped_database_key.clone(),
    )
    .unwrap();
    let inbox = work.join("inbox").to_string_lossy().into_owned();
    // The capture key pair, as the app makes on its first unlock.
    assert_eq!(journal.merge_captures(inbox.clone(), T0).unwrap(), 0);

    // Every record kind, every optional field set somewhere.
    let brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Acme".into(),
            },
            T0 + 1,
        )
        .unwrap();
    let deleted_brand = journal
        .save_brand(
            None,
            BrandData {
                name: "Gone".into(),
            },
            T0 + 2,
        )
        .unwrap();
    // Since 0.4.0: a parent strain with only a name, as the editor makes
    // for a typed name, and terpene levels, parents and cannabinoids.
    let parent_strain = journal
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
            },
            T0 + 3,
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
                terpene_levels: vec![
                    TerpeneLevelData {
                        terpene: builtin("myrcene"),
                        level: "dominant".into(),
                    },
                    TerpeneLevelData {
                        terpene: builtin("pinene"),
                        level: "secondary".into(),
                    },
                ],
                parents: vec![parent_strain.clone()],
                cannabinoids: vec!["cbg".into()],
            },
            T0 + 3,
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
            },
            T0 + 4,
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
                // Since 0.5.0.
                cannabinoids: vec![CannabinoidAmountData {
                    cannabinoid: "cbn".into(),
                    milli: 2_000,
                    unit: "mg_per_unit".into(),
                }],
            },
            T0 + 5,
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
                }],
            },
            T0 + 6,
        )
        .unwrap();
    let purchase_line = journal
        .get_purchase(purchase.clone())
        .unwrap()
        .unwrap()
        .lines[0]
        .line_id
        .clone()
        .unwrap();
    let tag = journal
        .save_tag(
            None,
            TagData {
                name: "giggly".into(),
                category: "effect".into(),
            },
            T0 + 7,
        )
        .unwrap();
    let symptom_tag = journal
        .save_tag(
            None,
            TagData {
                name: "jaw tension".into(),
                category: "symptom".into(),
            },
            T0 + 8,
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
                    purchase_id: Some(purchase.clone()),
                    purchase_line_id: Some(purchase_line.clone()),
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
                spot: Some(spot.clone()),
                symptoms: vec![
                    SymptomData {
                        tag: builtin("sleep"),
                        result: "helped".into(),
                    },
                    SymptomData {
                        tag: TagRefData::Custom {
                            id: symptom_tag.clone(),
                        },
                        result: "did_not_help".into(),
                    },
                ],
                attachments: vec![],
            },
            T0 + 9,
        )
        .unwrap();
    let snippet = journal
        .save_snippet(
            None,
            SnippetData {
                session: Some(session.clone()),
                at: at(1_760_000_100_000),
                text: "so giggly 🙂".into(),
                tags: vec![TagProvenance {
                    tag: TagRefData::Custom { id: tag.clone() },
                    provenance: "user".into(),
                }],
                added_while_locked: false,
            },
            T0 + 10,
        )
        .unwrap();

    // The sync folder's older blobs: before the edit and the deletion below.
    let early_blobs = write_blobs(&out.join("sync"), &journal);

    // An edit, a deletion, preferences, an attachment, a captured note.
    let mut edited = journal.get_session(session.clone()).unwrap().unwrap();
    edited.rating = Some(5);
    journal
        .save_session(Some(session.clone()), edited, T0 + 11)
        .unwrap();
    assert!(journal.delete_record(deleted_brand.clone()).unwrap());
    journal
        .set_data_point_tracked("terpenes".into(), false, T0 + 12)
        .unwrap();
    journal
        .set_strain_detail("detailed".into(), T0 + 12)
        .unwrap();
    journal
        .set_insight_feedback(
            "fixture-insight".into(),
            InsightFeedback::Dismissed,
            T0 + 13,
        )
        .unwrap();
    let attachment = journal
        .add_attachment(session.clone(), attachment_bytes(), T0 + 14)
        .unwrap();
    seal_capture(
        inbox.clone(),
        "a note written while locked".into(),
        at(1_760_000_200_000),
    )
    .unwrap();
    assert_eq!(journal.merge_captures(inbox.clone(), T0 + 15).unwrap(), 1);
    let captured = journal.locked_captures().unwrap();
    assert_eq!(captured.len(), 1);
    let captured_note = captured[0].clone();
    journal.set_maptiler_key(Some(MAPTILER_KEY.into())).unwrap();

    // The sync folder's newer blobs, and the export.
    let final_blobs = write_blobs(&out.join("sync"), &journal);
    write(
        &out.join("export.terpoir"),
        &journal
            .export_archive(master_wrapped_by_recovery.clone())
            .unwrap(),
    );

    // A note sealed while locked and not merged yet: the inbox as an
    // upgrade finds it.
    seal_capture(
        inbox.clone(),
        "a note waiting in the inbox".into(),
        at(1_760_000_300_000),
    )
    .unwrap();

    journal.maintenance().unwrap();
    journal.lock();
    drop(journal);

    for entry in fs::read_dir(&work).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        match name.as_str() {
            "journal.db" => write(
                &out.join("database/journal.db"),
                &fs::read(entry.path()).unwrap(),
            ),
            "files" | "inbox" => {
                for file in fs::read_dir(entry.path()).unwrap() {
                    let file = file.unwrap();
                    write(
                        &out.join(if name == "files" {
                            "database/files"
                        } else {
                            "inbox"
                        })
                        .join(file.file_name()),
                        &fs::read(file.path()).unwrap(),
                    );
                }
            }
            // An empty WAL left after a checkpoint carries nothing.
            "journal.db-wal" | "journal.db-shm" => {
                assert!(
                    name.ends_with("-shm") || entry.metadata().unwrap().len() == 0,
                    "unexpected non-empty WAL"
                );
            }
            other => panic!("unexpected file {other}"),
        }
    }
    fs::remove_dir_all(&work).unwrap();

    let list = |v: &[String]| {
        v.iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let manifest = format!(
        r#"{{
  "core_version": "{version}",
  "recovery_key": "{RECOVERY_KEY}",
  "passphrase": "{PASSPHRASE}",
  "maptiler_key": "{MAPTILER_KEY}",
  "product_cannabinoids": true,
  "ids": {{
    "brand": "{brand}",
    "deleted_brand": "{deleted_brand}",
    "strain": "{strain}",
    "parent_strain": "{parent_strain}",
    "spot": "{spot}",
    "product": "{product}",
    "purchase": "{purchase}",
    "purchase_line": "{purchase_line}",
    "tag": "{tag}",
    "symptom_tag": "{symptom_tag}",
    "session": "{session}",
    "snippet": "{snippet}",
    "captured_note": "{captured_note}",
    "attachment": "{attachment}"
  }},
  "sync_blobs_before_edit": [{early}],
  "sync_blobs_final": [{finals}]
}}
"#,
        version = terpoir_ffi::core_version(),
        early = list(&early_blobs),
        finals = list(&final_blobs),
    );
    write(&out.join("manifest.json"), manifest.as_bytes());
    println!("fixtures written to {}", out.display());
}
