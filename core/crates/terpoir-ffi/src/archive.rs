// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The encrypted export archive (docs/design-decisions.md, "Full export"):
//! the whole journal as one file that only the recovery key opens.
//!
//! It adds no cryptography. It holds the master key wrapped by the
//! recovery key (as the device already stores it) and the journal's full
//! set of sync blobs, which are encrypted under a key derived from the
//! master key, exactly as written to sync storage. Restoring unwraps the
//! master key with the recovery key and imports the blobs.
//!
//! Layout: the 8-byte magic `TERPOIR1`, a big-endian `u32` length and the
//! wrapped master key, a `u32` blob count, then each blob as a `u32`
//! length and its bytes.

use terpoir_crypto::{DatabaseKey, MasterKey, RecoveryKey, WrappedKey};
use terpoir_sync::inspect_blob;
use zeroize::Zeroizing;

use crate::{CoreError, Result};

const MAGIC: &[u8; 8] = b"TERPOIR1";

/// A parsed export archive.
#[derive(uniffi::Record)]
pub struct ExportArchive {
    /// The master key wrapped by the recovery key.
    pub master_wrapped_by_recovery: Vec<u8>,
    /// The journal's sync blobs, to import in any order.
    pub blobs: Vec<Vec<u8>>,
}

/// Keys for a journal restored from an export on a new device.
#[derive(uniffi::Record)]
pub struct RestoredKeys {
    /// The raw master key, to wrap with the platform keystore and then
    /// wipe. Never store it unwrapped.
    pub master_key: Vec<u8>,
    /// A new database key for this device, wrapped by the master key.
    pub wrapped_database_key: Vec<u8>,
}

/// What an export holds, checked before anything is changed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, uniffi::Record)]
pub struct ArchiveCheck {
    /// Records this version can read.
    pub records: u64,
    /// Records a newer app version wrote. Restoring or importing keeps
    /// them, and they appear after updating.
    pub newer_records: u64,
    /// Blobs in a format only a newer app can parse. If any, refuse the
    /// restore or import and ask the user to update first.
    pub newer_format_blobs: u64,
}

