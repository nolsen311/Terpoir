// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! XChaCha20-Poly1305 sealing and key wrapping.
//!
//! Sealed format: `version (1) || nonce (24) || ciphertext || tag (16)`.
//!
//! Every operation is domain-separated: the associated data is
//! `"terpoir" || 0x00 || version || domain || 0x00 || caller AAD`, so a
//! ciphertext made for one purpose (say, a wrapped record key) never opens
//! in another (say, a wrapped master key), even under the same key.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use core::fmt;
use zeroize::Zeroizing;

use crate::keys::KEY_LEN;
use crate::{Error, Result, fill_random};

pub(crate) const VERSION: u8 = 1;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;

/// Bytes [`seal`]ed output adds to the plaintext: version, nonce and tag.
pub const SEAL_OVERHEAD: usize = 1 + NONCE_LEN + TAG_LEN;

/// Length of a [`WrappedKey`] in bytes.
pub const WRAPPED_KEY_LEN: usize = SEAL_OVERHEAD + KEY_LEN;

/// Purpose labels. Each is used for exactly one kind of ciphertext.
#[derive(Clone, Copy)]
pub(crate) enum Domain {
    Record,
    WrapMaster,
    WrapRecord,
    WrapDatabase,
    FileStream,
    SubKeySeal,
}

impl Domain {
    const fn label(self) -> &'static [u8] {
        match self {
            Self::Record => b"record",
            Self::WrapMaster => b"wrap/master",
            Self::WrapRecord => b"wrap/record",
            Self::WrapDatabase => b"wrap/database",
            Self::FileStream => b"file-stream",
            Self::SubKeySeal => b"subkey-seal",
        }
    }
}

pub(crate) fn associated_data(domain: Domain, aad: &[u8]) -> Vec<u8> {
    let label = domain.label();
    let mut out = Vec::with_capacity(9 + label.len() + 1 + aad.len());
    out.extend_from_slice(b"terpoir\0");
    out.push(VERSION);
    out.extend_from_slice(label);
    out.push(0);
    out.extend_from_slice(aad);
    out
}

/// Encrypts `plaintext` under `key` with a fresh random nonce.
pub(crate) fn seal(
    key: &[u8; KEY_LEN],
    domain: Domain,
    aad: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(<&Key>::from(key));
    let mut nonce = [0u8; NONCE_LEN];
    fill_random(&mut nonce)?;
    let full_aad = associated_data(domain, aad);
    let ciphertext = cipher
        .encrypt(
            <&XNonce>::from(&nonce),
            Payload {
                msg: plaintext,
                aad: &full_aad,
            },
        )
        .map_err(|_| Error::Encrypt)?;
    let mut out = Vec::with_capacity(1 + NONCE_LEN + ciphertext.len());
    out.push(VERSION);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts and authenticates output of [`seal`].
pub(crate) fn open(
    key: &[u8; KEY_LEN],
    domain: Domain,
    aad: &[u8],
    sealed: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    if sealed.len() < SEAL_OVERHEAD {
        return Err(Error::Format);
    }
    let (version, rest) = sealed.split_at(1);
    if version[0] != VERSION {
        return Err(Error::Format);
    }
    let (nonce, ciphertext) = rest.split_at(NONCE_LEN);
    let nonce: &[u8; NONCE_LEN] = nonce.try_into().map_err(|_| Error::Format)?;
    let cipher = XChaCha20Poly1305::new(<&Key>::from(key));
    let full_aad = associated_data(domain, aad);
    cipher
        .decrypt(
            <&XNonce>::from(nonce),
            Payload {
                msg: ciphertext,
                aad: &full_aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Decrypt)
}

/// A key encrypted under another key (key wrapping).
///
/// Wrapped keys are safe to store and sync. Destroying every copy of a
/// record's wrapped key crypto-erases the record.
#[derive(Clone, PartialEq, Eq)]
pub struct WrappedKey([u8; WRAPPED_KEY_LEN]);

impl WrappedKey {
    /// The serialized form, [`WRAPPED_KEY_LEN`] bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; WRAPPED_KEY_LEN] {
        &self.0
    }

    /// Parses the serialized form.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] on wrong length or unknown version.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; WRAPPED_KEY_LEN] = bytes.try_into().map_err(|_| Error::Format)?;
        if bytes[0] != VERSION {
            return Err(Error::Format);
        }
        Ok(Self(bytes))
    }

    pub(crate) fn wrap(
        wrapping_key: &[u8; KEY_LEN],
        domain: Domain,
        context: &[u8],
        key: &[u8; KEY_LEN],
    ) -> Result<Self> {
        let sealed = seal(wrapping_key, domain, context, key)?;
        Self::from_slice(&sealed)
    }

    pub(crate) fn unwrap(
        &self,
        wrapping_key: &[u8; KEY_LEN],
        domain: Domain,
        context: &[u8],
    ) -> Result<Zeroizing<[u8; KEY_LEN]>> {
        let plain = open(wrapping_key, domain, context, &self.0)?;
        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        if plain.len() != KEY_LEN {
            return Err(Error::Format);
        }
        key.copy_from_slice(&plain);
        Ok(key)
    }
}

