// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Sync against the encrypted store: writing the journal out as blobs and
//! applying blobs from other devices.

use std::collections::BTreeMap;
use terpoir_crypto::MasterKey;
use terpoir_model::{DeviceId, Document, RecordId, migrate};
use terpoir_store::{PendingRecord, SealedRecord, Store};

use crate::{
    Blob, Clock, Error, Replica, Result, apply_changes, container_key, decode_blob, encode_blob,
    opaque_name,
};

/// Target size of one blob's contents before padding. Large journals are
/// split across several blobs.
const BATCH_BYTES: usize = 4 * 1024 * 1024;
/// Deletion markers per blob.
const BATCH_DELETIONS: usize = 65_536;

/// The encrypted store as a sync [`Replica`].
///
/// New records are stored exactly as received (same wrapped key and
/// ciphertext), so a record keeps one key on every device and its
/// attached files open everywhere. Existing records are merged and
/// re-sealed under their existing key.
pub struct StoreReplica<'a> {
    store: &'a mut Store,
    master: &'a MasterKey,
    incoming: BTreeMap<RecordId, SealedRecord>,
}

impl<'a> StoreReplica<'a> {
    /// Wraps a store for applying changes.
    pub fn new(store: &'a mut Store, master: &'a MasterKey) -> Self {
        Self {
            store,
            master,
            incoming: BTreeMap::new(),
        }
    }
}

impl core::fmt::Debug for StoreReplica<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("StoreReplica(..)")
    }
}

impl Replica for StoreReplica<'_> {
    fn is_deleted(&self, id: &RecordId) -> Result<bool> {
        Ok(self.store.is_deleted(id)?)
    }

    fn get(&self, id: &RecordId) -> Result<Option<Document>> {
        self.store
            .get(self.master, id)?
            .map(|doc| migrate(doc).map_err(Error::Model))
            .transpose()
    }

    fn put(&mut self, doc: Document) -> Result<()> {
        let id = doc.id();
        match self.incoming.remove(&id) {
            Some(sealed) if !self.store.contains(&id)? => {
                self.store.put_sealed(self.master, &sealed)?;
            }
            _ => self.store.put(self.master, &doc)?,
        }
        Ok(())
    }

    fn delete(&mut self, id: &RecordId) -> Result<()> {
        self.store.delete(id)?;
        Ok(())
    }
}

/// What [`import_blob`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportSummary {
    /// The device that wrote the blob.
    pub device: DeviceId,
    /// That device's sequence number for the blob.
    pub sequence: u64,
    /// Records added or merged.
    pub records: usize,
    /// Deletion markers applied.
    pub deletions: usize,
    /// Records a newer app version wrote. They are kept sealed, carried
    /// in exports, and applied by [`apply_pending`] after updating.
    pub deferred: usize,
}