/// Inspects every blob of an archive under `master`.
pub(crate) fn check(master: &MasterKey, archive: &ExportArchive) -> Result<ArchiveCheck> {
    let n = |v: usize| u64::try_from(v).unwrap_or(u64::MAX);
    let mut result = ArchiveCheck {
        records: 0,
        newer_records: 0,
        newer_format_blobs: 0,
    };
    for blob in &archive.blobs {
        match inspect_blob(master, blob) {
            Ok(inspection) => {
                result.records = result.records.saturating_add(n(inspection.records));
                result.newer_records = result.newer_records.saturating_add(n(inspection.newer));
            }
            Err(terpoir_sync::Error::NewerFormat) => result.newer_format_blobs += 1,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(result)
}

/// Checks an export before restoring it on a new device: what it holds and
/// whether any of it needs a newer app. Nothing is changed.
///
/// # Errors
///
/// The recovery key's parse errors, `WrongKey` if it is not this export's,
/// or `InvalidSyncData` if a blob is damaged.
#[uniffi::export]
pub fn check_export_archive(recovery_key: String, archive: ExportArchive) -> Result<ArchiveCheck> {
    let text = Zeroizing::new(recovery_key);
    let recovery = RecoveryKey::parse(&text)?;
    let wrapped = WrappedKey::from_slice(&archive.master_wrapped_by_recovery)?;
    let master = recovery.key_encryption_key()?.unwrap_master_key(&wrapped)?;
    check(&master, &archive)
}

/// The key file a sync folder holds (docs/design-decisions.md, "Restore
/// from a sync folder"): an archive with the master key wrapped by the
/// recovery key and no blobs. With the folder's sync blobs it makes an
/// export, so a new device restores from the folder with the recovery key.
///
/// # Errors
///
/// `InvalidInput` if `master_wrapped_by_recovery` is not a wrapped key.
#[uniffi::export]
pub fn sync_folder_key_file(master_wrapped_by_recovery: Vec<u8>) -> Result<Vec<u8>> {
    WrappedKey::from_slice(&master_wrapped_by_recovery)?;
    write(&master_wrapped_by_recovery, &[])
}

/// For restoring from a sync folder: `archive` (the folder's key file and
/// every blob in it) without the blobs that don't open under its master
/// key, such as one another device is still writing or one from before a
/// master-key rotation. Folder sync retries those later. Blobs in a newer
/// app's format are kept, so `check_export_archive` still asks for an
/// update.
///
/// # Errors
///
/// The recovery key's parse errors, or `WrongKey` if it is not this
/// folder's.
#[uniffi::export]
pub fn readable_blobs(recovery_key: String, archive: ExportArchive) -> Result<ExportArchive> {
    let text = Zeroizing::new(recovery_key);
    let recovery = RecoveryKey::parse(&text)?;
    let wrapped = WrappedKey::from_slice(&archive.master_wrapped_by_recovery)?;
    let master = recovery.key_encryption_key()?.unwrap_master_key(&wrapped)?;
    let blobs = archive
        .blobs
        .into_iter()
        .filter(|blob| {
            matches!(
                inspect_blob(&master, blob),
                Ok(_) | Err(terpoir_sync::Error::NewerFormat)
            )
        })
        .collect();
    Ok(ExportArchive {
        master_wrapped_by_recovery: archive.master_wrapped_by_recovery,
        blobs,
    })
}

/// Builds an archive from the wrapped master key and the blobs.
pub(crate) fn write(master_wrapped_by_recovery: &[u8], blobs: &[Vec<u8>]) -> Result<Vec<u8>> {
    let len = |n: usize| u32::try_from(n).map_err(|_| CoreError::Internal);
    let total = MAGIC.len()
        + 8
        + master_wrapped_by_recovery.len()
        + blobs.iter().map(|b| 4 + b.len()).sum::<usize>();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&len(master_wrapped_by_recovery.len())?.to_be_bytes());
    out.extend_from_slice(master_wrapped_by_recovery);
    out.extend_from_slice(&len(blobs.len())?.to_be_bytes());
    for blob in blobs {
        out.extend_from_slice(&len(blob.len())?.to_be_bytes());
        out.extend_from_slice(blob);
    }
    Ok(out)
}

/// Reads an export archive. Nothing is decrypted.
///
/// # Errors
///
/// `InvalidSyncData` if it is not a Terpoir export or is truncated.
#[uniffi::export]
pub fn read_export_archive(archive: Vec<u8>) -> Result<ExportArchive> {
    let rest = archive
        .strip_prefix(MAGIC.as_slice())
        .ok_or(CoreError::InvalidSyncData)?;
    let mut cursor = Cursor(rest);
    let key_len = cursor.length()?;
    let master_wrapped_by_recovery = cursor.take(key_len)?.to_vec();
    let count = cursor.length()?;
    let mut blobs = Vec::new();
    for _ in 0..count {
        let n = cursor.length()?;
        blobs.push(cursor.take(n)?.to_vec());
    }
    if !cursor.0.is_empty() {
        return Err(CoreError::InvalidSyncData);
    }
    Ok(ExportArchive {
        master_wrapped_by_recovery,
        blobs,
    })
}

/// Reads an archive front to back.
struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.0.len() < n {
            return Err(CoreError::InvalidSyncData);
        }
        let (head, tail) = self.0.split_at(n);
        self.0 = tail;
        Ok(head)
    }

    fn length(&mut self) -> Result<usize> {
        let bytes: [u8; 4] = self
            .take(4)?
            .try_into()
            .map_err(|_| CoreError::InvalidSyncData)?;
        usize::try_from(u32::from_be_bytes(bytes)).map_err(|_| CoreError::InvalidSyncData)
    }
}

/// Unwraps the master key from an export with the recovery key, and makes
/// a new database key for this device.
///
/// # Errors
///
/// The recovery key's parse errors, or `WrongKey` if it is not this
/// export's.
#[uniffi::export]
pub fn keys_for_restore(
    recovery_key: String,
    master_wrapped_by_recovery: Vec<u8>,
) -> Result<RestoredKeys> {
    let text = Zeroizing::new(recovery_key);
    let recovery = RecoveryKey::parse(&text)?;
    let wrapped = WrappedKey::from_slice(&master_wrapped_by_recovery)?;
    let master: MasterKey = recovery.key_encryption_key()?.unwrap_master_key(&wrapped)?;
    let database = DatabaseKey::generate()?;
    Ok(RestoredKeys {
        master_key: master.expose_secret().to_vec(),
        wrapped_database_key: master.wrap_database_key(&database)?.as_bytes().to_vec(),
    })
}
