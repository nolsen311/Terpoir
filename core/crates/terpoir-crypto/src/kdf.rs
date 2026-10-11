// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.

//! Key derivation: HKDF-SHA-256 for subkeys, Argon2id for passphrases.

use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use secrecy::SecretBox;
use sha2::Sha256;

use crate::keys::{KEY_LEN, KeyEncryptionKey};
use crate::{Error, Result, fill_random};

/// Prefix of every HKDF `info` string, for domain separation.
const HKDF_INFO_PREFIX: &[u8] = b"terpoir/v1/";

/// HKDF-SHA-256 with no salt (the input is already a uniformly random key)
/// and `info = "terpoir/v1/" || parts...`.
pub(crate) fn hkdf_sha256(
    ikm: &[u8; KEY_LEN],
    info_parts: &[&[u8]],
) -> Result<SecretBox<[u8; KEY_LEN]>> {
    let hkdf = Hkdf::<Sha256>::new(None, ikm);
    let mut info = Vec::with_capacity(64);
    info.extend_from_slice(HKDF_INFO_PREFIX);
    for part in info_parts {
        info.extend_from_slice(part);
    }
    let mut result = Ok(());
    let okm = SecretBox::<[u8; KEY_LEN]>::init_with_mut(|okm| {
        result = hkdf.expand(&info, okm).map_err(|_| Error::Kdf);
    });
    result.map(|()| okm)
}

/// Salt length for passphrase derivation.
pub const SALT_LEN: usize = 16;

const PARAMS_VERSION: u8 = 1;
const PARAMS_LEN: usize = 1 + 4 + 4 + 4 + SALT_LEN;

/// Upper bounds, so tampered parameters cannot demand unbounded memory or
/// time when the app tries to unlock.
const MAX_MEMORY_KIB: u32 = 1024 * 1024;
const MAX_ITERATIONS: u32 = 16;
const MAX_PARALLELISM: u32 = 8;

/// Argon2id parameters and salt for deriving a [`KeyEncryptionKey`] from
/// the app passphrase. Stored next to the wrapped master key; not secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassphraseParams {
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    salt: [u8; SALT_LEN],
}

impl PassphraseParams {
    /// Production defaults with a fresh random salt: 64 MiB of memory,
    /// 3 iterations, 1 lane. Stronger than the OWASP minimum
    /// (19 MiB, 2 iterations) while staying around a second on a phone.
    ///
    /// # Errors
    ///
    /// [`Error::Random`] if the operating system's generator fails.
    pub fn recommended() -> Result<Self> {
        Self::new(64 * 1024, 3, 1)
    }

    /// Custom costs with a fresh random salt. Use
    /// [`PassphraseParams::recommended`] in production; lower costs are for
    /// tests.
    ///
    /// # Errors
    ///
    /// [`Error::Kdf`] if the costs are outside what Argon2 accepts or above
    /// this crate's limits; [`Error::Random`] if salt generation fails.
    pub fn new(memory_kib: u32, iterations: u32, parallelism: u32) -> Result<Self> {
        let mut salt = [0u8; SALT_LEN];
        fill_random(&mut salt)?;
        let params = Self {
            memory_kib,
            iterations,
            parallelism,
            salt,
        };
        params.argon2_params()?;
        Ok(params)
    }

    fn argon2_params(&self) -> Result<Params> {
        if self.memory_kib > MAX_MEMORY_KIB
            || self.iterations > MAX_ITERATIONS
            || self.parallelism > MAX_PARALLELISM
        {
            return Err(Error::Kdf);
        }
        Params::new(
            self.memory_kib,
            self.iterations,
            self.parallelism,
            Some(KEY_LEN),
        )
        .map_err(|_| Error::Kdf)
    }

