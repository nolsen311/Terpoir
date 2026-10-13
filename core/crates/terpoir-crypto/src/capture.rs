// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Sealing notes to the capture key pair (write-only capture).
//!
//! A note written while the journal is locked is sealed with HPKE
//! (RFC 9180) in base mode, suite DHKEM(X25519, HKDF-SHA-256),
//! HKDF-SHA-256, ChaCha20-Poly1305, to the public key the app keeps in
//! the clear. Sealing needs no secret, so it works while the journal is
//! locked; opening needs the [`CaptureKey`], which the master key wraps.
//!
//! Sealed format: `version (1) || recipient public key (32) ||
//! encapsulated key (32) || ciphertext || tag (16)`. The recipient's public
//! key says which capture key opens it (the current one, or the one before
//! the last master-key rotation). The associated data is domain-separated
//! as in [`crate::aead`].

use core::fmt;
use hpke::aead::ChaCha20Poly1305;
use hpke::kdf::HkdfSha256;
use hpke::kem::X25519HkdfSha256;
use hpke::{Deserializable, Kem, OpModeR, OpModeS, Serializable};
use zeroize::Zeroizing;

use crate::aead::{Domain, VERSION, associated_data};
use crate::{CaptureKey, Error, Result};

/// Length of a [`CapturePublicKey`] in bytes.
pub const CAPTURE_PUBLIC_KEY_LEN: usize = 32;

const ENCAPPED_LEN: usize = 32;
const TAG_LEN: usize = 16;

/// Bytes [`CapturePublicKey::seal`] adds to the plaintext.
pub const CAPTURE_SEAL_OVERHEAD: usize = 1 + CAPTURE_PUBLIC_KEY_LEN + ENCAPPED_LEN + TAG_LEN;

/// HPKE `info`: binds every capture to this purpose and format version.
const INFO: &[u8] = b"terpoir/capture/v1";

type Suite = X25519HkdfSha256;

/// The public half of the capture key pair. Not secret: the app stores it
/// unencrypted so it can seal notes while the journal is locked.
#[derive(Clone, PartialEq, Eq)]
pub struct CapturePublicKey([u8; CAPTURE_PUBLIC_KEY_LEN]);

impl CapturePublicKey {
    /// The serialized form, [`CAPTURE_PUBLIC_KEY_LEN`] bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; CAPTURE_PUBLIC_KEY_LEN] {
        &self.0
    }

    /// Parses the serialized form.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] on wrong length or an invalid point.
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; CAPTURE_PUBLIC_KEY_LEN] = bytes.try_into().map_err(|_| Error::Format)?;
        <Suite as Kem>::PublicKey::from_bytes(&bytes).map_err(|_| Error::Format)?;
        Ok(Self(bytes))
    }

    /// The recipient of a sealed capture: which public key it was sealed
    /// to.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] for input too short or of an unknown version.
    pub fn of_sealed(sealed: &[u8]) -> Result<Self> {
        if sealed.len() < CAPTURE_SEAL_OVERHEAD || sealed[0] != VERSION {
            return Err(Error::Format);
        }
        Self::from_slice(&sealed[1..=CAPTURE_PUBLIC_KEY_LEN])
    }

    /// Seals `plaintext` to this public key. `aad` binds the result to its
    /// context; [`CaptureKey::open`] must pass the same bytes.
    ///
    /// # Errors
    ///
    /// [`Error::Encrypt`] if sealing fails.
    pub fn seal(&self, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
        let public = <Suite as Kem>::PublicKey::from_bytes(&self.0).map_err(|_| Error::Encrypt)?;
        let (encapped, ciphertext) = hpke::single_shot_seal::<ChaCha20Poly1305, HkdfSha256, Suite>(
            &OpModeS::Base,
            &public,
            INFO,
            plaintext,
            &associated_data(Domain::Capture, aad),
        )
        .map_err(|_| Error::Encrypt)?;
        let mut out = Vec::with_capacity(CAPTURE_SEAL_OVERHEAD + plaintext.len());
        out.push(VERSION);
        out.extend_from_slice(&self.0);
        out.extend_from_slice(&encapped.to_bytes());
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }
}

impl fmt::Debug for CapturePublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CapturePublicKey(..)")
    }
}

impl CaptureKey {
    fn keypair(&self) -> (<Suite as Kem>::PrivateKey, <Suite as Kem>::PublicKey) {
        Suite::derive_keypair(self.secret())
    }

    /// The public half of this key pair.
    #[must_use]
    pub fn public_key(&self) -> CapturePublicKey {
        let mut bytes = [0u8; CAPTURE_PUBLIC_KEY_LEN];
        bytes.copy_from_slice(&self.keypair().1.to_bytes());
        CapturePublicKey(bytes)
    }