/// Opens and migrates a sealed record, or `None` if a newer app version
/// wrote it.
fn readable(sealed: &SealedRecord, master: &MasterKey) -> Result<Option<Document>> {
    let doc = match sealed.open(master) {
        Ok(doc) => doc,
        Err(terpoir_store::Error::Model(terpoir_model::Error::NewerSchema)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match migrate(doc) {
        Ok(doc) => Ok(Some(doc)),
        Err(terpoir_model::Error::NewerSchema) => Ok(None),
        Err(e) => Err(Error::Model(e)),
    }
}

/// What a blob holds, from [`inspect_blob`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobInspection {
    /// Records this version can read.
    pub records: usize,
    /// Records a newer app version wrote; importing keeps them for later.
    pub newer: usize,
}

/// Checks a blob without changing anything, so a restore or import can say
/// up front what needs a newer app.
///
/// # Errors
///
/// [`Error::NewerFormat`] if a newer app wrote it in a format this version
/// can't parse; [`Error::Blob`] if it does not decrypt; crypto or model
/// errors for a damaged record.
pub fn inspect_blob(master: &MasterKey, bytes: &[u8]) -> Result<BlobInspection> {
    let blob = decode_blob(&container_key(master)?, bytes)?;
    let mut inspection = BlobInspection {
        records: 0,
        newer: blob.pending.len(),
    };
    for sealed in &blob.records {
        match readable(sealed, master)? {
            Some(_) => inspection.records += 1,
            None => inspection.newer += 1,
        }
    }
    Ok(inspection)
}

/// Decrypts a blob from another device and applies it to the store.
/// `clock` is advanced past every stamp seen so later local edits are
/// ordered after them; persist it afterwards.
///
/// # Errors
///
/// [`Error::Blob`] if the blob does not decrypt; [`Error::NewerFormat`] if
/// a newer app wrote it in a format this version can't parse; crypto,
/// model or store errors otherwise. Applying the same blob again is
/// harmless.
///
/// Records this version can't read (an unknown kind or a newer schema)
/// are kept sealed, exactly as received, unless they were deleted.
pub fn import_blob(
    store: &mut Store,
    master: &MasterKey,
    clock: &mut Clock,
    bytes: &[u8],
    now_ms: u64,
) -> Result<ImportSummary> {
    let blob = decode_blob(&container_key(master)?, bytes)?;
    let mut replica = StoreReplica::new(store, master);
    let mut docs = Vec::with_capacity(blob.records.len());
    let mut kept = blob.pending;
    for sealed in blob.records {
        match readable(&sealed, master)? {
            Some(doc) => {
                if let Some(stamp) = doc.latest_stamp() {
                    clock.observe(stamp.hlc, now_ms);
                }
                replica.incoming.insert(doc.id(), sealed);
                docs.push(doc);
            }
            None => kept.push(PendingRecord::from(&sealed)),
        }
    }
    let summary = ImportSummary {
        device: blob.device,
        sequence: blob.sequence,
        records: docs.len(),
        deletions: blob.deletions.len(),
        deferred: kept.len(),
    };
    apply_changes(&mut replica, docs, blob.deletions)?;
    for record in &kept {
        if !store.is_deleted(&record.id)? {
            store.put_pending(record)?;
        }
    }
    Ok(summary)
}

/// Applies the records kept for a newer app version that this version
/// can now read, merging them like records from a sync blob, and stops
/// keeping them. Call after opening the store; records still unreadable
/// stay kept. Returns how many were applied.
///
/// # Errors
///
/// Store or crypto errors.
pub fn apply_pending(
    store: &mut Store,
    master: &MasterKey,
    clock: &mut Clock,
    now_ms: u64,
) -> Result<usize> {
    let pending = store.pending()?;
    let mut replica = StoreReplica::new(store, master);
    let mut docs = Vec::new();
    for sealed in pending.iter().filter_map(PendingRecord::to_sealed) {
        let Ok(Some(doc)) = readable(&sealed, master) else {
            continue;
        };
        if let Some(stamp) = doc.latest_stamp() {
            clock.observe(stamp.hlc, now_ms);
        }
        replica.incoming.insert(doc.id(), sealed);
        docs.push(doc);
    }
    let applied: Vec<RecordId> = docs.iter().map(Document::id).collect();
    apply_changes(&mut replica, docs, [])?;
    for id in &applied {
        store.remove_pending(id)?;
    }
    Ok(applied.len())
}

/// Writes the whole journal (every sealed record, record kept for a newer
/// app version, and deletion marker) as
/// encrypted, padded blobs, numbered from `first_sequence`. Returns
/// `(opaque name, bytes)` pairs to upload.
///
/// Exporting full state is always correct (merging is idempotent);
/// incremental export is an optimization for later.
///
/// # Errors
///
/// Store or crypto errors.
pub fn export_blobs(
    store: &Store,
    master: &MasterKey,
    device: DeviceId,
    first_sequence: u64,
) -> Result<Vec<(String, Vec<u8>)>> {
    let key = container_key(master)?;
    let mut blobs = Vec::new();
    let mut sequence = first_sequence;
    let mut current = Blob {
        device,
        sequence,
        records: Vec::new(),
        pending: Vec::new(),
        deletions: Vec::new(),
    };
    let mut current_bytes = 0usize;
    let mut flush = |blob: &mut Blob, size: &mut usize, seq: &mut u64| -> Result<()> {
        if blob.records.is_empty() && blob.pending.is_empty() && blob.deletions.is_empty() {
            return Ok(());
        }
        blobs.push((opaque_name()?, encode_blob(&key, blob)?));
        *seq += 1;
        *blob = Blob {
            device,
            sequence: *seq,
            records: Vec::new(),
            pending: Vec::new(),
            deletions: Vec::new(),
        };
        *size = 0;
        Ok(())
    };
    for chunk in store.deletion_markers()?.chunks(BATCH_DELETIONS) {
        current.deletions.extend_from_slice(chunk);
        flush(&mut current, &mut current_bytes, &mut sequence)?;
    }
    for id in store.all_ids()? {
        let Some(sealed) = store.sealed(&id)? else {
            continue;
        };
        let size = sealed.ciphertext.len() + 128;
        if current_bytes + size > BATCH_BYTES {
            flush(&mut current, &mut current_bytes, &mut sequence)?;
        }
        current_bytes += size;
        current.records.push(sealed);
    }
    for record in store.pending()? {
        let size = record.ciphertext.len() + 128;
        if current_bytes + size > BATCH_BYTES {
            flush(&mut current, &mut current_bytes, &mut sequence)?;
        }
        current_bytes += size;
        current.pending.push(record);
    }
    flush(&mut current, &mut current_bytes, &mut sequence)?;
    Ok(blobs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::{AAD, FORMAT_VERSION};

    #[test]
    fn inspecting_reports_newer_formats_without_changing_anything() {
        let master = MasterKey::generate().unwrap();
        let key = container_key(&master).unwrap();
        let mut plain = vec![FORMAT_VERSION + 1, 0, 0, 0, 0];
        plain.resize(4096, 0);
        let newer = key.seal(AAD, &plain).unwrap();
        assert_eq!(inspect_blob(&master, &newer), Err(Error::NewerFormat));
        let other = MasterKey::generate().unwrap();
        assert_eq!(inspect_blob(&other, &newer), Err(Error::Blob));
    }
}
