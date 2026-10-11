// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! The recovery key and its human-readable encoding.
//!
//! Encoding: `version (1 byte) || key (32 bytes) || checksum (2 bytes)`,
//! 35 bytes = 280 bits = exactly 56 characters of Crockford Base32, shown
//! as 14 groups of 4 separated by hyphens:
//!
//! ```text
//! 1ABC-DEFG-HJKM-NPQR-STVW-XYZ0-1234-5678-9ABC-DEFG-HJKM-NPQR-STVW-XYZ0
//! ```
//!
//! Crockford Base32 avoids the letters I, L, O and U; on input, `I` and `L`
//! read as `1`, `O` reads as `0`, case is ignored, and spaces and hyphens
//! are skipped. The checksum is the first two bytes of
//! `SHA-256("terpoir/v1/recovery-checksum" || version || key)`, catching
//! typos before the key is used.

use core::fmt;
use secrecy::{ExposeSecret, SecretBox};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::kdf::hkdf_sha256;
use crate::keys::{KEY_LEN, KeyEncryptionKey};
use crate::{Error, RecoveryKeyError, Result, fill_random};

const VERSION: u8 = 1;
const CHECKSUM_LEN: usize = 2;
const PAYLOAD_LEN: usize = 1 + KEY_LEN + CHECKSUM_LEN;
const CHARS: usize = PAYLOAD_LEN * 8 / 5;
const GROUP: usize = 4;
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The user-held root of the key hierarchy.
///
/// Shown once during onboarding for the user to back up; afterwards it is
/// only needed to restore the journal on a new device.
pub struct RecoveryKey(SecretBox<[u8; KEY_LEN]>);

impl RecoveryKey {
    /// Generates a new random recovery key.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn generate() -> Result<Self> {
        let mut result = Ok(());
        let key = SecretBox::init_with_mut(|bytes: &mut [u8; KEY_LEN]| {
            result = fill_random(bytes);
        });
        result.map(|()| Self(key))
    }

    /// The key-encryption key that wraps the master key.
    ///
    /// # Errors
    ///
    /// [`Error::Kdf`] (not expected in practice).
    pub fn key_encryption_key(&self) -> Result<KeyEncryptionKey> {
        hkdf_sha256(self.0.expose_secret(), &[b"kek/recovery"]).map(KeyEncryptionKey::from_secret)
    }

    /// The human-readable form, 14 hyphen-separated groups of 4. The string
    /// is wiped when dropped; avoid copying it.
    #[must_use]
    pub fn to_display_string(&self) -> Zeroizing<String> {
        let mut payload = Zeroizing::new([0u8; PAYLOAD_LEN]);
        payload[0] = VERSION;
        payload[1..=KEY_LEN].copy_from_slice(self.0.expose_secret());
        let sum = checksum(VERSION, self.0.expose_secret());
        payload[1 + KEY_LEN..].copy_from_slice(&sum);

        let mut out = Zeroizing::new(String::with_capacity(CHARS + CHARS / GROUP));
        let mut acc: u16 = 0;
        let mut bits = 0u32;
        let mut written = 0usize;
        for &byte in payload.iter() {
            acc = (acc << 8) | u16::from(byte);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                let index = usize::from((acc >> bits) & 0x1f);
                if written > 0 && written.is_multiple_of(GROUP) {
                    out.push('-');
                }
                out.push(char::from(ALPHABET[index]));
                written += 1;
            }
            acc &= (1 << bits) - 1;
        }
        out
    }

    /// Parses the human-readable form.
    ///
    /// # Errors
    ///
    /// [`Error::RecoveryKey`] saying whether the length, a character, the
    /// checksum or the version is wrong.
    pub fn parse(text: &str) -> Result<Self> {
        let mut payload = Zeroizing::new([0u8; PAYLOAD_LEN]);
        let mut acc: u16 = 0;
        let mut bits = 0u32;
        let mut count = 0usize;
        for c in text.chars() {
            if c == '-' || c.is_whitespace() {
                continue;
            }
            let value = decode_char(c).ok_or(RecoveryKeyError::Character)?;
            count += 1;
            if count > CHARS {
                return Err(RecoveryKeyError::Length.into());
            }
            acc = (acc << 5) | u16::from(value);
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                let byte_index = (count * 5) / 8 - 1;
                payload[byte_index] =
                    u8::try_from((acc >> bits) & 0xff).map_err(|_| Error::Format)?;
                acc &= (1 << bits) - 1;
            }
        }
        if count != CHARS {
            return Err(RecoveryKeyError::Length.into());
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&payload[1..=KEY_LEN]);
        let key = Zeroizing::new(key);
        if checksum(payload[0], &key) != payload[1 + KEY_LEN..] {
            return Err(RecoveryKeyError::Checksum.into());
        }
        if payload[0] != VERSION {
            return Err(RecoveryKeyError::Version.into());
        }
        Ok(Self(SecretBox::init_with_mut(|k: &mut [u8; KEY_LEN]| {
            k.copy_from_slice(&*key);
        })))
    }
}

fn checksum(version: u8, key: &[u8; KEY_LEN]) -> [u8; CHECKSUM_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(b"terpoir/v1/recovery-checksum");
    hasher.update([version]);
    hasher.update(key);
    let digest = hasher.finalize();
    [digest[0], digest[1]]
}

fn decode_char(c: char) -> Option<u8> {
    let c = match c.to_ascii_uppercase() {
        'O' => '0',
        'I' | 'L' => '1',
        other => other,
    };
    let byte = u8::try_from(c).ok()?;
    ALPHABET
        .iter()
        .position(|&a| a == byte)
        .and_then(|p| u8::try_from(p).ok())
}

