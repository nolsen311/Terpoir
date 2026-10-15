// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Compatibility fixtures (docs/design-decisions.md, "Schema upgrades":
//! old backups keep working). `tests/fixtures/<version>/` holds a
//! database, an encrypted export, a sync folder's blobs and a capture
//! inbox written by each released version (see the README there). For
//! every such directory, this test opens each one with this version's
//! code, checks every record field by field, and round-trips it through
//! this version's export and sync. A release that can no longer read one
//! fails CI.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use terpoir_ffi::{
    BrandData, CannabinoidAmountData, IdentifierData, Journal, LocalTimeData, PassphraseWrapped,
    ProductData, ProductLineData, PurchaseData, PurchaseLineData, RecordType, SessionData,
    SnippetData, SpotData, StrainData, SymptomData, TagData, TagIntensity, TagProvenance,
    TagRefData, TerpeneLevelData, TimelineLevelData, check_export_archive, keys_for_restore,
    master_key_from_passphrase, master_key_from_recovery, read_export_archive,
};

/// Every fixture version directory, oldest name first.
fn fixture_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    assert!(!dirs.is_empty(), "no fixtures under {}", root.display());
    dirs
}

/// One released version's fixtures.
struct Fixture {
    dir: PathBuf,
    manifest: Value,
}

impl Fixture {
    fn load(dir: PathBuf) -> Self {
        let manifest = serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap())
            .unwrap_or_else(|e| panic!("{}: manifest.json: {e}", dir.display()));
        Self { dir, manifest }
    }

    fn name(&self) -> String {
        self.dir.file_name().unwrap().to_string_lossy().into_owned()
    }

    fn text(&self, key: &str) -> String {
        self.manifest[key].as_str().unwrap().to_owned()
    }

    /// An ID only fixtures from some versions have.
    fn optional_id(&self, key: &str) -> Option<String> {
        self.manifest["ids"][key].as_str().map(str::to_owned)
    }

    fn id(&self, key: &str) -> String {
        self.manifest["ids"][key]
            .as_str()
            .unwrap_or_else(|| panic!("{}: no id {key} in manifest.json", self.name()))
            .to_owned()
    }

    fn bytes(&self, path: &str) -> Vec<u8> {
        fs::read(self.dir.join(path)).unwrap_or_else(|e| panic!("{}: {path}: {e}", self.name()))
    }

    fn recovery_key(&self) -> String {
        self.text("recovery_key")
    }

    /// The sync folder's blobs, by file name.
    fn sync_blobs(&self) -> Vec<(String, Vec<u8>)> {
        let mut blobs: Vec<(String, Vec<u8>)> = fs::read_dir(self.dir.join("sync"))
            .unwrap()
            .map(|e| e.unwrap())
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    fs::read(e.path()).unwrap(),
                )
            })
            .collect();
        blobs.sort();
        assert!(!blobs.is_empty(), "{}: empty sync folder", self.name());
        blobs
    }
}

/// One device's app storage, in a temporary directory.
struct Device {
    dir: PathBuf,
}