impl fmt::Debug for WrappedKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WrappedKey(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; KEY_LEN] = [7; KEY_LEN];
    const OTHER: [u8; KEY_LEN] = [8; KEY_LEN];

    #[test]
    fn seal_open_round_trip() {
        let sealed = seal(&KEY, Domain::Record, b"aad", b"hello").unwrap();
        assert_eq!(sealed.len(), 5 + SEAL_OVERHEAD);
        let opened = open(&KEY, Domain::Record, b"aad", &sealed).unwrap();
        assert_eq!(opened.as_slice(), b"hello");
    }

    #[test]
    fn nonces_are_fresh() {
        let a = seal(&KEY, Domain::Record, b"", b"same").unwrap();
        let b = seal(&KEY, Domain::Record, b"", b"same").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn open_rejects_wrong_key_aad_domain_and_tampering() {
        let sealed = seal(&KEY, Domain::Record, b"aad", b"hello").unwrap();
        assert_eq!(
            open(&OTHER, Domain::Record, b"aad", &sealed),
            Err(Error::Decrypt)
        );
        assert_eq!(
            open(&KEY, Domain::Record, b"other", &sealed),
            Err(Error::Decrypt)
        );
        assert_eq!(
            open(&KEY, Domain::WrapRecord, b"aad", &sealed),
            Err(Error::Decrypt)
        );
        for i in 1..sealed.len() {
            let mut tampered = sealed.clone();
            tampered[i] ^= 1;
            assert_eq!(
                open(&KEY, Domain::Record, b"aad", &tampered),
                Err(Error::Decrypt)
            );
        }
    }

    #[test]
    fn open_rejects_malformed() {
        assert_eq!(
            open(&KEY, Domain::Record, b"", &[1; 10]),
            Err(Error::Format)
        );
        let mut sealed = seal(&KEY, Domain::Record, b"", b"x").unwrap();
        sealed[0] = 2;
        assert_eq!(open(&KEY, Domain::Record, b"", &sealed), Err(Error::Format));
    }

    #[test]
    fn domain_label_and_aad_cannot_collide() {
        // "wrap/record" + aad "" must differ from "wrap" + aad "/record".
        assert_ne!(
            associated_data(Domain::WrapRecord, b""),
            associated_data(Domain::Record, b"\0")
        );
    }

    #[test]
    fn wrapped_key_round_trip() {
        let wrapped = WrappedKey::wrap(&KEY, Domain::WrapRecord, b"ctx", &OTHER).unwrap();
        let parsed = WrappedKey::from_slice(wrapped.as_bytes()).unwrap();
        let key = parsed.unwrap(&KEY, Domain::WrapRecord, b"ctx").unwrap();
        assert_eq!(*key, OTHER);
        assert_eq!(
            parsed.unwrap(&KEY, Domain::WrapRecord, b"other").err(),
            Some(Error::Decrypt)
        );
        assert_eq!(WrappedKey::from_slice(&[1; 10]).err(), Some(Error::Format));
    }
}
