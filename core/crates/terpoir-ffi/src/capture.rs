// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Write-only capture: notes written while the journal is locked.
//!
//! The inbox is a directory the app chooses (app-private, excluded from
//! backups). It holds:
//!
//! - `capture.pub`: `version (1) || capture public key (32)`. Not secret.
//!   Written by [`Journal::merge_captures`] on every unlock.
//! - `<record ID in hex>.cap`: one sealed note each, written by
//!   [`seal_capture`] without the master key.
//!
//! A sealed note is the capture public key's HPKE seal (see
//! `terpoir_crypto::CapturePublicKey`) of `version (1) || unix_ms (8) ||
//! utc_offset_min (2) || text length (4) || UTF-8 text || zero padding`,
//! padded to a multiple of [`PAD_TO`] so its size gives away only a rough
//! length. The associated data is the record ID from the file name, so a
//! file renamed to another ID does not open.
//!
//! On unlock, each note becomes a standalone stream snippet with that
//! record ID, marked "added while locked" until the user keeps or
//! discards it, and its file is overwritten and deleted. A crash between
//! the two steps is harmless: a note whose ID is already in the journal
//! is not added twice.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use terpoir_crypto::{CaptureKey, CapturePublicKey, MasterKey, RecordId, WrappedKey};
use terpoir_model::{Document, LocalTime, Record, Snippet, migrate};
use zeroize::Zeroizing;

use crate::journal::{Journal, Unlocked};
use crate::records::LocalTimeData;
use crate::{CoreError, Result};

/// The current capture key pair: `public key || wrapped private key`.
pub(crate) const META_CURRENT: &str = "capture_key";
/// The pair before the last master-key rotation, kept so notes sealed to
/// it in between still open.
pub(crate) const META_PREVIOUS: &str = "capture_key_previous";

const PUBLIC_FILE: &str = "capture.pub";
const EXTENSION: &str = "cap";
const TEMP_EXTENSION: &str = "tmp";
const FILE_VERSION: u8 = 1;
const PAYLOAD_VERSION: u8 = 1;
const HEADER_LEN: usize = 1 + 8 + 2 + 4;
/// Notes are padded to a multiple of this many bytes.
pub(crate) const PAD_TO: usize = 512;
/// Longest note accepted, in bytes of UTF-8.
pub(crate) const MAX_TEXT_LEN: usize = 256 * 1024;

pub(crate) fn encode_payload(at: LocalTime, text: &str) -> Result<Zeroizing<Vec<u8>>> {
    if text.len() > MAX_TEXT_LEN {
        return Err(CoreError::InvalidInput);
    }
    let len = u32::try_from(text.len()).map_err(|_| CoreError::InvalidInput)?;
    let padded = (HEADER_LEN + text.len()).div_ceil(PAD_TO) * PAD_TO;
    let mut out = Zeroizing::new(Vec::with_capacity(padded));
    out.push(PAYLOAD_VERSION);
    out.extend_from_slice(&at.unix_ms.to_be_bytes());
    out.extend_from_slice(&at.utc_offset_min.to_be_bytes());
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(text.as_bytes());
    out.resize(padded, 0);
    Ok(out)
}

pub(crate) fn decode_payload(bytes: &[u8]) -> Result<(LocalTime, String)> {
    if bytes.len() < HEADER_LEN || !bytes.len().is_multiple_of(PAD_TO) {
        return Err(CoreError::InvalidInput);
    }
    if bytes[0] != PAYLOAD_VERSION {
        return Err(CoreError::NewerVersion);
    }
    let field = |range: core::ops::Range<usize>| bytes.get(range).ok_or(CoreError::InvalidInput);
    let unix_ms = i64::from_be_bytes(
        field(1..9)?
            .try_into()
            .map_err(|_| CoreError::InvalidInput)?,
    );
    let utc_offset_min = i16::from_be_bytes(
        field(9..11)?
            .try_into()
            .map_err(|_| CoreError::InvalidInput)?,
    );
    let len = u32::from_be_bytes(
        field(11..15)?
            .try_into()
            .map_err(|_| CoreError::InvalidInput)?,
    );
    let len = usize::try_from(len).map_err(|_| CoreError::InvalidInput)?;
    let end = HEADER_LEN.checked_add(len).ok_or(CoreError::InvalidInput)?;
    let text = field(HEADER_LEN..end)?;
    if bytes[end..].iter().any(|&b| b != 0) {
        return Err(CoreError::InvalidInput);
    }
    let text = String::from_utf8(text.to_vec()).map_err(|_| CoreError::InvalidInput)?;
    Ok((
        LocalTime {
            unix_ms,
            utc_offset_min,
        },
        text,
    ))
}

