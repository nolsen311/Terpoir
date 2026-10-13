// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The sync blob: what Terpoir writes to the user's own storage.
//!
//! A blob batches sealed records (exactly as stored locally; each is
//! already encrypted under its own record key) and deletion markers (IDs
//! only). The batch is padded to a size bucket and encrypted again under
//! a container key derived from the master key, so the storage provider
//! sees neither how many records changed, nor their IDs or kinds, nor
//! precise sizes. Blobs are stored under random names that say nothing
//! about their contents or order.
//!
//! Plaintext layout before encryption:
//!
//! ```text
//! version (1) || body length (4, big-endian) || body (postcard) || zero padding
//! ```

use serde::{Deserialize, Serialize};
use terpoir_crypto::{MasterKey, SubKey, WrappedKey};
use terpoir_model::{DeviceId, RecordId};
use terpoir_store::{PendingRecord, SealedRecord};
use zeroize::Zeroizing;

use crate::{Error, Result};

const FORMAT_VERSION: u8 = 1;
const HEADER_LEN: usize = 5;
const AAD: &[u8] = b"terpoir/sync-blob";
const MIN_BUCKET: usize = 4 * 1024;
const LARGE_STEP: usize = 1024 * 1024;

/// Largest padded blob, in bytes. Larger batches must be split.
pub const MAX_BLOB_LEN: usize = 64 * 1024 * 1024;

/// One batch of changes from one device.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Blob {
    /// The device that wrote it.
    pub device: DeviceId,
    /// That device's batch counter, so readers can tell which blobs they
    /// have already applied.
    pub sequence: u64,
    /// Records added or changed.
    pub records: Vec<SealedRecord>,
    /// Records of a kind this version doesn't know, from a newer app.
    /// Carried through unchanged; see [`crate::apply_pending`].
    pub pending: Vec<PendingRecord>,
    /// IDs of deleted records.
    pub deletions: Vec<RecordId>,
}

/// A sealed record on the wire: ID, kind code, wrapped key, ciphertext.
type WireRecord = ([u8; 16], u8, Vec<u8>, Vec<u8>);

#[derive(Serialize, Deserialize)]
struct Wire {
    device: DeviceId,
    sequence: u64,
    records: Vec<WireRecord>,
    deletions: Vec<[u8; 16]>,
}

/// The key that encrypts blobs, derived from the master key. Rotating the
/// master key changes it, so blobs must be rewritten after rotation.
///
/// # Errors
///
/// [`Error::Crypto`] (not expected in practice).
pub fn container_key(master: &MasterKey) -> Result<SubKey> {
    Ok(master.derive_subkey("sync/container")?)
}

/// A random file name for a new blob: 32 hex characters, no extension.
///
/// # Errors
///
/// [`Error::Crypto`] if the random number generator fails.
pub fn opaque_name() -> Result<String> {
    Ok(RecordId::random()
        .map_err(|_| Error::Crypto(terpoir_crypto::Error::Random))?
        .to_hex())
}

/// The padded plaintext size for `len` bytes: at least 4 KiB, then the
/// next power of two up to 1 MiB, then the next multiple of 1 MiB.
#[must_use]
pub fn padded_len(len: usize) -> usize {
    if len <= MIN_BUCKET {
        MIN_BUCKET
    } else if len <= LARGE_STEP {
        len.next_power_of_two()
    } else {
        len.div_ceil(LARGE_STEP).saturating_mul(LARGE_STEP)
    }
}

/// Pads and encrypts a blob.
///
/// # Errors
///
/// [`Error::Blob`] if it would exceed [`MAX_BLOB_LEN`]; [`Error::Crypto`]
/// otherwise.
pub fn encode_blob(key: &SubKey, blob: &Blob) -> Result<Vec<u8>> {
    let wire = Wire {
        device: blob.device,
        sequence: blob.sequence,
        records: blob
            .records
            .iter()
            .map(|r| {
                (
                    *r.id.as_bytes(),
                    r.kind.code(),
                    r.wrapped_key.as_bytes().to_vec(),
                    r.ciphertext.clone(),
                )
            })
            .chain(blob.pending.iter().map(|r| {
                (
                    *r.id.as_bytes(),
                    r.kind_code,
                    r.wrapped_key.as_bytes().to_vec(),
                    r.ciphertext.clone(),
                )
            }))
            .collect(),
        deletions: blob.deletions.iter().map(|id| *id.as_bytes()).collect(),
    };
    let body = Zeroizing::new(postcard::to_allocvec(&wire).map_err(|_| Error::Blob)?);
    let body_len = u32::try_from(body.len()).map_err(|_| Error::Blob)?;
    let total = padded_len(HEADER_LEN + body.len());
    if total > MAX_BLOB_LEN {
        return Err(Error::Blob);
    }
    let mut plain = Zeroizing::new(Vec::with_capacity(total));
    plain.push(FORMAT_VERSION);
    plain.extend_from_slice(&body_len.to_be_bytes());
    plain.extend_from_slice(&body);
    plain.resize(total, 0);
    Ok(key.seal(AAD, &plain)?)
}