    /// Derives the key-encryption key from a passphrase.
    ///
    /// # Errors
    ///
    /// [`Error::Kdf`] for invalid parameters or if Argon2 fails.
    pub fn derive(&self, passphrase: &[u8]) -> Result<KeyEncryptionKey> {
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, self.argon2_params()?);
        let mut result = Ok(());
        let key = SecretBox::<[u8; KEY_LEN]>::init_with_mut(|out| {
            result = argon2
                .hash_password_into(passphrase, &self.salt, out)
                .map_err(|_| Error::Kdf);
        });
        result.map(|()| KeyEncryptionKey::from_secret(key))
    }

    /// Serialized form: version, memory, iterations, parallelism (each
    /// big-endian `u32`), then the salt.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(PARAMS_LEN);
        out.push(PARAMS_VERSION);
        out.extend_from_slice(&self.memory_kib.to_be_bytes());
        out.extend_from_slice(&self.iterations.to_be_bytes());
        out.extend_from_slice(&self.parallelism.to_be_bytes());
        out.extend_from_slice(&self.salt);
        out
    }

    /// Parses [`PassphraseParams::to_bytes`] output and validates it.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] for malformed input; [`Error::Kdf`] for costs out
    /// of range.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != PARAMS_LEN || bytes[0] != PARAMS_VERSION {
            return Err(Error::Format);
        }
        let word = |at: usize| -> Result<u32> {
            bytes
                .get(at..at + 4)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_be_bytes)
                .ok_or(Error::Format)
        };
        let params = Self {
            memory_kib: word(1)?,
            iterations: word(5)?,
            parallelism: word(9)?,
            salt: bytes[13..].try_into().map_err(|_| Error::Format)?,
        };
        params.argon2_params()?;
        Ok(params)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::MasterKey;

    fn cheap() -> PassphraseParams {
        PassphraseParams::new(64, 1, 1).unwrap()
    }

    #[test]
    fn passphrase_unlocks_master_key() {
        let params = cheap();
        let master = MasterKey::generate().unwrap();
        let wrapped = params
            .derive(b"correct horse")
            .unwrap()
            .wrap_master_key(&master)
            .unwrap();

        let stored = PassphraseParams::from_bytes(&params.to_bytes()).unwrap();
        let unwrapped = stored
            .derive(b"correct horse")
            .unwrap()
            .unwrap_master_key(&wrapped)
            .unwrap();
        assert_eq!(unwrapped.expose_secret(), master.expose_secret());

        let wrong = stored.derive(b"wrong horse").unwrap();
        assert_eq!(
            wrong.unwrap_master_key(&wrapped).err(),
            Some(Error::Decrypt)
        );
    }

    #[test]
    fn salt_is_random() {
        assert_ne!(cheap().salt, cheap().salt);
    }

    #[test]
    fn recommended_params_are_valid() {
        let params = PassphraseParams::recommended().unwrap();
        assert_eq!(params.memory_kib, 65536);
        assert!(params.argon2_params().is_ok());
    }

    #[test]
    fn rejects_out_of_range_costs() {
        assert_eq!(PassphraseParams::new(64, 0, 1).err(), Some(Error::Kdf));
        assert_eq!(
            PassphraseParams::new(MAX_MEMORY_KIB + 1, 1, 1).err(),
            Some(Error::Kdf)
        );
        let mut bytes = cheap().to_bytes();
        bytes[5..9].copy_from_slice(&1000u32.to_be_bytes());
        assert_eq!(PassphraseParams::from_bytes(&bytes).err(), Some(Error::Kdf));
    }

    #[test]
    fn rejects_malformed_bytes() {
        assert_eq!(
            PassphraseParams::from_bytes(&[1; 5]).err(),
            Some(Error::Format)
        );
        let mut bytes = cheap().to_bytes();
        bytes[0] = 9;
        assert_eq!(
            PassphraseParams::from_bytes(&bytes).err(),
            Some(Error::Format)
        );
    }

    #[test]
    fn hkdf_known_answer() {
        // Independently computed with Python's hmac/hashlib (RFC 5869,
        // empty salt): HKDF-SHA-256(ikm = 32 x 0x03, info = "terpoir/v1/x").
        use secrecy::ExposeSecret;
        let okm = hkdf_sha256(&[3u8; KEY_LEN], &[b"x"]).unwrap();
        assert_eq!(
            okm.expose_secret(),
            &hex32("d861b9cb4f7102a10efdccd1df7c5e2ddd36ca6e761b51c181f76a9a1ce3674f")
        );
        let other = hkdf_sha256(&[3u8; KEY_LEN], &[b"y"]).unwrap();
        assert_ne!(other.expose_secret(), okm.expose_secret());
    }

    pub(crate) fn hex32(hex: &str) -> [u8; KEY_LEN] {
        let mut out = [0u8; KEY_LEN];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }
}