/// Writes `bytes` to `path` so that readers see the old file or the whole
/// new one, never a partial write.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(TEMP_EXTENSION);
    let mut file = File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, path)?;
    if let Some(dir) = path.parent() {
        // Persist the rename. Not every platform lets a directory be opened.
        if let Ok(dir) = File::open(dir) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

/// Overwrites a file with zeros, then deletes it. Flash storage may keep
/// old blocks, so this is best effort; the note is also encrypted to a
/// key only the master key opens.
pub(crate) fn wipe_file(path: &Path) -> Result<()> {
    if let Ok(mut file) = OpenOptions::new().write(true).open(path) {
        let len = file.metadata()?.len();
        let zeros = [0u8; 4096];
        let mut left = len;
        while left > 0 {
            let n = usize::try_from(left.min(zeros.len() as u64)).unwrap_or(zeros.len());
            file.write_all(&zeros[..n])?;
            left -= n as u64;
        }
        file.sync_all()?;
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn read_public_key(inbox: &Path) -> Result<Option<CapturePublicKey>> {
    let bytes = match fs::read(inbox.join(PUBLIC_FILE)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    match bytes.split_first() {
        Some((&FILE_VERSION, key)) => Ok(Some(CapturePublicKey::from_slice(key)?)),
        Some(_) => Err(CoreError::NewerVersion),
        None => Err(CoreError::InvalidInput),
    }
}

/// Whether notes can be written while locked: the journal has been
/// unlocked at least once with this app version, so the inbox has a
/// public key.
#[uniffi::export]
#[must_use]
pub fn capture_ready(inbox_dir: String) -> bool {
    matches!(read_public_key(Path::new(&inbox_dir)), Ok(Some(_)))
}

/// Seals a note into the inbox without unlocking the journal. Nothing in
/// it can be read until the journal is unlocked, when
/// [`Journal::merge_captures`] adds it.
///
/// # Errors
///
/// `NotFound` if the inbox has no public key yet (unlock once first);
/// `InvalidInput` for empty or overlong text; `Storage` if writing fails.
#[uniffi::export]
pub fn seal_capture(inbox_dir: String, text: String, at: LocalTimeData) -> Result<()> {
    crate::init();
    let text = Zeroizing::new(text);
    if text.trim().is_empty() {
        return Err(CoreError::InvalidInput);
    }
    let inbox = PathBuf::from(inbox_dir);
    let public = read_public_key(&inbox)?.ok_or(CoreError::NotFound)?;
    let id = RecordId::random()?;
    let payload = encode_payload(at.into(), &text)?;
    let sealed = public.seal(id.as_bytes(), &payload)?;
    write_atomically(&inbox.join(format!("{}.{EXTENSION}", id.to_hex())), &sealed)
}

/// One stored capture key pair.
pub(crate) struct StoredPair {
    public: CapturePublicKey,
    wrapped: WrappedKey,
}

impl StoredPair {
    fn new(master: &MasterKey, key: &CaptureKey) -> Result<Self> {
        Ok(Self {
            public: key.public_key(),
            wrapped: master.wrap_capture_key(key)?,
        })
    }

    fn load(unlocked: &Unlocked, meta: &str) -> Result<Option<Self>> {
        let Some(bytes) = unlocked.store.meta(meta)? else {
            return Ok(None);
        };
        if bytes.len() < terpoir_crypto::CAPTURE_PUBLIC_KEY_LEN {
            return Err(CoreError::Internal);
        }
        let (public, wrapped) = bytes.split_at(terpoir_crypto::CAPTURE_PUBLIC_KEY_LEN);
        Ok(Some(Self {
            public: CapturePublicKey::from_slice(public)?,
            wrapped: WrappedKey::from_slice(wrapped)?,
        }))
    }

    fn save(&self, unlocked: &mut Unlocked, meta: &str) -> Result<()> {
        let mut bytes = self.public.as_bytes().to_vec();
        bytes.extend_from_slice(self.wrapped.as_bytes());
        unlocked.store.set_meta(meta, &bytes)?;
        Ok(())
    }

    fn unwrap(&self, master: &MasterKey) -> Result<CaptureKey> {
        Ok(master.unwrap_capture_key(&self.wrapped, &self.public)?)
    }
}

/// The current capture key pair, created on first use.
fn current_pair(unlocked: &mut Unlocked) -> Result<StoredPair> {
    if let Some(pair) = StoredPair::load(unlocked, META_CURRENT)? {
        return Ok(pair);
    }
    let pair = StoredPair::new(&unlocked.master, &CaptureKey::generate()?)?;
    pair.save(unlocked, META_CURRENT)?;
    Ok(pair)
}

/// For master-key rotation, after the store is re-wrapped: a new capture
/// key pair under `new_master`; the old one, re-wrapped, becomes the
/// previous pair so notes already sealed to it still open.
pub(crate) fn rotate(unlocked: &mut Unlocked, new_master: &MasterKey) -> Result<()> {
    if let Some(old) = StoredPair::load(unlocked, META_CURRENT)? {
        let key = old.unwrap(&unlocked.master)?;
        StoredPair::new(new_master, &key)?.save(unlocked, META_PREVIOUS)?;
    }
    StoredPair::new(new_master, &CaptureKey::generate()?)?.save(unlocked, META_CURRENT)
}

/// Note files in the inbox, with the record ID from each name. Leftover
/// temporary files from an interrupted write are wiped.
fn pending(inbox: &Path) -> Result<Vec<(RecordId, PathBuf)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(inbox)? {
        let path = entry?.path();
        let extension = path.extension().and_then(|e| e.to_str());
        if extension == Some(TEMP_EXTENSION) {
            wipe_file(&path)?;
            continue;
        }
        if extension != Some(EXTENSION) {
            continue;
        }
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .and_then(|s| RecordId::from_hex(s).ok());
        match id {
            Some(id) => out.push((id, path)),
            None => wipe_file(&path)?,
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

/// What happened to one note file.
enum Outcome {
    Added,
    /// Already in the journal, or discarded (an earlier merge stopped
    /// before deleting the file).
    Present,
    /// From a newer app version: left for after the update.
    Deferred,
    /// Can never be opened (tampered, or sealed to a retired key).
    Unreadable,
}

fn merge_one(
    unlocked: &mut Unlocked,
    keys: &[(CapturePublicKey, CaptureKey)],
    id: &RecordId,
    sealed: &[u8],
    now_ms: u64,
) -> Result<Outcome> {
    if unlocked.store.contains(id)? || unlocked.store.is_deleted(id)? {
        return Ok(Outcome::Present);
    }
    if sealed
        .first()
        .is_some_and(|&version| version > FILE_VERSION)
    {
        return Ok(Outcome::Deferred);
    }
    let Ok(recipient) = CapturePublicKey::of_sealed(sealed) else {
        return Ok(Outcome::Unreadable);
    };
    let Some((_, key)) = keys.iter().find(|(public, _)| *public == recipient) else {
        return Ok(Outcome::Unreadable);
    };
    let Ok(payload) = key.open(id.as_bytes(), sealed) else {
        return Ok(Outcome::Unreadable);
    };
    let (at, text) = match decode_payload(&payload) {
        Ok(note) => note,
        Err(CoreError::NewerVersion) => return Ok(Outcome::Deferred),
        Err(_) => return Ok(Outcome::Unreadable),
    };
    let text = Zeroizing::new(text);
    let snippet = Snippet {
        session: None,
        at,
        text: text.to_string(),
        tags: Default::default(),
        added_while_locked: true,
    };
    let mut doc = Document::new(*id, Snippet::KIND);
    doc.update(&snippet, unlocked.clock.stamp(now_ms))?;
    unlocked.store.put(&unlocked.master, &doc)?;
    unlocked.save_clock()?;
    Ok(Outcome::Added)
}

#[uniffi::export]
impl Journal {
    /// Call on every unlock (and after master-key rotation): creates the
    /// capture key pair if needed, writes its public key to the inbox so
    /// [`seal_capture`] works while locked, and adds every pending note as
    /// a standalone snippet marked "added while locked". Merged note files
    /// are overwritten and deleted. Returns how many notes were added.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn merge_captures(&self, inbox_dir: String, now_ms: u64) -> Result<u32> {
        let inbox = PathBuf::from(inbox_dir);
        fs::create_dir_all(&inbox)?;
        self.with(|u| {
            let current = current_pair(u)?;
            if read_public_key(&inbox).ok().flatten().as_ref() != Some(&current.public) {
                let mut bytes = vec![FILE_VERSION];
                bytes.extend_from_slice(current.public.as_bytes());
                write_atomically(&inbox.join(PUBLIC_FILE), &bytes)?;
            }
            let mut keys = vec![(current.public.clone(), current.unwrap(&u.master)?)];
            if let Some(previous) = StoredPair::load(u, META_PREVIOUS)? {
                keys.push((previous.public.clone(), previous.unwrap(&u.master)?));
            }
            let mut added = 0u32;
            for (id, path) in pending(&inbox)? {
                let sealed = fs::read(&path)?;
                match merge_one(u, &keys, &id, &sealed, now_ms)? {
                    Outcome::Added => {
                        added = added.saturating_add(1);
                        wipe_file(&path)?;
                    }
                    Outcome::Present | Outcome::Unreadable => wipe_file(&path)?,
                    Outcome::Deferred => {}
                }
            }
            Ok(added)
        })
    }

    /// Snippets added while locked that the user has not kept or
    /// discarded yet, oldest first. Keep one by saving it with
    /// `added_while_locked = false`; discard it with `delete_record`.
    ///
    /// # Errors
    ///
    /// `Locked` or `Storage`.
    pub fn locked_captures(&self) -> Result<Vec<String>> {
        self.with(|u| {
            let mut found = Vec::new();
            for id in u.store.ids_of_kind(Snippet::KIND)? {
                let Some(doc) = u.store.get(&u.master, &id)? else {
                    continue;
                };
                let snippet = Snippet::from_document(&migrate(doc)?)?;
                if snippet.added_while_locked {
                    found.push((snippet.at.unix_ms, id));
                }
            }
            found.sort();
            Ok(found.into_iter().map(|(_, id)| id.to_hex()).collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> LocalTime {
        LocalTime {
            unix_ms: 1_760_000_000_000,
            utc_offset_min: -420,
        }
    }

    #[test]
    fn payload_round_trip_and_padding() {
        for text in ["", "piney", &"x".repeat(PAD_TO), "héllo 🌿"] {
            let bytes = encode_payload(at(), text).unwrap();
            assert_eq!(bytes.len() % PAD_TO, 0);
            assert!(bytes.len() >= HEADER_LEN + text.len());
            let (time, decoded) = decode_payload(&bytes).unwrap();
            assert_eq!(time, at());
            assert_eq!(decoded, text);
        }
        assert_eq!(encode_payload(at(), "a").unwrap().len(), PAD_TO);
    }

    #[test]
    fn payload_rejects_malformed() {
        let good = encode_payload(at(), "note").unwrap();
        assert!(decode_payload(&good[..10]).is_err());
        let mut newer = good.clone();
        newer[0] = 2;
        assert!(matches!(
            decode_payload(&newer),
            Err(CoreError::NewerVersion)
        ));
        let mut long = good.clone();
        long[11..15].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(decode_payload(&long).is_err());
        let mut dirty = good.clone();
        *dirty.last_mut().unwrap() = 1;
        assert!(decode_payload(&dirty).is_err());
        assert!(encode_payload(at(), &"x".repeat(MAX_TEXT_LEN + 1)).is_err());
    }

    #[test]
    fn wipe_removes_the_file() {
        let dir = std::env::temp_dir().join(format!(
            "terpoir-wipe-{}",
            RecordId::random().unwrap().to_hex()
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.cap");
        fs::write(&path, b"secret").unwrap();
        wipe_file(&path).unwrap();
        assert!(!path.exists());
        // Already gone: not an error.
        wipe_file(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