/// Decrypts and parses a blob. The records inside are still sealed; open
/// them with the master key (see `terpoir_store::SealedRecord::open`).
///
/// # Errors
///
/// [`Error::NewerFormat`] if a newer app wrote it in a format this version
/// can't parse; [`Error::Blob`] for anything malformed, oversized, or not
/// decrypting under `key`.
pub fn decode_blob(key: &SubKey, bytes: &[u8]) -> Result<Blob> {
    if bytes.len() > MAX_BLOB_LEN + terpoir_crypto::SEAL_OVERHEAD {
        return Err(Error::Blob);
    }
    let plain = key.open(AAD, bytes).map_err(|_| Error::Blob)?;
    let (header, rest) = plain.split_at_checked(HEADER_LEN).ok_or(Error::Blob)?;
    if header[0] > FORMAT_VERSION {
        return Err(Error::NewerFormat);
    }
    if header[0] != FORMAT_VERSION {
        return Err(Error::Blob);
    }
    let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]);
    let len = usize::try_from(len).map_err(|_| Error::Blob)?;
    let (body, padding) = rest.split_at_checked(len).ok_or(Error::Blob)?;
    if padding.iter().any(|&b| b != 0) {
        return Err(Error::Blob);
    }
    let (wire, extra): (Wire, _) = postcard::take_from_bytes(body).map_err(|_| Error::Blob)?;
    if !extra.is_empty() {
        return Err(Error::Blob);
    }
    let mut records = Vec::new();
    let mut pending = Vec::new();
    for (id, kind_code, wrapped, ciphertext) in wire.records {
        let record = PendingRecord {
            id: RecordId::from_bytes(id),
            kind_code,
            wrapped_key: WrappedKey::from_slice(&wrapped).map_err(|_| Error::Blob)?,
            ciphertext,
        };
        match record.to_sealed() {
            Some(sealed) => records.push(sealed),
            None => pending.push(record),
        }
    }
    Ok(Blob {
        device: wire.device,
        sequence: wire.sequence,
        records,
        pending,
        deletions: wire
            .deletions
            .into_iter()
            .map(RecordId::from_bytes)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use terpoir_crypto::RecordKey;
    use terpoir_model::RecordKind;

    fn sealed(master: &MasterKey, n: usize) -> SealedRecord {
        let id = RecordId::random().unwrap();
        let key = RecordKey::generate().unwrap();
        SealedRecord {
            id,
            kind: RecordKind::Session,
            wrapped_key: master.wrap_record_key(&key, &id).unwrap(),
            ciphertext: key.seal(b"", &vec![7u8; n]).unwrap(),
        }
    }

    fn blob(master: &MasterKey, records: usize, size: usize) -> Blob {
        Blob {
            device: DeviceId::from_bytes([3; 16]),
            sequence: 42,
            records: (0..records).map(|_| sealed(master, size)).collect(),
            pending: Vec::new(),
            deletions: vec![RecordId::random().unwrap()],
        }
    }

    #[test]
    fn round_trip() {
        let master = MasterKey::generate().unwrap();
        let key = container_key(&master).unwrap();
        let b = blob(&master, 3, 100);
        let bytes = encode_blob(&key, &b).unwrap();
        assert_eq!(decode_blob(&key, &bytes).unwrap(), b);
    }

    #[test]
    fn sizes_are_bucketed() {
        assert_eq!(padded_len(0), 4096);
        assert_eq!(padded_len(4096), 4096);
        assert_eq!(padded_len(4097), 8192);
        assert_eq!(padded_len(700_000), 1 << 20);
        assert_eq!(padded_len((1 << 20) + 1), 2 << 20);
        let master = MasterKey::generate().unwrap();
        let key = container_key(&master).unwrap();
        let small = encode_blob(&key, &blob(&master, 1, 10)).unwrap();
        let bigger = encode_blob(&key, &blob(&master, 5, 300)).unwrap();
        assert_eq!(small.len(), bigger.len());
        assert_eq!(small.len(), 4096 + terpoir_crypto::SEAL_OVERHEAD);
    }

    #[test]
    fn wrong_key_and_tampering_are_rejected() {
        let master = MasterKey::generate().unwrap();
        let key = container_key(&master).unwrap();
        let bytes = encode_blob(&key, &blob(&master, 1, 10)).unwrap();
        let other = container_key(&MasterKey::generate().unwrap()).unwrap();
        assert_eq!(decode_blob(&other, &bytes), Err(Error::Blob));
        let mut tampered = bytes.clone();
        tampered[100] ^= 1;
        assert_eq!(decode_blob(&key, &tampered), Err(Error::Blob));
        assert_eq!(decode_blob(&key, &bytes[..10]), Err(Error::Blob));
    }

    #[test]
    fn nonzero_padding_or_bad_length_is_rejected() {
        let master = MasterKey::generate().unwrap();
        let key = container_key(&master).unwrap();
        let mut plain = vec![FORMAT_VERSION, 0, 0, 0xff, 0xff];
        plain.resize(4096, 0);
        assert_eq!(
            decode_blob(&key, &key.seal(AAD, &plain).unwrap()),
            Err(Error::Blob)
        );
        let mut plain = vec![FORMAT_VERSION, 0, 0, 0, 0];
        plain.resize(4096, 0);
        plain[4095] = 1;
        assert_eq!(
            decode_blob(&key, &key.seal(AAD, &plain).unwrap()),
            Err(Error::Blob)
        );
    }

    #[test]
    fn unknown_kinds_are_kept_and_newer_formats_reported() {
        let master = MasterKey::generate().unwrap();
        let key = container_key(&master).unwrap();
        let mut b = blob(&master, 2, 10);
        let mut unknown = PendingRecord::from(&b.records[0]);
        unknown.kind_code = 200;
        b.pending.push(unknown);
        let bytes = encode_blob(&key, &b).unwrap();
        assert_eq!(decode_blob(&key, &bytes).unwrap(), b);

        let mut plain = vec![FORMAT_VERSION + 1, 0, 0, 0, 0];
        plain.resize(4096, 0);
        assert_eq!(
            decode_blob(&key, &key.seal(AAD, &plain).unwrap()),
            Err(Error::NewerFormat)
        );
    }

    #[test]
    fn names_are_random_hex() {
        let a = opaque_name().unwrap();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, opaque_name().unwrap());
    }
}