impl Device {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "terpoir-compat-test-{}",
            terpoir_model::RecordId::random().unwrap().to_hex()
        ));
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().into_owned()
    }

    fn open(&self, master_key: Vec<u8>, wrapped_database_key: Vec<u8>) -> Arc<Journal> {
        Journal::open(
            self.path("journal.db"),
            self.path("files"),
            master_key,
            wrapped_database_key,
        )
        .unwrap()
    }

    /// A new journal for the fixture's master key, as a restore makes.
    fn fresh(&self, fixture: &Fixture) -> Arc<Journal> {
        let keys = keys_for_restore(
            fixture.recovery_key(),
            fixture.bytes("keys/master_wrapped_by_recovery.bin"),
        )
        .unwrap();
        self.open(keys.master_key, keys.wrapped_database_key)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Every record of the journal, every field, as the plaintext export
/// writes it (fixed export time).
fn contents(journal: &Journal) -> Vec<(String, String)> {
    journal
        .export_plaintext(0)
        .unwrap()
        .into_iter()
        .map(|f| (f.name, String::from_utf8(f.bytes).unwrap()))
        .collect()
}

fn import_all(journal: &Journal, blobs: impl IntoIterator<Item = Vec<u8>>) {
    for blob in blobs {
        let result = journal.import_sync_blob(blob, 2_000_000_000_000).unwrap();
        assert_eq!(result.deferred, 0, "a record needs a newer app");
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

/// The attached file's bytes: 3000 bytes counting 0..=255.
fn attachment_bytes() -> Vec<u8> {
    (0..3000u32).map(|i| (i % 256) as u8).collect()
}

fn check<T: PartialEq>(fixture: &Fixture, what: &str, actual: Option<T>, expected: T) {
    assert!(
        actual.as_ref() == Some(&expected),
        "{}: {what} differs from what the release wrote",
        fixture.name()
    );
}

/// Checks that `journal` holds exactly the fixture journal's records,
/// every field as the generator wrote it.
fn check_records(fixture: &Fixture, journal: &Journal) {
    let id = |key: &str| fixture.id(key);
    let count = |t: RecordType| journal.list(t).unwrap().len();
    assert_eq!(count(RecordType::Session), 1);
    assert_eq!(count(RecordType::Product), 1);
    // Since 0.4.0, a parent strain too.
    let parent_strain = fixture.optional_id("parent_strain");
    assert_eq!(
        count(RecordType::Strain),
        1 + usize::from(parent_strain.is_some())
    );
    assert_eq!(count(RecordType::Brand), 1);
    assert_eq!(count(RecordType::Spot), 1);
    assert_eq!(count(RecordType::Snippet), 2);
    // Since 0.6.0, a custom subtype tag too.
    let subtype_tag = fixture.optional_id("subtype_tag");
    let subtypes = fixture.manifest["product_subtypes"].as_bool() == Some(true);
    assert_eq!(
        count(RecordType::Tag),
        2 + usize::from(subtype_tag.is_some())
    );
    assert_eq!(count(RecordType::Purchase), 1);
    assert_eq!(journal.pending_record_count().unwrap(), 0);

    check(
        fixture,
        "brand",
        journal.get_brand(id("brand")).unwrap(),
        BrandData {
            name: "Acme".into(),
        },
    );
    assert!(
        journal.get_brand(id("deleted_brand")).unwrap().is_none(),
        "{}: a deleted brand came back",
        fixture.name()
    );
    // Older versions wrote no levels, parents or cannabinoids, and those
    // read back empty.
    let (terpene_levels, parents, cannabinoids) = match &parent_strain {
        Some(parent) => (
            vec![
                TerpeneLevelData {
                    terpene: builtin("myrcene"),
                    level: "dominant".into(),
                },
                TerpeneLevelData {
                    terpene: builtin("pinene"),
                    level: "secondary".into(),
                },
            ],
            vec![parent.clone()],
            vec!["cbg".to_owned()],
        ),
        None => (vec![], vec![], vec![]),
    };
    check(
        fixture,
        "strain",
        journal.get_strain(id("strain")).unwrap(),
        StrainData {
            name: "Blue Dream".into(),
            lineage: Some("Blueberry x Haze".into()),
            terpenes: vec![builtin("myrcene"), builtin("pinene")],
            strain_type: Some("hybrid".into()),
            terpene_levels,
            parents,
            cannabinoids,
        },
    );
    if let Some(parent) = &parent_strain {
        check(
            fixture,
            "parent strain",
            journal.get_strain(parent.clone()).unwrap(),
            StrainData {
                name: "Haze".into(),
                lineage: None,
                terpenes: vec![],
                strain_type: None,
                terpene_levels: vec![],
                parents: vec![],
                cannabinoids: vec![],
            },
        );
    }
    check(
        fixture,
        "spot",
        journal.get_spot(id("spot")).unwrap(),
        SpotData {
            name: "Corner shop".into(),
            purposes: vec![builtin("buy")],
            lat_e7: Some(377_000_000),
            lon_e7: Some(-1_224_000_000),
            accuracy_m: Some(25),
            radius_m: Some(150),
            purchase_prompts: true,
        },
    );
    check(
        fixture,
        "product",
        journal.get_product(id("product")).unwrap(),
        ProductData {
            name: "Gummies".into(),
            form: Some("edible".into()),
            brand: Some(id("brand")),
            strains: vec![id("strain")],
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
            shops: vec![id("spot")],
            // Since 0.5.0; older fixtures have none.
            cannabinoids: if fixture.manifest["product_cannabinoids"].as_bool() == Some(true) {
                vec![CannabinoidAmountData {
                    cannabinoid: "cbn".into(),
                    milli: 2_000,
                    unit: "mg_per_unit".into(),
                }]
            } else {
                vec![]
            },
            // Since 0.6.0; older fixtures have none.
            subtype: subtypes.then(|| builtin("gummy")),
        },
    );
    check(
        fixture,
        "purchase",
        journal.get_purchase(id("purchase")).unwrap(),
        PurchaseData {
            when: at(1_759_900_000_000),
            spot: Some(id("spot")),
            lines: vec![PurchaseLineData {
                line_id: Some(id("purchase_line")),
                product: Some(id("product")),
                batch: Some("B-7".into()),
                price_minor_units: Some(2_000),
                price_currency: Some("USD".into()),
            }],
        },
    );
    check(
        fixture,
        "effect tag",
        journal.get_tag(id("tag")).unwrap(),
        TagData {
            name: "giggly".into(),
            category: "effect".into(),
            form: None,
            method: None,
        },
    );
    check(
        fixture,
        "symptom tag",
        journal.get_tag(id("symptom_tag")).unwrap(),
        TagData {
            name: "jaw tension".into(),
            category: "symptom".into(),
            form: None,
            method: None,
        },
    );
    if let Some(subtype_tag) = &subtype_tag {
        check(
            fixture,
            "subtype tag",
            journal.get_tag(subtype_tag.clone()).unwrap(),
            TagData {
                name: "Lozenge".into(),
                category: "subtype".into(),
                form: Some("edible".into()),
                method: Some("tincture".into()),
            },
        );
    }
    let mut session = journal.get_session(id("session")).unwrap();
    let timeline = fixture.manifest["session_timeline"].as_bool() == Some(true);
    // The session lines' IDs are not in the manifest; they are whatever
    // the release made, so lines are compared without them, by method.
    if let Some(s) = &mut session {
        for line in &mut s.lines {
            assert!(
                line.line_id.take().is_some(),
                "{}: session line",
                fixture.name()
            );
        }
        s.lines.sort_by(|a, b| a.method.cmp(&b.method));
    }
    let mut lines = vec![ProductLineData {
        line_id: None,
        product: Some(id("product")),
        method: "edible".into(),
        dose_milli: Some(5_000),
        dose_unit: Some("mg".into()),
        purchase_id: Some(id("purchase")),
        purchase_line_id: Some(id("purchase_line")),
        shop: None,
    }];
    // Since 0.6.0, a patch too.
    if subtypes {
        lines.push(ProductLineData {
            line_id: None,
            product: None,
            method: "transdermal".into(),
            dose_milli: Some(10_000),
            dose_unit: Some("mg".into()),
            purchase_id: None,
            purchase_line_id: None,
            shop: None,
        });
    }
    check(
        fixture,
        "session",
        session,
        SessionData {
            when: at(1_760_000_000_000),
            lines,
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
                    tag: TagRefData::Custom { id: id("tag") },
                    value: 2,
                },
            ],
            onset_min: Some(60),
            peak_min: Some(120),
            duration_min: Some(240),
            mood_after: Some(5),
            // 4 at first, edited to 5.
            rating: Some(5),
            would_repeat: Some(true),
            spot: Some(id("spot")),
            symptoms: vec![
                SymptomData {
                    tag: builtin("sleep"),
                    result: "helped".into(),
                },
                SymptomData {
                    tag: TagRefData::Custom {
                        id: id("symptom_tag"),
                    },
                    result: "did_not_help".into(),
                },
            ],
            attachments: vec![id("attachment")],
            // Since 0.5.0; older fixtures have none.
            timeline_levels: if timeline {
                vec![
                    TimelineLevelData {
                        minute: 48,
                        level: 2,
                    },
                    TimelineLevelData {
                        minute: 96,
                        level: 4,
                    },
                ]
            } else {
                vec![]
            },
            timeline_updated_ms: timeline.then_some(1_760_000_600_000),
        },
    );
    check(
        fixture,
        "snippet",
        journal.get_snippet(id("snippet")).unwrap(),
        SnippetData {
            session: Some(id("session")),
            at: at(1_760_000_100_000),
            text: "so giggly 🙂".into(),
            tags: vec![TagProvenance {
                tag: TagRefData::Custom { id: id("tag") },
                provenance: "user".into(),
            }],
            added_while_locked: false,
        },
    );
    check(
        fixture,
        "captured note",
        journal.get_snippet(id("captured_note")).unwrap(),
        SnippetData {
            session: None,
            at: at(1_760_000_200_000),
            text: "a note written while locked".into(),
            tags: vec![],
            added_while_locked: true,
        },
    );
    assert_eq!(
        journal.locked_captures().unwrap(),
        vec![id("captured_note")]
    );
    assert_eq!(
        journal.hidden_data_points().unwrap(),
        vec!["terpenes".to_owned()]
    );
    // Before 0.4.0 there was no step; a journal without one reads as
    // Terpenes.
    let step = if parent_strain.is_some() {
        "detailed"
    } else {
        "terpenes"
    };
    assert_eq!(
        journal.strain_detail().unwrap(),
        step,
        "{}: strain detail",
        fixture.name()
    );

    // The indexes answer for the old records.
    let mut refs = journal.referencing(id("product")).unwrap();
    refs.sort();
    let mut expected = vec![id("session"), id("purchase")];
    expected.sort();
    assert_eq!(refs, expected, "{}: references", fixture.name());
    if let Some(parent) = &parent_strain {
        assert_eq!(
            journal.referencing(parent.clone()).unwrap(),
            vec![id("strain")],
            "{}: parent strain references",
            fixture.name()
        );
    }
    assert_eq!(
        journal.with_tag(builtin("relaxed")).unwrap(),
        vec![id("session")]
    );
    // 2025-10-09 in UTC-7 is day 20_370.
    let mut today = journal.on_days(20_370, 20_370).unwrap();
    today.sort();
    let mut expected = vec![id("session"), id("snippet"), id("captured_note")];
    expected.sort();
    assert_eq!(today, expected, "{}: day index", fixture.name());
}

/// Round-trips `journal` through this version's export and sync, and
/// checks nothing changed.
fn round_trip(fixture: &Fixture, journal: &Journal) {
    let before = contents(journal);

    let archive = journal
        .export_archive(fixture.bytes("keys/master_wrapped_by_recovery.bin"))
        .unwrap();
    let restored_dir = Device::new();
    let restored = restored_dir.fresh(fixture);
    import_all(&restored, read_export_archive(archive).unwrap().blobs);
    check_records(fixture, &restored);
    assert!(contents(&restored) == before, "{}: export", fixture.name());

    let synced_dir = Device::new();
    let synced = synced_dir.fresh(fixture);
    import_all(
        &synced,
        journal
            .export_sync_blobs()
            .unwrap()
            .into_iter()
            .map(|b| b.bytes),
    );
    check_records(fixture, &synced);
    assert!(contents(&synced) == before, "{}: sync", fixture.name());

    // Merging the new blobs back changes nothing.
    import_all(
        journal,
        synced
            .export_sync_blobs()
            .unwrap()
            .into_iter()
            .map(|b| b.bytes),
    );
    assert!(contents(journal) == before, "{}: re-import", fixture.name());
}

/// The database as the release left it on the phone: opened with this
/// version (which migrates it), every record and the device's own data
/// checked, a note captured before the upgrade merged, then round-tripped.
fn check_database(fixture: &Fixture) {
    let recovery_master = master_key_from_recovery(
        fixture.recovery_key(),
        fixture.bytes("keys/master_wrapped_by_recovery.bin"),
    )
    .unwrap();
    let passphrase_master = master_key_from_passphrase(
        fixture.text("passphrase"),
        PassphraseWrapped {
            params: fixture.bytes("keys/passphrase_params.bin"),
            wrapped_master_key: fixture.bytes("keys/passphrase_wrapped_master_key.bin"),
        },
    )
    .unwrap();
    assert_eq!(recovery_master, passphrase_master);

    let device = Device::new();
    copy_dir(&fixture.dir.join("database"), &device.dir);
    copy_dir(&fixture.dir.join("inbox"), &device.dir.join("inbox"));
    let journal = device.open(
        recovery_master,
        fixture.bytes("keys/wrapped_database_key.bin"),
    );
    check_records(fixture, &journal);
    assert_eq!(
        journal.read_attachment(fixture.id("attachment")).unwrap(),
        attachment_bytes()
    );
    assert_eq!(
        journal.maptiler_key().unwrap(),
        Some(fixture.text("maptiler_key"))
    );

    // The database and the release's export and sync blobs agree.
    let db_contents = contents(&journal);
    let from_export = Device::new();
    let exported = from_export.fresh(fixture);
    import_all(
        &exported,
        read_export_archive(fixture.bytes("export.terpoir"))
            .unwrap()
            .blobs,
    );
    assert!(
        contents(&exported) == db_contents,
        "{}: database and export differ",
        fixture.name()
    );
    // And the release's blobs change nothing in its own database.
    import_all(&journal, fixture.sync_blobs().into_iter().map(|(_, b)| b));
    assert!(contents(&journal) == db_contents, "{}", fixture.name());

    round_trip(fixture, &journal);

    // Closed and opened again: still the same journal.
    journal.lock();
    drop(journal);
    let journal = device.open(
        master_key_from_recovery(
            fixture.recovery_key(),
            fixture.bytes("keys/master_wrapped_by_recovery.bin"),
        )
        .unwrap(),
        fixture.bytes("keys/wrapped_database_key.bin"),
    );
    check_records(fixture, &journal);

    // A note sealed while locked, before the upgrade, is merged after it.
    assert_eq!(journal.merge_captures(device.path("inbox"), 1).unwrap(), 1);
    let captures = journal.locked_captures().unwrap();
    assert_eq!(captures.len(), 2);
    let waiting = captures
        .iter()
        .filter_map(|c| journal.get_snippet(c.clone()).unwrap())
        .find(|s| s.text == "a note waiting in the inbox");
    assert!(
        waiting.is_some_and(|s| s.at == at(1_760_000_300_000) && s.added_while_locked),
        "{}: the waiting capture",
        fixture.name()
    );

    // The migrated database still takes writes.
    journal
        .save_brand(None, BrandData { name: "New".into() }, 2)
        .unwrap();
    assert_eq!(journal.list(RecordType::Brand).unwrap().len(), 2);
}

/// The release's encrypted export, restored on a new device with this
/// version, then round-tripped.
fn check_export(fixture: &Fixture) {
    let archive = read_export_archive(fixture.bytes("export.terpoir")).unwrap();
    let checked = check_export_archive(
        fixture.recovery_key(),
        read_export_archive(fixture.bytes("export.terpoir")).unwrap(),
    )
    .unwrap();
    assert_eq!(checked.newer_records, 0);
    assert_eq!(checked.newer_format_blobs, 0);
    assert!(checked.records > 0);

    let device = Device::new();
    let journal = device.fresh(fixture);
    import_all(&journal, archive.blobs);
    check_records(fixture, &journal);
    // Attached files are not in exports yet ("Not yet" in the design doc).
    assert!(journal.read_attachment(fixture.id("attachment")).is_err());
    // This device's own data stays on that device.
    assert_eq!(journal.maptiler_key().unwrap(), None);
    round_trip(fixture, &journal);
}

/// The release's sync folder, merged into a new journal in either order,
/// then round-tripped.
fn check_sync_folder(fixture: &Fixture) {
    let blobs = fixture.sync_blobs();
    for name in &blobs {
        assert!(
            name.0.len() == 32 + ".terpoir".len() && name.0.ends_with(".terpoir"),
            "{}: sync/{}",
            fixture.name(),
            name.0
        );
    }

    let forward_dir = Device::new();
    let forward = forward_dir.fresh(fixture);
    import_all(&forward, blobs.iter().map(|(_, b)| b.clone()));
    check_records(fixture, &forward);

    // Oldest last: the edit and the deletion still win.
    let backward_dir = Device::new();
    let backward = backward_dir.fresh(fixture);
    import_all(&backward, blobs.iter().rev().map(|(_, b)| b.clone()));
    check_records(fixture, &backward);
    assert!(
        contents(&forward) == contents(&backward),
        "{}",
        fixture.name()
    );

    // The older blobs alone: the record as it was before the edit.
    let early_dir = Device::new();
    let early = early_dir.fresh(fixture);
    let names = fixture.manifest["sync_blobs_before_edit"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| format!("{}.terpoir", n.as_str().unwrap()))
        .collect::<Vec<_>>();
    assert!(!names.is_empty());
    import_all(
        &early,
        blobs
            .iter()
            .filter(|(n, _)| names.contains(n))
            .map(|(_, b)| b.clone()),
    );
    let session = early.get_session(fixture.id("session")).unwrap().unwrap();
    assert!(session.rating == Some(4), "{}", fixture.name());
    assert!(
        early
            .get_brand(fixture.id("deleted_brand"))
            .unwrap()
            .is_some()
    );
    // Then the newer ones bring it up to date.
    import_all(&early, blobs.iter().map(|(_, b)| b.clone()));
    check_records(fixture, &early);

    round_trip(fixture, &forward);
}

#[test]
fn every_released_database_opens_migrates_and_round_trips() {
    for dir in fixture_dirs() {
        check_database(&Fixture::load(dir));
    }
}

#[test]
fn every_released_export_restores_and_round_trips() {
    for dir in fixture_dirs() {
        check_export(&Fixture::load(dir));
    }
}

#[test]
fn every_released_sync_folder_merges_and_round_trips() {
    for dir in fixture_dirs() {
        check_sync_folder(&Fixture::load(dir));
    }
}