    /// Opens output of [`CapturePublicKey::seal`] sealed to this key's
    /// public half. The plaintext is wiped when dropped.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] for malformed input or another recipient;
    /// [`Error::Decrypt`] for the wrong `aad` or tampered input.
    pub fn open(&self, aad: &[u8], sealed: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        let (private, public) = self.keypair();
        if CapturePublicKey::of_sealed(sealed)?.0[..] != public.to_bytes()[..] {
            return Err(Error::Format);
        }
        let rest = &sealed[1 + CAPTURE_PUBLIC_KEY_LEN..];
        let (encapped, ciphertext) = rest.split_at(ENCAPPED_LEN);
        let encapped =
            <Suite as Kem>::EncappedKey::from_bytes(encapped).map_err(|_| Error::Decrypt)?;
        hpke::single_shot_open::<ChaCha20Poly1305, HkdfSha256, Suite>(
            &OpModeR::Base,
            &private,
            &encapped,
            INFO,
            ciphertext,
            &associated_data(Domain::Capture, aad),
        )
        .map(Zeroizing::new)
        .map_err(|_| Error::Decrypt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MasterKey;

    #[test]
    fn seal_open_round_trip() {
        let key = CaptureKey::generate().unwrap();
        let public = key.public_key();
        let sealed = public.seal(b"id", b"piney, very chill").unwrap();
        assert_eq!(sealed.len(), 17 + CAPTURE_SEAL_OVERHEAD);
        assert_eq!(CapturePublicKey::of_sealed(&sealed).unwrap(), public);
        assert_eq!(
            key.open(b"id", &sealed).unwrap().as_slice(),
            b"piney, very chill"
        );
    }

    #[test]
    fn public_key_is_deterministic_and_parses() {
        let key = CaptureKey::generate().unwrap();
        assert_eq!(key.public_key(), key.public_key());
        let parsed = CapturePublicKey::from_slice(key.public_key().as_bytes()).unwrap();
        assert_eq!(parsed, key.public_key());
        assert_ne!(
            CaptureKey::generate().unwrap().public_key(),
            key.public_key()
        );
        assert_eq!(
            CapturePublicKey::from_slice(&[1; 31]).err(),
            Some(Error::Format)
        );
    }

    #[test]
    fn sealing_is_randomized() {
        let public = CaptureKey::generate().unwrap().public_key();
        assert_ne!(
            public.seal(b"", b"same").unwrap(),
            public.seal(b"", b"same").unwrap()
        );
    }

    #[test]
    fn open_rejects_other_key_aad_and_tampering() {
        let key = CaptureKey::generate().unwrap();
        let other = CaptureKey::generate().unwrap();
        let sealed = key.public_key().seal(b"aad", b"hello").unwrap();
        // Another recipient: refused before any decryption.
        assert_eq!(other.open(b"aad", &sealed).err(), Some(Error::Format));
        assert_eq!(key.open(b"other", &sealed).err(), Some(Error::Decrypt));
        // Claiming another recipient in the header does not help.
        let mut relabelled = sealed.clone();
        relabelled[1..=32].copy_from_slice(other.public_key().as_bytes());
        assert_eq!(other.open(b"aad", &relabelled).err(), Some(Error::Decrypt));
        for i in 1 + CAPTURE_PUBLIC_KEY_LEN..sealed.len() {
            let mut tampered = sealed.clone();
            tampered[i] ^= 1;
            assert!(key.open(b"aad", &tampered).is_err(), "byte {i}");
        }
    }

    #[test]
    fn open_rejects_malformed() {
        let key = CaptureKey::generate().unwrap();
        assert_eq!(key.open(b"", &[1; 10]).err(), Some(Error::Format));
        let mut sealed = key.public_key().seal(b"", b"x").unwrap();
        sealed[0] = 2;
        assert_eq!(key.open(b"", &sealed).err(), Some(Error::Format));
    }

    #[test]
    fn capture_seal_is_not_a_record_seal() {
        // Same plaintext and AAD under the record domain must not open
        // here, and vice versa: the domain label differs.
        assert_ne!(
            associated_data(Domain::Capture, b""),
            associated_data(Domain::Record, b"")
        );
    }

    #[test]
    fn wrap_binds_to_master_and_public_key() {
        let master = MasterKey::generate().unwrap();
        let key = CaptureKey::generate().unwrap();
        let public = key.public_key();
        let wrapped = master.wrap_capture_key(&key).unwrap();
        let unwrapped = master.unwrap_capture_key(&wrapped, &public).unwrap();
        assert_eq!(unwrapped.public_key(), public);

        let other_master = MasterKey::generate().unwrap();
        assert_eq!(
            other_master.unwrap_capture_key(&wrapped, &public).err(),
            Some(Error::Decrypt)
        );
        let other_public = CaptureKey::generate().unwrap().public_key();
        assert_eq!(
            master.unwrap_capture_key(&wrapped, &other_public).err(),
            Some(Error::Decrypt)
        );
        // A wrapped capture key is not a wrapped record key.
        assert!(
            master
                .unwrap_record_key(&wrapped, &crate::RecordId::from_bytes([0; 16]))
                .is_err()
        );
    }

    #[test]
    fn debug_is_redacted() {
        let key = CaptureKey::generate().unwrap();
        assert_eq!(format!("{key:?}"), "CaptureKey(<redacted>)");
        assert_eq!(format!("{:?}", key.public_key()), "CapturePublicKey(..)");
    }
}