impl fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryKey(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MasterKey;

    fn secret(key: &RecoveryKey) -> [u8; KEY_LEN] {
        *key.0.expose_secret()
    }

    #[test]
    fn display_format() {
        let key = RecoveryKey::generate().unwrap();
        let text = key.to_display_string();
        let groups: Vec<&str> = text.split('-').collect();
        assert_eq!(groups.len(), 14);
        assert!(groups.iter().all(|g| g.len() == 4));
        assert!(
            text.chars()
                .all(|c| c == '-' || ALPHABET.contains(&(c as u8)))
        );
    }

    #[test]
    fn round_trip() {
        for _ in 0..64 {
            let key = RecoveryKey::generate().unwrap();
            let parsed = RecoveryKey::parse(&key.to_display_string()).unwrap();
            assert_eq!(secret(&parsed), secret(&key));
        }
    }

    #[test]
    fn known_encoding() {
        let key = RecoveryKey(SecretBox::init_with_mut(|k: &mut [u8; KEY_LEN]| {
            *k = [0; KEY_LEN];
        }));
        let text = key.to_display_string();
        // Version byte 0x01 then zeros: first group encodes 00000001 000...
        assert!(text.starts_with("0400-0000-"));
        assert_eq!(secret(&RecoveryKey::parse(&text).unwrap()), [0; KEY_LEN]);
    }

    #[test]
    fn known_answer() {
        // Independently computed with Python: key bytes 0x00..=0x1f.
        let key = RecoveryKey(SecretBox::init_with_mut(|k: &mut [u8; KEY_LEN]| {
            for (i, b) in k.iter_mut().enumerate() {
                *b = i as u8;
            }
        }));
        assert_eq!(
            key.to_display_string().as_str(),
            "0400-20G3-0G2G-C1R8-1450-P30D-1R7H-048J-2CA1-A5GQ-30CH-M6RW-3MF1-Y1YN"
        );
        let kek = RecoveryKey(SecretBox::init_with_mut(|k: &mut [u8; KEY_LEN]| {
            *k = [0; KEY_LEN];
        }))
        .key_encryption_key()
        .unwrap();
        assert_eq!(
            kek.secret(),
            &crate::kdf::tests::hex32(
                "fe5910912bee6a3bfa40fce0c00d58b5393fae1541571f2aa6534a4ab4dc3607"
            )
        );
    }

    #[test]
    fn lenient_input() {
        let key = RecoveryKey::generate().unwrap();
        let text = key.to_display_string();
        let messy = text
            .to_lowercase()
            .replace('-', " ")
            .replace('0', "o")
            .replace('1', "l");
        assert_eq!(secret(&RecoveryKey::parse(&messy).unwrap()), secret(&key));
    }

    #[test]
    fn detects_typos() {
        let key = RecoveryKey::generate().unwrap();
        let text = key.to_display_string().replace('-', "");
        let mut rejected = 0;
        for i in 0..text.len() {
            let mut chars: Vec<char> = text.chars().collect();
            chars[i] = if chars[i] == '2' { '3' } else { '2' };
            let typo: String = chars.into_iter().collect();
            if RecoveryKey::parse(&typo).is_err() {
                rejected += 1;
            }
        }
        // A 16-bit checksum misses about 1 in 65536 random errors; allow one
        // miss so the test is not flaky.
        assert!(rejected >= text.len() - 1);
    }

    #[test]
    fn rejects_bad_input() {
        let text = RecoveryKey::generate().unwrap().to_display_string();
        assert_eq!(
            RecoveryKey::parse(&text[..text.len() - 1]).err(),
            Some(Error::RecoveryKey(RecoveryKeyError::Length))
        );
        assert_eq!(
            RecoveryKey::parse(&format!("{}A", *text)).err(),
            Some(Error::RecoveryKey(RecoveryKeyError::Length))
        );
        assert_eq!(
            RecoveryKey::parse(&text.replacen(|c: char| c != '-', "U", 1)).err(),
            Some(Error::RecoveryKey(RecoveryKeyError::Character))
        );
        assert_eq!(
            RecoveryKey::parse("").err(),
            Some(Error::RecoveryKey(RecoveryKeyError::Length))
        );
    }

    #[test]
    fn rejects_unknown_version_with_valid_checksum() {
        // Hand-build a payload with version 2 and a matching checksum.
        let key = [9u8; KEY_LEN];
        let sum = checksum(2, &key);
        let mut payload = vec![2u8];
        payload.extend_from_slice(&key);
        payload.extend_from_slice(&sum);
        let mut text = String::new();
        let (mut acc, mut bits) = (0u16, 0u32);
        for byte in payload {
            acc = (acc << 8) | u16::from(byte);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                text.push(char::from(ALPHABET[usize::from((acc >> bits) & 0x1f)]));
            }
            acc &= (1 << bits) - 1;
        }
        assert_eq!(
            RecoveryKey::parse(&text).err(),
            Some(Error::RecoveryKey(RecoveryKeyError::Version))
        );
    }

    #[test]
    fn recovery_key_unwraps_master_key() {
        let recovery = RecoveryKey::generate().unwrap();
        let master = MasterKey::generate().unwrap();
        let wrapped = recovery
            .key_encryption_key()
            .unwrap()
            .wrap_master_key(&master)
            .unwrap();
        let restored = RecoveryKey::parse(&recovery.to_display_string()).unwrap();
        let unwrapped = restored
            .key_encryption_key()
            .unwrap()
            .unwrap_master_key(&wrapped)
            .unwrap();
        assert_eq!(unwrapped.expose_secret(), master.expose_secret());
    }

    #[test]
    fn debug_is_redacted() {
        let key = RecoveryKey::generate().unwrap();
        assert_eq!(format!("{key:?}"), "RecoveryKey(<redacted>)");
    }
}
