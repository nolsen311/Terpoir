// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Merging: per-field last-writer-wins, set union for records, and
//! ID-only deletion markers.

use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use terpoir_model::{Document, Field, RecordId, Value};

use crate::{Error, Result};

/// Merges `remote` into `local` field by field: for each field, the
/// write with the greater stamp wins (time, then counter, then device
/// ID). Returns whether `local` changed.
///
/// Both documents must have the same ID, kind and schema version: migrate
/// both to the current schema first. A document from a newer schema than
/// this app knows cannot be merged; keep it aside until the app updates.
///
/// # Errors
///
/// [`Error::Mismatch`] if ID, kind or schema version differ.
pub fn merge_document(local: &mut Document, remote: &Document) -> Result<bool> {
    if local.id() != remote.id()
        || local.kind() != remote.kind()
        || local.schema_version() != remote.schema_version()
    {
        return Err(Error::Mismatch);
    }
    let mut changed = false;
    for (key, theirs) in remote.fields() {
        let take = local.field(key).is_none_or(|ours| wins(theirs, ours));
        if take {
            local.insert_field(key.clone(), theirs.clone());
            changed = true;
        }
    }
    Ok(changed)
}

/// Whether `theirs` beats `ours`. Equal stamps only happen for the same
/// write seen twice; if the values ever differ anyway (corruption), the
/// larger encoding wins so every device still picks the same one.
fn wins(theirs: &Field, ours: &Field) -> bool {
    match theirs.stamp.cmp(&ours.stamp) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => value_bytes(&theirs.value) > value_bytes(&ours.value),
    }
}

fn value_bytes(value: &Value) -> Vec<u8> {
    postcard::to_allocvec(value).unwrap_or_default()
}

/// Where merged records go: the encrypted store in the app, a map in
/// tests.
pub trait Replica {
    /// Whether `id` has been deleted here (a deletion marker exists).
    ///
    /// # Errors
    ///
    /// Storage errors.
    fn is_deleted(&self, id: &RecordId) -> Result<bool>;

    /// The current document for `id`, if any.
    ///
    /// # Errors
    ///
    /// Storage errors.
    fn get(&self, id: &RecordId) -> Result<Option<Document>>;

    /// Saves a document (new or merged).
    ///
    /// # Errors
    ///
    /// Storage errors.
    fn put(&mut self, doc: Document) -> Result<()>;

    /// Deletes a record (crypto-erasing it) and records a deletion marker.
    ///
    /// # Errors
    ///
    /// Storage errors.
    fn delete(&mut self, id: &RecordId) -> Result<()>;
}

/// Applies changes received from another device: unknown records are
/// added (set union), known ones are merged per field, and deletions win
/// over any edits: a deleted record never comes back.
///
/// # Errors
///
/// Merge or storage errors. Changes applied before an error stay applied;
/// applying the same changes again is safe.
pub fn apply_changes(
    replica: &mut impl Replica,
    docs: impl IntoIterator<Item = Document>,
    deletions: impl IntoIterator<Item = RecordId>,
) -> Result<()> {
    for id in deletions {
        replica.delete(&id)?;
    }
    for doc in docs {
        let id = doc.id();
        if replica.is_deleted(&id)? {
            continue;
        }
        match replica.get(&id)? {
            None => replica.put(doc)?,
            Some(mut local) => {
                if merge_document(&mut local, &doc)? {
                    replica.put(local)?;
                }
            }
        }
    }
    Ok(())
}

/// Deletion markers every paired device has seen, which can therefore be
/// dropped. Only drop them once no sync blob still carries the deleted
/// records (rewrite or remove old blobs first), or an old blob could
/// bring a record back to a newly paired device.
#[must_use]
pub fn prunable_markers(
    markers: &BTreeSet<RecordId>,
    seen_by_each_device: &[&BTreeSet<RecordId>],
) -> BTreeSet<RecordId> {
    markers
        .iter()
        .filter(|id| seen_by_each_device.iter().all(|seen| seen.contains(id)))
        .copied()
        .collect()
}

