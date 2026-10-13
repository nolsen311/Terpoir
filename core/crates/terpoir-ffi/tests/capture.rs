// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Write-only capture, exercised the way the Android app uses it: notes
//! sealed while the journal is locked, merged on unlock, then reviewed.

// Test code may unwrap.
#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use terpoir_ffi::{
    CoreError, Journal, LocalTimeData, NewKeys, capture_ready, create_keys, seal_capture,
};

struct Device {
    dir: PathBuf,
    keys: NewKeys,
}

impl Device {
    fn new() -> Self {
        let keys = create_keys().unwrap();
        let tag: String = keys.master_key[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let dir = std::env::temp_dir().join(format!("terpoir-capture-test-{tag}"));
        fs::create_dir_all(&dir).unwrap();
        Self { dir, keys }
    }

    fn open(&self) -> Arc<Journal> {
        Journal::open(
            self.path("journal.db"),
            self.path("files"),
            self.keys.master_key.clone(),
            self.keys.wrapped_database_key.clone(),
        )
        .unwrap()
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().into_owned()
    }

    fn inbox(&self) -> String {
        self.path("inbox")
    }

    fn inbox_files(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.dir.join("inbox"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn notes(&self) -> Vec<PathBuf> {
        self.inbox_files()
            .into_iter()
            .filter(|n| n.ends_with(".cap"))
            .map(|n| self.dir.join("inbox").join(n))
            .collect()
    }

    fn seal(&self, text: &str, unix_ms: i64) {
        seal_capture(
            self.inbox(),
            text.into(),
            LocalTimeData {
                unix_ms,
                utc_offset_min: 60,
            },
        )
        .unwrap();
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn texts(journal: &Journal, ids: &[String]) -> Vec<String> {
    ids.iter()
        .map(|id| journal.get_snippet(id.clone()).unwrap().unwrap().text)
        .collect()
}

#[test]
fn needs_one_unlock_before_capturing() {
    let device = Device::new();
    assert!(!capture_ready(device.inbox()));
    let at = LocalTimeData {
        unix_ms: 1,
        utc_offset_min: 0,
    };
    assert_eq!(
        seal_capture(device.inbox(), "note".into(), at),
        Err(CoreError::NotFound)
    );
    let journal = device.open();
    assert_eq!(journal.merge_captures(device.inbox(), 1).unwrap(), 0);
    assert!(capture_ready(device.inbox()));
    assert_eq!(device.inbox_files(), ["capture.pub"]);
    // Blank notes are refused.
    assert_eq!(
        seal_capture(device.inbox(), "  \n".into(), at),
        Err(CoreError::InvalidInput)
    );
}

#[test]
fn locked_notes_merge_on_unlock_and_wait_for_review() {
    let device = Device::new();
    let journal = device.open();
    journal.merge_captures(device.inbox(), 1).unwrap();
    journal.lock();
    assert_eq!(journal.locked_captures(), Err(CoreError::Locked));

    device.seal("second note", 2_000);
    device.seal("first note, piney", 1_000);
    device.seal("third note", 3_000);
    let notes = device.notes();
    assert_eq!(notes.len(), 3);
    for note in &notes {
        let bytes = fs::read(note).unwrap();
        assert_eq!(bytes.len() % 512, 81, "padded payload plus overhead");
        assert!(!bytes.windows(5).any(|w| w == b"piney" || w == b"note,"));
    }

    let journal = device.open();
    assert_eq!(journal.merge_captures(device.inbox(), 10).unwrap(), 3);
    assert_eq!(device.inbox_files(), ["capture.pub"]);
    let pending = journal.locked_captures().unwrap();
    assert_eq!(
        texts(&journal, &pending),
        ["first note, piney", "second note", "third note"]
    );
    let first = journal.get_snippet(pending[0].clone()).unwrap().unwrap();
    assert!(first.added_while_locked);
    assert_eq!(first.session, None);
    assert_eq!(first.at.unix_ms, 1_000);
    assert_eq!(first.at.utc_offset_min, 60);
    assert!(first.tags.is_empty());
    assert_eq!(journal.merge_captures(device.inbox(), 11).unwrap(), 0);

    // Keep the first, discard the second.
    let mut kept = first.clone();
    kept.added_while_locked = false;
    journal
        .save_snippet(Some(pending[0].clone()), kept, 12)
        .unwrap();
    assert!(journal.delete_record(pending[1].clone()).unwrap());
    assert_eq!(journal.locked_captures().unwrap(), [pending[2].clone()]);
    assert_eq!(
        journal
            .get_snippet(pending[0].clone())
            .unwrap()
            .unwrap()
            .text,
        "first note, piney"
    );
}

#[test]
fn a_note_is_never_added_twice() {
    let device = Device::new();
    let journal = device.open();
    journal.merge_captures(device.inbox(), 1).unwrap();
    device.seal("only once", 1_000);
    let note = device.notes().remove(0);
    let copy = fs::read(&note).unwrap();
    assert_eq!(journal.merge_captures(device.inbox(), 2).unwrap(), 1);
    // As if the app stopped after adding the note but before deleting it.
    fs::write(&note, &copy).unwrap();
    assert_eq!(journal.merge_captures(device.inbox(), 3).unwrap(), 0);
    assert!(device.notes().is_empty());
    // Discarded, then the file comes back: still not re-added.
    let id = journal.locked_captures().unwrap().remove(0);
    journal.delete_record(id).unwrap();
    fs::write(&note, &copy).unwrap();
    assert_eq!(journal.merge_captures(device.inbox(), 4).unwrap(), 0);
    assert!(journal.locked_captures().unwrap().is_empty());
}

#[test]
fn bad_files_are_removed_and_newer_ones_kept() {
    let device = Device::new();
    let journal = device.open();
    journal.merge_captures(device.inbox(), 1).unwrap();
    device.seal("renamed", 1_000);
    let note = device.notes().remove(0);
    let inbox = device.dir.join("inbox");
    // Bound to its record ID: under another name it does not open.
    fs::rename(&note, inbox.join(format!("{}.cap", "ab".repeat(16)))).unwrap();
    // From a newer app version: kept for after an update.
    let newer = inbox.join(format!("{}.cap", "cd".repeat(16)));
    fs::write(&newer, [2u8; 200]).unwrap();
    // Empty: never readable.
    fs::write(inbox.join(format!("{}.cap", "12".repeat(16))), b"").unwrap();
    // Not a record ID, and an interrupted write.
    fs::write(inbox.join("junk.cap"), b"x").unwrap();
    fs::write(inbox.join(format!("{}.tmp", "ef".repeat(16))), b"x").unwrap();

    assert_eq!(journal.merge_captures(device.inbox(), 2).unwrap(), 0);
    assert!(journal.locked_captures().unwrap().is_empty());
    assert_eq!(
        device.inbox_files(),
        ["capture.pub".to_owned(), format!("{}.cap", "cd".repeat(16))]
    );
}

#[test]
fn notes_survive_master_key_rotation() {
    let device = Device::new();
    let journal = device.open();
    journal.merge_captures(device.inbox(), 1).unwrap();
    let old_public = fs::read(device.dir.join("inbox/capture.pub")).unwrap();
    device.seal("sealed to the old key", 1_000);

    journal
        .rotate_master_key(
            device.keys.recovery_key.clone(),
            device.keys.master_wrapped_by_recovery.clone(),
        )
        .unwrap();
    // Sealed after rotation but before the app refreshes the inbox.
    device.seal("also to the old key", 2_000);
    assert_eq!(journal.merge_captures(device.inbox(), 2).unwrap(), 2);
    let new_public = fs::read(device.dir.join("inbox/capture.pub")).unwrap();
    assert_ne!(old_public, new_public);
    device.seal("to the new key", 3_000);
    assert_eq!(journal.merge_captures(device.inbox(), 3).unwrap(), 1);
    let pending = journal.locked_captures().unwrap();
    assert_eq!(
        texts(&journal, &pending),
        [
            "sealed to the old key",
            "also to the old key",
            "to the new key"
        ]
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn every_locked_note_arrives_intact(
        notes in proptest::collection::vec("[^\\s][\\PC]{0,300}", 1..6),
    ) {
        let device = Device::new();
        let journal = device.open();
        journal.merge_captures(device.inbox(), 1).unwrap();
        journal.lock();
        for (i, text) in notes.iter().enumerate() {
            device.seal(text, i64::try_from(i).unwrap());
        }
        let journal = device.open();
        let added = journal.merge_captures(device.inbox(), 2).unwrap();
        prop_assert_eq!(added as usize, notes.len());
        let pending = journal.locked_captures().unwrap();
        prop_assert_eq!(texts(&journal, &pending), notes);
        prop_assert_eq!(device.inbox_files(), vec!["capture.pub".to_owned()]);
    }
}