/// An in-memory replica, for tests and simulations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryReplica {
    /// Live documents.
    pub docs: BTreeMap<RecordId, Document>,
    /// Deletion markers.
    pub deleted: BTreeSet<RecordId>,
}

impl Replica for MemoryReplica {
    fn is_deleted(&self, id: &RecordId) -> Result<bool> {
        Ok(self.deleted.contains(id))
    }

    fn get(&self, id: &RecordId) -> Result<Option<Document>> {
        Ok(self.docs.get(id).cloned())
    }

    fn put(&mut self, doc: Document) -> Result<()> {
        self.docs.insert(doc.id(), doc);
        Ok(())
    }

    fn delete(&mut self, id: &RecordId) -> Result<()> {
        self.docs.remove(id);
        self.deleted.insert(*id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terpoir_model::{DeviceId, Hlc, RecordKind, Stamp};

    fn stamp(ms: u64, device: u8) -> Stamp {
        Stamp {
            hlc: Hlc {
                wall_ms: ms,
                counter: 0,
            },
            device: DeviceId::from_bytes([device; 16]),
        }
    }

    fn doc() -> Document {
        Document::new(RecordId::from_bytes([1; 16]), RecordKind::Brand)
    }

    #[test]
    fn later_stamp_wins_per_field() {
        let mut a = doc();
        a.set("name", Value::Text("a".into()), stamp(2, 1));
        a.set("x", Value::Int(1), stamp(1, 1));
        let mut b = doc();
        b.set("name", Value::Text("b".into()), stamp(1, 2));
        b.set("x", Value::Int(2), stamp(3, 2));
        assert!(merge_document(&mut a, &b).unwrap());
        assert_eq!(a.value("name"), &Value::Text("a".into()));
        assert_eq!(a.value("x"), &Value::Int(2));
    }

    #[test]
    fn device_id_breaks_ties() {
        let mut a = doc();
        a.set("name", Value::Text("a".into()), stamp(5, 1));
        let mut b = doc();
        b.set("name", Value::Text("b".into()), stamp(5, 2));
        let mut a2 = a.clone();
        merge_document(&mut a2, &b).unwrap();
        let mut b2 = b.clone();
        merge_document(&mut b2, &a).unwrap();
        assert_eq!(a2, b2);
        assert_eq!(a2.value("name"), &Value::Text("b".into()));
    }

    #[test]
    fn merging_same_document_is_a_no_op() {
        let mut a = doc();
        a.set("name", Value::Text("a".into()), stamp(5, 1));
        let copy = a.clone();
        assert!(!merge_document(&mut a, &copy).unwrap());
    }

    #[test]
    fn mismatches_are_rejected() {
        let mut a = doc();
        let other = Document::new(RecordId::from_bytes([2; 16]), RecordKind::Brand);
        assert_eq!(merge_document(&mut a, &other).err(), Some(Error::Mismatch));
        let other = Document::new(RecordId::from_bytes([1; 16]), RecordKind::Tag);
        assert_eq!(merge_document(&mut a, &other).err(), Some(Error::Mismatch));
    }

    #[test]
    fn deletion_wins_over_later_edits() {
        let mut replica = MemoryReplica::default();
        let mut d = doc();
        d.set("name", Value::Text("a".into()), stamp(1, 1));
        apply_changes(&mut replica, [d.clone()], []).unwrap();
        apply_changes(&mut replica, [], [d.id()]).unwrap();
        d.set("name", Value::Text("edited later".into()), stamp(9, 2));
        apply_changes(&mut replica, [d.clone()], []).unwrap();
        assert!(replica.docs.is_empty());
        assert!(replica.deleted.contains(&d.id()));
    }

    #[test]
    fn markers_prune_only_when_seen_everywhere() {
        let a = RecordId::from_bytes([1; 16]);
        let b = RecordId::from_bytes([2; 16]);
        let markers: BTreeSet<_> = [a, b].into();
        let phone: BTreeSet<_> = [a, b].into();
        let tablet: BTreeSet<_> = [a].into();
        assert_eq!(prunable_markers(&markers, &[&phone, &tablet]), [a].into());
        assert_eq!(prunable_markers(&markers, &[]), markers);
    }
}
